//! `nubless`: control an LG 27GM950B from the command line.

mod probe;

use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::mpsc;
use std::time::Duration;

use clap::{Parser, Subcommand};
use nubless_core::controller::Snapshot;
use nubless_core::journal::Journal;
use nubless_core::mock::MockMonitor;
use nubless_core::profile::{Match, ValueKind};
use nubless_core::profiles::gm950b::PROFILE;
use nubless_core::safety::describe_allowed;
use nubless_core::transport::Transport;
use nubless_core::{Controller, Timing, WritePolicy};

#[derive(Parser)]
#[command(
    name = "nubless",
    version,
    about = "Control your LG 27GM950B without touching the joystick nub"
)]
struct Cli {
    /// Use a simulated 27GM950B instead of real hardware.
    #[arg(long, global = true)]
    mock: bool,
    /// Allow writing controls that are only "strong" candidates (for people
    /// helping to map the monitor). Deny-list and confirm-or-revert still apply.
    #[arg(long, global = true)]
    experimental: bool,
    /// Pick a display by the number shown in `nubless list`.
    #[arg(long, global = true)]
    monitor: Option<usize>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List connected displays and whether each one is a supported model (read-only).
    List,
    /// Show every control nubless knows for the 27GM950B, how sure we are, and whether it can be changed.
    Controls,
    /// Read one control, or all of them.
    Get { control: Option<String> },
    /// Change a control. VALUE is a number or a label such as "Extended".
    Set {
        control: String,
        value: String,
        /// Keep the change without the "Keep this change?" question. Only use
        /// this when you can see the screen.
        #[arg(long)]
        yes: bool,
        /// Seconds before an unconfirmed change reverts on its own.
        #[arg(long, default_value_t = KEEP_SECONDS)]
        revert_after: u64,
    },
    /// Undo the most recent change.
    Undo,
    /// Show the change journal.
    Journal,
    /// Save or restore all writable settings.
    Snapshot {
        #[command(subcommand)]
        action: SnapshotAction,
    },
    /// Print the monitor's raw capabilities string.
    Caps,
    /// Read-only mapping tools: see which codes change when you change a setting.
    Probe {
        #[command(subcommand)]
        action: ProbeAction,
    },
}

#[derive(Subcommand)]
enum ProbeAction {
    /// Save every readable code to FILE.
    Save { file: PathBuf },
    /// Compare the monitor with FILE and print what changed.
    Diff { file: PathBuf },
    /// Print every change as it happens.
    Watch {
        #[arg(long, default_value_t = 180)]
        seconds: u64,
        /// Only watch these codes (hex, comma-separated), e.g. 0x15. Much faster.
        #[arg(long, value_delimiter = ',')]
        codes: Vec<String>,
    },
}

#[derive(Subcommand)]
enum SnapshotAction {
    /// Save current settings to FILE.
    Save { file: PathBuf },
    /// Restore settings from FILE, or from the baseline taken before nubless first changed anything.
    Restore {
        file: Option<PathBuf>,
        #[arg(long, conflicts_with = "file")]
        baseline: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

type AnyResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// How long "Keep this change?" waits. The desktop app waits the same time
/// (`KEEP_MS` in `apps/desktop/src/lib/store.svelte.ts`).
const KEEP_SECONDS: u64 = 5;

fn state_dir() -> PathBuf {
    nubless_core::paths::state_dir()
}

fn run(cli: Cli) -> AnyResult {
    match &cli.command {
        Command::Controls => return print_controls(cli.experimental),
        Command::List => return list_displays(cli.mock),
        _ => {}
    }

    let policy = if cli.experimental {
        WritePolicy::Experimental
    } else {
        WritePolicy::Safe
    };
    let (transport, journal, timing): (Box<dyn Transport>, Journal, Timing) = if cli.mock {
        (
            Box::new(MockMonitor::gm950b()),
            Journal::in_memory(),
            Timing::instant(),
        )
    } else {
        let display = pick_display(cli.monitor)?;
        (
            Box::new(display),
            Journal::open(state_dir().join("journal.jsonl"))?,
            Timing::HARDWARE,
        )
    };
    let mut ctl = Controller::new(transport, &PROFILE, policy, timing, journal)?;
    // Capture the user's original settings on every connection, before any write.
    if !cli.mock {
        ensure_baseline(&mut ctl)?;
    }

    match cli.command {
        Command::Get { control } => get(&mut ctl, control.as_deref()),
        Command::Set {
            control,
            value,
            yes,
            revert_after,
        } => set(
            &mut ctl,
            &control,
            &value,
            yes,
            Duration::from_secs(revert_after),
        ),
        Command::Undo => {
            ctl.undo_last()?;
            println!("Undone.");
            Ok(())
        }
        Command::Journal => {
            for e in &ctl.journal().entries {
                println!(
                    "{:>15}  {:<18} 0x{:02X}  {:>5} -> {:<5} {:?}",
                    e.unix_ms, e.control, e.vcp, e.old, e.new, e.state
                );
            }
            Ok(())
        }
        Command::Snapshot { action } => snapshot(&mut ctl, action),
        Command::Caps => {
            println!("{}", ctl.capabilities().raw);
            Ok(())
        }
        Command::Probe { action } => match action {
            ProbeAction::Save { file } => Ok(probe::save(&mut ctl, &file)?),
            ProbeAction::Diff { file } => probe::diff(&mut ctl, &file),
            ProbeAction::Watch { seconds, codes } => {
                let only = codes
                    .iter()
                    .map(|c| {
                        u8::from_str_radix(c.trim_start_matches("0x").trim_start_matches("0X"), 16)
                    })
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| "codes must be hex, like 0x15")?;
                probe::watch(&mut ctl, seconds, &only);
                Ok(())
            }
        },
        Command::Controls | Command::List => unreachable!(),
    }
}

fn list_displays(mock: bool) -> AnyResult {
    if mock {
        println!("#0 [mock] simulated 27GM950B  -> supported (mock)");
        return Ok(());
    }
    let displays = nubless_hw::enumerate(&PROFILE);
    if displays.is_empty() {
        println!("No DDC/CI displays found. Check that DDC/CI is enabled in the monitor's menu and that no dock or KVM is in the way.");
    }
    for d in &displays {
        let verdict = match d.matched {
            Match::Edid => "supported (EDID)",
            Match::Fingerprint => "supported (capabilities fingerprint)",
            Match::No => "not supported",
        };
        println!("{}  -> {verdict}", d.summary());
    }
    Ok(())
}

fn pick_display(choice: Option<usize>) -> AnyResult<nubless_hw::HwDisplay> {
    let mut displays = nubless_hw::enumerate(&PROFILE);
    if let Some(i) = choice {
        let pos = displays
            .iter()
            .position(|d| d.index == i)
            .ok_or(format!("no display #{i}; see `nubless list`"))?;
        let d = displays.swap_remove(pos);
        if d.matched == Match::No {
            return Err(format!(
                "display #{i} is not a {}; nubless only controls supported models",
                PROFILE.model
            )
            .into());
        }
        return Ok(d);
    }
    let mut supported: Vec<_> = displays
        .into_iter()
        .filter(|d| d.matched != Match::No)
        .collect();
    match supported.len() {
        0 => Err(format!("no {} found; see `nubless list`", PROFILE.model).into()),
        1 => Ok(supported.remove(0)),
        n => Err(format!(
            "{n} supported displays found; pick one with --monitor (see `nubless list`)"
        )
        .into()),
    }
}

fn print_controls(experimental: bool) -> AnyResult {
    let policy = if experimental {
        WritePolicy::Experimental
    } else {
        WritePolicy::Safe
    };
    println!(
        "{} controls ({} policy)\n",
        PROFILE.model,
        if experimental { "experimental" } else { "safe" }
    );
    for c in PROFILE.controls {
        let can = if policy.allows(c) {
            "writable"
        } else {
            "read-only"
        };
        println!(
            "{:<18} 0x{:02X}  {:<15} {:<9}  {}",
            c.id,
            c.vcp,
            format!("{:?}", c.confidence),
            can,
            describe_allowed(c)
        );
        println!("{:<24}{}", "", c.risk.describe());
        println!("{:<24}{}\n", "", c.note);
    }
    Ok(())
}

fn format_value<T: Transport>(ctl: &Controller<T>, id: &str, raw: u16) -> String {
    let Ok(c) = ctl.control(id) else {
        return raw.to_string();
    };
    match (c.choice_label(raw), c.kind) {
        (Some(label), _) => format!("{label} ({raw})"),
        (None, ValueKind::Info) if c.vcp == 0xC9 => {
            format!("{}.{:02X} (0x{raw:04X})", raw >> 8, raw & 0xFF)
        }
        (None, ValueKind::Choice { .. }) => format!("{raw} (unmapped)"),
        _ => raw.to_string(),
    }
}

fn get<T: Transport>(ctl: &mut Controller<T>, id: Option<&str>) -> AnyResult {
    let ids: Vec<&str> = match id {
        Some(id) => vec![ctl.control(id)?.id],
        None => ctl.profile().controls.iter().map(|c| c.id).collect(),
    };
    for id in ids {
        match ctl.read(id) {
            Ok(r) => println!("{:<18} {}", id, format_value(ctl, id, r.current)),
            Err(e) => println!("{:<18} unknown ({e})", id),
        }
    }
    Ok(())
}

fn parse_value(control: &nubless_core::Control, s: &str) -> Result<u16, String> {
    let norm = |x: &str| x.to_ascii_lowercase().replace([' ', '-', '_'], "");
    if let ValueKind::Choice { choices } | ValueKind::SubCommand { choices, .. } = control.kind {
        if let Some(c) = choices.iter().find(|c| norm(c.label) == norm(s)) {
            return Ok(c.raw);
        }
    }
    let parsed = match s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        Some(hex) => u16::from_str_radix(hex, 16),
        None => s.parse(),
    };
    parsed.map_err(|_| {
        format!(
            "'{s}' is not a value for {} ({})",
            control.id,
            describe_allowed(control)
        )
    })
}

fn set<T: Transport>(
    ctl: &mut Controller<T>,
    id: &str,
    value: &str,
    yes: bool,
    revert_after: Duration,
) -> AnyResult {
    let control = ctl.control(id)?;
    let raw = parse_value(control, value)?;
    let applied = ctl.write(control.id, raw)?;
    let Some(index) = applied.journal_index else {
        println!(
            "{} is already {}.",
            control.label,
            format_value(ctl, control.id, raw)
        );
        return Ok(());
    };
    println!(
        "{}: {} -> {}",
        control.label,
        format_value(ctl, control.id, applied.old),
        format_value(ctl, control.id, applied.new)
    );
    if applied.needs_confirmation && !yes {
        print!(
            "Keep this change? [y/N] (reverts in {} s) ",
            revert_after.as_secs()
        );
        io::stdout().flush()?;
        if !confirmed_within(revert_after) {
            ctl.revert(index)?;
            println!(
                "\nReverted to {}.",
                format_value(ctl, control.id, applied.old)
            );
            return Ok(());
        }
        println!("Kept.");
    }
    Ok(())
}

/// Waits for "y" on stdin. Anything else, EOF or the timeout means no.
fn confirmed_within(timeout: Duration) -> bool {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut line = String::new();
        let _ = io::stdin().lock().read_line(&mut line);
        let _ = tx.send(line);
    });
    matches!(rx.recv_timeout(timeout), Ok(line) if line.trim().eq_ignore_ascii_case("y"))
}

/// Before the first change ever, remember how the monitor was set up, so
/// `nubless snapshot restore --baseline` can always put it back.
fn ensure_baseline<T: Transport>(ctl: &mut Controller<T>) -> AnyResult {
    let path = state_dir().join("baseline.json");
    let (_, saved) = nubless_core::baseline::ensure(&path, ctl)?;
    if saved {
        eprintln!("Saved your original settings: {}", path.display());
    }
    Ok(())
}

fn snapshot<T: Transport>(ctl: &mut Controller<T>, action: SnapshotAction) -> AnyResult {
    match action {
        SnapshotAction::Save { file } => {
            let snap = ctl.snapshot();
            std::fs::write(&file, serde_json::to_string_pretty(&snap)?)?;
            println!("Saved {} values to {}", snap.values.len(), file.display());
        }
        SnapshotAction::Restore { file, baseline } => {
            let path = match (file, baseline) {
                (Some(f), _) => f,
                (None, true) => state_dir().join("baseline.json"),
                (None, false) => return Err("give a FILE or --baseline".into()),
            };
            let snap: Snapshot = serde_json::from_str(&std::fs::read_to_string(&path)?)?;
            if snap.model != PROFILE.model {
                return Err(format!(
                    "{} is a snapshot for {}, not {}",
                    path.display(),
                    snap.model,
                    PROFILE.model
                )
                .into());
            }
            let mut failed = 0;
            for (id, result) in ctl.restore(&snap) {
                match result {
                    Ok(()) => println!("{id:<18} ok"),
                    Err(e) => {
                        failed += 1;
                        println!("{id:<18} {e}");
                    }
                }
            }
            if failed > 0 {
                return Err(format!("{failed} value(s) could not be restored").into());
            }
        }
    }
    Ok(())
}
