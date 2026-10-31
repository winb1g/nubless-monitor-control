//! The `nubless` binary end to end against the simulated monitor (`--mock`):
//! what a person types, what they are shown, and the exit code.

use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

struct Run {
    ok: bool,
    out: String,
    err: String,
    took: Duration,
}

/// How the terminal answers "Keep this change?".
enum Stdin {
    /// Closed (EOF), like running from a script.
    Closed,
    /// A typed line.
    Line(&'static str),
    /// Open but silent: nobody is at the keyboard.
    Silent,
}

fn nubless(args: &[&str], stdin: Stdin) -> Run {
    let home = std::env::temp_dir().join(format!("nubless-cli-test-{}", std::process::id()));
    let mut child = Command::new(env!("CARGO_BIN_EXE_nubless"))
        .args(args)
        .env("NUBLESS_HOME", &home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start nubless");
    let started = Instant::now();
    let mut pipe = child.stdin.take().expect("stdin");
    let held = match stdin {
        Stdin::Closed => {
            drop(pipe);
            None
        }
        Stdin::Line(text) => {
            pipe.write_all(text.as_bytes()).unwrap();
            drop(pipe);
            None
        }
        Stdin::Silent => Some(pipe),
    };
    let out = child.wait_with_output().expect("wait");
    drop(held);
    Run {
        ok: out.status.success(),
        out: String::from_utf8_lossy(&out.stdout).into_owned(),
        err: String::from_utf8_lossy(&out.stderr).into_owned(),
        took: started.elapsed(),
    }
}

fn mock(args: &[&str]) -> Run {
    let mut all = vec!["--mock"];
    all.extend_from_slice(args);
    nubless(&all, Stdin::Closed)
}

fn labs(args: &[&str]) -> Run {
    let mut all = vec!["--mock", "--experimental"];
    all.extend_from_slice(args);
    nubless(&all, Stdin::Closed)
}

#[test]
fn list_shows_the_simulated_monitor() {
    let r = mock(&["list"]);
    assert!(r.ok && r.out.contains("27GM950B"), "{}", r.out);
}

#[test]
fn get_reads_every_control_and_one_by_name() {
    let all = mock(&["get"]);
    assert!(all.ok);
    for line in ["brightness", "response-time", "firmware", "picture-mode"] {
        assert!(all.out.contains(line), "missing {line}:\n{}", all.out);
    }
    assert!(all.out.contains("3.05"), "firmware level: {}", all.out);
    let one = mock(&["get", "brightness"]);
    assert!(one.ok && one.out.contains("80"), "{}", one.out);
    assert!(!mock(&["get", "no-such-control"]).ok);
}

#[test]
fn safe_controls_change_without_a_question() {
    let r = mock(&["set", "brightness", "40"]);
    assert!(r.ok, "{}", r.err);
    assert!(r.out.contains("Brightness: 80 -> 40"), "{}", r.out);
    assert!(!r.out.contains("Keep this change"), "{}", r.out);
}

#[test]
fn setting_what_is_already_set_says_so() {
    let r = mock(&["set", "brightness", "80"]);
    assert!(r.ok && r.out.contains("already"), "{}", r.out);
}

#[test]
fn values_outside_the_safe_range_are_refused() {
    for (control, value) in [
        ("brightness", "500"),
        ("contrast", "5"),
        ("brightness", "abc"),
    ] {
        let r = mock(&["set", control, value]);
        assert!(!r.ok, "{control} {value}");
        assert!(r.err.contains("error"), "{}", r.err);
    }
}

#[test]
fn hardware_confirmed_controls_need_no_labs() {
    // Response Time passed its hardware card (fw 3.05, 2026-10-30): the label works
    // as well as the number, without --experimental.
    assert!(
        nubless(
            &["--mock", "set", "response-time", "Faster", "--yes"],
            Stdin::Closed
        )
        .ok
    );
    // AI Upscaling and the color presets passed theirs too, by label, in the same way.
    for args in [
        ["--mock", "set", "ai-upscaling", "low"],
        ["--mock", "set", "color-preset", "Cool"],
    ] {
        let r = nubless(&args, Stdin::Closed);
        assert!(r.ok, "{args:?}: {}", r.err);
        assert!(r.out.contains("->"), "{}", r.out);
    }
    // A candidate is refused with or without Labs, and nothing is written.
    for args in [
        &["--mock", "set", "red-gain", "60"][..],
        &["--mock", "--experimental", "set", "red-gain", "60"][..],
    ] {
        let r = nubless(args, Stdin::Closed);
        assert!(!r.ok, "{args:?}");
        assert!(
            !r.out.contains("->"),
            "nothing may have been written: {}",
            r.out
        );
    }
}

#[test]
fn motion_blur_says_to_turn_adaptive_sync_off_first() {
    // The simulated monitor starts on Adaptive-Sync Extended.
    let r = mock(&["set", "motion-blur", "on"]);
    assert!(!r.ok, "{}", r.out);
    assert!(r.err.contains("Turn Adaptive-Sync off first"), "{}", r.err);
    assert!(!r.out.contains("->"), "{}", r.out);
}

#[test]
fn the_monitors_dead_ends_are_never_written() {
    for args in [["set", "input", "17"], ["set", "firmware", "1"]] {
        let r = labs(&args);
        assert!(!r.ok, "{args:?}");
        assert!(!r.out.contains("->"), "{}", r.out);
    }
}

#[test]
fn a_risky_change_asks_and_y_keeps_it() {
    let r = nubless(
        &["--mock", "--experimental", "set", "response-time", "3"],
        Stdin::Line("y\n"),
    );
    assert!(r.ok, "{}", r.err);
    assert!(
        r.out.contains("Keep this change? [y/N] (reverts in 5 s)"),
        "{}",
        r.out
    );
    assert!(r.out.contains("Kept."), "{}", r.out);
    assert!(!r.out.contains("Reverted"), "{}", r.out);
}

#[test]
fn a_risky_change_is_put_back_on_no_or_a_closed_terminal() {
    for stdin in [Stdin::Line("n\n"), Stdin::Line("\n"), Stdin::Closed] {
        let r = nubless(
            &["--mock", "--experimental", "set", "response-time", "3"],
            stdin,
        );
        assert!(r.ok, "{}", r.err);
        assert!(r.out.contains("Reverted to Fast (2)."), "{}", r.out);
    }
}

#[test]
fn an_unanswered_question_reverts_when_the_time_runs_out() {
    let r = nubless(
        &[
            "--mock",
            "--experimental",
            "set",
            "black-stabilizer",
            "90",
            "--revert-after",
            "1",
        ],
        Stdin::Silent,
    );
    assert!(r.ok, "{}", r.err);
    assert!(r.out.contains("reverts in 1 s"), "{}", r.out);
    assert!(r.out.contains("Reverted to 50."), "{}", r.out);
    assert!(
        r.took >= Duration::from_millis(900) && r.took < Duration::from_secs(4),
        "waited {:?}",
        r.took
    );
}

#[test]
fn yes_skips_the_question() {
    let r = labs(&["set", "adaptive-sync", "off", "--yes"]);
    assert!(r.ok, "{}", r.err);
    assert!(!r.out.contains("Keep this change"), "{}", r.out);
    assert!(r.out.contains("->"), "{}", r.out);
}

#[test]
fn the_default_wait_is_five_seconds() {
    let r = mock(&["set", "--help"]);
    assert!(r.out.contains("[default: 5]"), "{}", r.out);
}

#[test]
fn controls_explains_what_is_locked_without_labs() {
    // Nothing is in Labs now; Red gain (still a candidate) is the read-only example.
    let safe = mock(&["controls"]);
    let labs = labs(&["controls"]);
    assert!(safe.ok && labs.ok);
    assert!(safe.out.contains("safe policy") && labs.out.contains("experimental policy"));
    let row = |text: &str, id: &str| {
        text.lines()
            .find(|l| l.starts_with(id))
            .unwrap_or_default()
            .to_string()
    };
    assert!(
        row(&safe.out, "red-gain").contains("read-only"),
        "{}",
        safe.out
    );
    assert!(
        row(&labs.out, "red-gain").contains("read-only"),
        "{}",
        labs.out
    );
    assert!(
        row(&safe.out, "response-time").contains("writable"),
        "{}",
        safe.out
    );
    assert!(row(&safe.out, "brightness").contains("writable"));
}

#[test]
fn snapshots_round_trip_and_refuse_the_wrong_monitor() {
    let dir = std::env::temp_dir().join(format!("nubless-cli-snap-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("snap.json");
    let path = file.to_str().unwrap();
    let saved = mock(&["snapshot", "save", path]);
    assert!(saved.ok && saved.out.contains("Saved"), "{}", saved.out);
    let restored = mock(&["snapshot", "restore", path]);
    assert!(
        restored.ok && restored.out.contains("brightness"),
        "{}",
        restored.out
    );
    assert!(!restored.out.contains("could not"), "{}", restored.out);

    let json = std::fs::read_to_string(&file)
        .unwrap()
        .replace("27GM950B", "34WK95U");
    std::fs::write(&file, json).unwrap();
    let wrong = mock(&["snapshot", "restore", path]);
    assert!(!wrong.ok && wrong.err.contains("34WK95U"), "{}", wrong.err);

    std::fs::write(&file, "not json").unwrap();
    assert!(!mock(&["snapshot", "restore", path]).ok);
    assert!(
        !mock(&[
            "snapshot",
            "restore",
            dir.join("missing.json").to_str().unwrap()
        ])
        .ok
    );
    assert!(
        !mock(&["snapshot", "restore"]).ok,
        "needs a FILE or --baseline"
    );
}

#[test]
fn undo_with_nothing_to_undo_is_an_error_not_a_crash() {
    let r = mock(&["undo"]);
    assert!(!r.ok && r.err.contains("nothing to undo"), "{}", r.err);
}

#[test]
fn the_capabilities_string_is_printed_for_bug_reports() {
    let r = mock(&["caps"]);
    assert!(r.ok && r.out.starts_with('('), "{}", r.out);
}
