//! `nubless probe`: read-only tools for mapping the monitor. Nothing here writes.
//!
//! Change one setting in LG Switch or the OSD and see which VCP codes move.

use std::collections::BTreeMap;
use std::io::Write;
use std::time::{Duration, Instant};

use nubless_core::transport::Transport;
use nubless_core::Controller;

/// Codes worth watching on the 27GM950B: everything it declares plus LG's
/// private codes from LG Switch's opcode table. Factory-restore and calibration
/// codes are skipped: reading them is harmless per MCCS, but there's nothing to learn.
const LG_EXTRAS: &[u8] = &[
    0x45, 0x47, 0x69, 0x6A, 0x72, 0x87, 0x8A, 0xA1, 0xA2, 0xA6, 0xA7, 0xAA, 0xAF, 0xB0, 0xB1, 0xDB,
    0xDC, 0xE0, 0xE1, 0xE2, 0xE3, 0xEC, 0xED, 0xEE, 0xF0, 0xF1, 0xF2, 0xF3, 0xFB, 0xFC,
];
const SKIP: &[u8] = &[
    0x04, 0x05, 0x06, 0x08, 0x0A, 0xE4, 0xE5, 0xE6, 0xE7, 0xE8, 0xE9, 0xEA, 0xEB,
];

pub fn codes<T: Transport>(ctl: &Controller<T>) -> Vec<u8> {
    let mut v: Vec<u8> = ctl
        .capabilities()
        .vcp
        .keys()
        .copied()
        .chain(LG_EXTRAS.iter().copied())
        .collect();
    v.sort_unstable();
    v.dedup();
    v.retain(|c| !SKIP.contains(c));
    v
}

/// One pass over `codes`; unanswered codes are left out.
pub fn read_all<T: Transport>(ctl: &mut Controller<T>, codes: &[u8]) -> BTreeMap<u8, u16> {
    codes
        .iter()
        .filter_map(|&c| ctl.read_code(c).ok().map(|r| (c, r.current)))
        .collect()
}

fn name<T: Transport>(ctl: &Controller<T>, code: u8) -> String {
    ctl.profile()
        .controls
        .iter()
        .find(|c| c.vcp == code)
        .map(|c| c.id.to_string())
        .unwrap_or_default()
}

pub fn save<T: Transport>(ctl: &mut Controller<T>, path: &std::path::Path) -> std::io::Result<()> {
    let list = codes(ctl);
    let values: BTreeMap<String, u16> = read_all(ctl, &list)
        .into_iter()
        .map(|(c, v)| (format!("0x{c:02X}"), v))
        .collect();
    std::fs::write(path, serde_json::to_string_pretty(&values)?)?;
    println!("Saved {} codes to {}", values.len(), path.display());
    Ok(())
}

pub fn diff<T: Transport>(
    ctl: &mut Controller<T>,
    path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let before: BTreeMap<String, u16> = serde_json::from_str(&std::fs::read_to_string(path)?)?;
    let list = codes(ctl);
    let now = read_all(ctl, &list);
    let mut changed = 0;
    for (code, v) in &now {
        let key = format!("0x{code:02X}");
        if let Some(old) = before.get(&key) {
            if old != v {
                changed += 1;
                println!(
                    "{key:<6} {:<18} {old:>6} (0x{old:04X}) -> {v:>6} (0x{v:04X})",
                    name(ctl, *code)
                );
            }
        }
    }
    if changed == 0 {
        println!("No changes.");
    }
    Ok(())
}

/// Poll and print every change with a timestamp, for `seconds`.
pub fn watch<T: Transport>(ctl: &mut Controller<T>, seconds: u64) {
    let start = Instant::now();
    let mut last = read_all(ctl, &codes(ctl));
    // Only poll codes that answered; a silent code costs three retries every pass.
    let list: Vec<u8> = last.keys().copied().collect();
    println!(
        "Watching {} codes for {seconds} s. Change settings now; wait ~5 s between changes.",
        list.len()
    );
    let _ = std::io::stdout().flush();
    let mut cycle = 0u32;
    while start.elapsed() < Duration::from_secs(seconds) {
        cycle += 1;
        for &c in &list {
            let Ok(r) = ctl.read_code(c) else { continue };
            let old = last.insert(c, r.current);
            if let Some(old) = old.filter(|o| *o != r.current) {
                println!(
                    "{:>6.1}s  0x{c:02X} {:<18} {old:>6} (0x{old:04X}) -> {:>6} (0x{:04X})",
                    start.elapsed().as_secs_f32(),
                    name(ctl, c),
                    r.current,
                    r.current
                );
                let _ = std::io::stdout().flush();
            }
        }
        if cycle == 1 {
            println!(
                "(one full pass takes {:.1} s)",
                start.elapsed().as_secs_f32()
            );
            let _ = std::io::stdout().flush();
        }
    }
    println!("Done.");
}
