//! The controller against a monitor that misbehaves the way real DDC/CI links do
//! (see [`crate::mock::Faults`]), plus the write → keep-or-undo protocol for every
//! control Labs can reach.

use super::*;
use crate::journal::EntryState;
use crate::mock::MockMonitor;
use crate::profiles::gm950b::PROFILE;
use crate::transport::VcpReading;

fn ctl(m: MockMonitor, policy: WritePolicy) -> Controller<MockMonitor> {
    Controller::new(m, &PROFILE, policy, Timing::instant(), Journal::in_memory()).unwrap()
}

/// A valid value for `c` that differs from `current`.
fn other_value(c: &Control, current: u16) -> u16 {
    match c.kind {
        ValueKind::Range { min, max } => {
            if current == max {
                min
            } else {
                max
            }
        }
        ValueKind::Choice { choices } | ValueKind::SubCommand { choices, .. } => choices
            .iter()
            .map(|x| x.raw)
            .find(|&r| r != current)
            .unwrap(),
        ValueKind::Info => unreachable!("{} is read-only", c.id),
    }
}

// ---- dropped, lost and lagging traffic ---------------------------------------

#[test]
fn dropped_reads_inside_the_retry_budget_are_invisible() {
    let mut m = MockMonitor::gm950b();
    m.faults.drop_reads.insert(0x10, 2);
    let mut c = ctl(m, WritePolicy::Safe);
    c.write("brightness", 40).unwrap();
    assert_eq!(c.read("brightness").unwrap().current, 40);
}

#[test]
fn dropped_reads_beyond_the_budget_stop_the_write_cold() {
    let mut m = MockMonitor::gm950b();
    m.faults.drop_reads.insert(0x10, 3);
    let mut c = ctl(m, WritePolicy::Safe);
    assert!(matches!(
        c.write("brightness", 40),
        Err(ControlError::CannotReadCurrent("brightness"))
    ));
    assert!(c.transport.writes.is_empty());
    assert!(c.journal().entries.is_empty());
}

#[test]
fn a_write_lost_on_the_wire_leaves_the_value_alone() {
    let mut m = MockMonitor::gm950b();
    m.faults.fail_writes = 1;
    let mut c = ctl(m, WritePolicy::Safe);
    assert!(matches!(
        c.write("brightness", 40),
        Err(ControlError::Transport(_))
    ));
    assert_eq!(c.read("brightness").unwrap().current, 80);
    assert_eq!(c.journal().entries[0].state, EntryState::RolledBack);
}

#[test]
fn a_swallowed_write_is_reported_and_not_retried_blindly() {
    let mut m = MockMonitor::gm950b();
    m.faults.swallow_writes = 1;
    let mut c = ctl(m, WritePolicy::Safe);
    let err = c.write("brightness", 40).unwrap_err();
    assert!(matches!(err, ControlError::NotApplied { .. }), "{err}");
    assert_eq!(c.read("brightness").unwrap().current, 80);
    assert_eq!(c.journal().entries[0].state, EntryState::RolledBack);
    assert_eq!(
        c.transport.writes.len(),
        1,
        "nothing was applied, so nothing to put back"
    );
}

#[test]
fn a_lagging_read_back_does_not_undo_a_change_that_landed() {
    // Response time and VRR resync the panel; the first read-back can still be old.
    let mut m = MockMonitor::gm950b();
    m.faults.stale_reads.insert(0xF7, 1);
    let mut c = ctl(m, WritePolicy::Experimental);
    let a = c.write("response-time", 3).unwrap();
    assert!(a.needs_confirmation);
    assert_eq!(c.read("response-time").unwrap().current, 3);
    assert_eq!(c.journal().entries[0].state, EntryState::Applied);
    assert_eq!(c.transport.writes.len(), 1, "no corrective write was sent");
}

#[test]
fn a_monitor_that_goes_quiet_while_the_panel_resyncs_is_waited_for() {
    // Turning VRR off blanks the panel; the monitor answers nothing for seconds.
    let mut m = MockMonitor::gm950b();
    m.faults.quiet_after_write.insert(0xF8, 12);
    let mut c = ctl(m, WritePolicy::Experimental);
    let a = c.write("adaptive-sync", 0).unwrap();
    assert!(a.needs_confirmation);
    assert_eq!(c.read("adaptive-sync").unwrap().current, 0);
    assert_eq!(c.journal().entries[0].state, EntryState::Applied);
    assert_eq!(
        c.transport.writes.len(),
        1,
        "no corrective write while it was busy"
    );
}

#[test]
fn a_monitor_that_never_comes_back_is_reported_in_plain_words() {
    let mut m = MockMonitor::gm950b();
    m.faults.quiet_after_write.insert(0xF8, 10_000);
    let mut c = ctl(m, WritePolicy::Experimental);
    let err = c.write("adaptive-sync", 0).unwrap_err();
    let text = err.to_string();
    assert!(text.contains("did not answer"), "{text}");
    assert!(!text.contains("None"), "{text}");
}

#[test]
fn a_refusal_is_not_mistaken_for_a_busy_monitor() {
    // The monitor answers with the old value: it said no. Do not wait it out.
    let mut m = MockMonitor::gm950b();
    m.ignore_writes_to.push(0xF8);
    let mut c = ctl(m, WritePolicy::Experimental);
    let err = c.write("adaptive-sync", 0).unwrap_err();
    assert!(err.to_string().contains("it reads 3"), "{err}");
}

#[test]
fn all_zero_replies_are_not_values() {
    // Some LG firmware answers codes it does not implement with 0 / 0. Writing
    // "the old value" back to that would be inventing a value.
    for (code, id, policy) in [
        (0x10, "brightness", WritePolicy::Safe),
        (0xF7, "response-time", WritePolicy::Experimental),
    ] {
        let mut m = MockMonitor::gm950b();
        m.faults.zero_reply.push(code);
        let mut c = ctl(m, policy);
        let control = c.control(id).unwrap();
        let target = other_value(control, 0);
        assert!(
            matches!(c.write(id, target), Err(ControlError::CannotReadCurrent(_))),
            "{id}"
        );
        assert!(c.transport.writes.is_empty(), "{id}");
    }
}

// ---- the link itself ---------------------------------------------------------

#[test]
fn capabilities_are_retried_through_noise_but_not_forever() {
    let build = |m: MockMonitor| {
        Controller::new(
            m,
            &PROFILE,
            WritePolicy::Safe,
            Timing::instant(),
            Journal::in_memory(),
        )
    };
    let mut m = MockMonitor::gm950b();
    m.faults.caps_garbled = 2;
    m.faults.caps_fail = 3;
    assert!(build(m).is_ok(), "five bad answers, then a good one");

    let mut m = MockMonitor::gm950b();
    m.faults.caps_garbled = 6;
    assert!(matches!(build(m), Err(ControlError::Caps(_))));

    let mut m = MockMonitor::gm950b();
    m.faults.offline = true;
    assert!(matches!(build(m), Err(ControlError::Transport(_))));
}

#[test]
fn a_monitor_that_goes_away_mid_session_fails_cleanly() {
    let mut c = ctl(MockMonitor::gm950b(), WritePolicy::Experimental);
    c.write("brightness", 40).unwrap();
    c.transport.faults.offline = true;
    assert!(matches!(
        c.read("brightness"),
        Err(ControlError::Transport(_))
    ));
    assert!(matches!(
        c.write("brightness", 50),
        Err(ControlError::CannotReadCurrent(_))
    ));
    assert!(matches!(
        c.undo_last(),
        Err(ControlError::NotApplied { .. } | ControlError::Transport(_))
    ));
    // And it picks up where it left off when the link comes back.
    c.transport.faults.offline = false;
    assert_eq!(c.read("brightness").unwrap().current, 40);
    c.undo_last().unwrap();
    assert_eq!(c.read("brightness").unwrap().current, 80);
}

#[test]
fn restore_reports_each_failure_and_can_simply_be_run_again() {
    let mut c = ctl(MockMonitor::gm950b(), WritePolicy::Experimental);
    let snap = c.snapshot();
    c.write("brightness", 5).unwrap();
    c.write("contrast", 30).unwrap();
    c.transport.ignore_writes_to.push(0x10);
    let results = c.restore(&snap);
    let failed: Vec<_> = results.iter().filter(|(_, r)| r.is_err()).collect();
    assert_eq!(failed.len(), 1, "{results:?}");
    assert_eq!(failed[0].0, "brightness");
    assert_eq!(
        c.read("contrast").unwrap().current,
        70,
        "the rest still went back"
    );

    c.transport.ignore_writes_to.clear();
    assert!(c.restore(&snap).iter().all(|(_, r)| r.is_ok()));
    assert_eq!(c.read("brightness").unwrap().current, 80);
}

// ---- the keep-or-undo protocol, for every control Labs can reach -------------

#[test]
fn every_writable_control_writes_reads_back_and_undoes() {
    let mut covered = Vec::new();
    for control in PROFILE
        .controls
        .iter()
        .filter(|c| WritePolicy::Experimental.allows(c))
    {
        let mut m = MockMonitor::gm950b();
        if control.id == "motion-blur" {
            // Only available while Adaptive-Sync is Off.
            m.values.insert(
                0xF8,
                VcpReading {
                    current: 0,
                    max: 255,
                },
            );
        }
        let mut c = ctl(m, WritePolicy::Experimental);
        let before = c.read(control.id).unwrap().current;
        let target = other_value(control, before);
        let applied = c.write(control.id, target).unwrap();
        assert_eq!(
            (applied.old, applied.new),
            (before, target),
            "{}",
            control.id
        );
        assert_eq!(
            applied.needs_confirmation,
            control.needs_confirm_or_revert(),
            "{}",
            control.id
        );
        assert_eq!(
            c.read(control.id).unwrap().current,
            target,
            "{}",
            control.id
        );
        c.revert(applied.journal_index.unwrap()).unwrap();
        assert_eq!(
            c.read(control.id).unwrap().current,
            before,
            "{}",
            control.id
        );
        assert_eq!(
            c.journal().entries[0].state,
            EntryState::Reverted,
            "{}",
            control.id
        );
        covered.push(control.id);
    }
    for id in [
        "black-stabilizer",
        "response-time",
        "adaptive-sync",
        "volume",
        "mute",
    ] {
        assert!(covered.contains(&id), "{id} is not covered");
    }
}

#[test]
fn risky_controls_ask_to_be_kept() {
    let risky: Vec<_> = PROFILE
        .controls
        .iter()
        .filter(|c| WritePolicy::Experimental.allows(c) && c.needs_confirm_or_revert())
        .map(|c| c.id)
        .collect();
    for id in ["black-stabilizer", "response-time", "adaptive-sync"] {
        assert!(risky.contains(&id), "{id} must be confirm-or-revert");
    }
}

#[test]
fn an_undo_the_monitor_refuses_is_reported_and_stays_undoable() {
    let mut c = ctl(MockMonitor::gm950b(), WritePolicy::Experimental);
    let index = c.write("response-time", 3).unwrap().journal_index.unwrap();
    c.transport.ignore_writes_to.push(0xF7);
    let err = c.revert(index).unwrap_err();
    assert!(
        matches!(
            err,
            ControlError::NotApplied {
                outcome: "undo did not take",
                ..
            }
        ),
        "{err}"
    );
    assert_eq!(c.journal().entries[index].state, EntryState::Applied);
    c.transport.ignore_writes_to.clear();
    c.revert(index).unwrap();
    assert_eq!(c.read("response-time").unwrap().current, 2);
}

#[test]
fn unconfirmed_controls_send_nothing() {
    // Nothing is in Labs now; the controls that are not writable must stay silent.
    let silent: Vec<_> = PROFILE
        .controls
        .iter()
        .filter(|c| !WritePolicy::Experimental.allows(c) && !matches!(c.kind, ValueKind::Info))
        .collect();
    assert!(!silent.is_empty());
    for control in silent {
        let mut c = ctl(MockMonitor::gm950b(), WritePolicy::Safe);
        let target = other_value(control, 0);
        assert!(
            matches!(c.write(control.id, target), Err(ControlError::Safety(_))),
            "{}",
            control.id
        );
        assert!(c.transport.writes.is_empty(), "{}", control.id);
    }
}

// ---- endurance ---------------------------------------------------------------

#[test]
fn a_thousand_write_read_cycles_leave_no_stuck_state() {
    // Roadmap phase 1 exit criterion, against a link that drops every 7th read.
    let mut m = MockMonitor::gm950b();
    m.flaky_every = 7;
    let mut c = ctl(m, WritePolicy::Safe);
    let baseline = c.snapshot();
    for i in 0..1000u16 {
        let (id, value) = if i % 2 == 0 {
            ("brightness", 10 + i % 91)
        } else {
            ("contrast", 20 + i % 81)
        };
        c.write(id, value)
            .unwrap_or_else(|e| panic!("cycle {i}: {e}"));
        assert_eq!(c.read(id).unwrap().current, value, "cycle {i}");
    }
    assert!(c
        .journal()
        .entries
        .iter()
        .all(|e| e.state == EntryState::Applied));
    assert!(c.restore(&baseline).iter().all(|(_, r)| r.is_ok()));
    assert_eq!(c.snapshot(), baseline);
}

#[test]
fn a_silent_monitor_is_not_rolled_back_and_settles_once_it_answers() {
    let mut m = MockMonitor::gm950b();
    m.faults.quiet_after_write.insert(0xF8, 10_000);
    let mut c = ctl(m, WritePolicy::Experimental);
    assert!(matches!(
        c.write("adaptive-sync", 0),
        Err(ControlError::Silent { .. })
    ));
    assert_eq!(
        c.transport.writes.len(),
        1,
        "no corrective write while silent"
    );
    assert_eq!(c.journal().entries[0].state, EntryState::Pending);
    // Still quiet: stays pending. Once it answers with the new value: applied.
    assert!(matches!(
        c.settle_silent("adaptive-sync", 0),
        Err(ControlError::Silent { .. })
    ));
    c.transport.faults.quiet_after_write.clear();
    c.transport.quiet.clear();
    assert!(c.settle_silent("adaptive-sync", 0).is_ok());
    assert_eq!(c.journal().entries[0].state, EntryState::Applied);
}

#[test]
fn black_stabilizer_is_written_in_fifths_and_read_whole() {
    // Measured on fw 3.05: write 10 reads 50, writes above 20 are refused.
    let mut c = ctl(MockMonitor::gm950b(), WritePolicy::Experimental);
    let a = c.write("black-stabilizer", 80).unwrap();
    assert_eq!(a.new, 80);
    assert_eq!(c.transport.writes, vec![(0xF9, 16)]);
    assert_eq!(c.read("black-stabilizer").unwrap().current, 80);
    // A value between steps lands on the nearest one.
    assert_eq!(c.write("black-stabilizer", 53).unwrap().new, 55);
    // Putting back the original goes through the same scale.
    c.revert(0).unwrap();
    assert_eq!(c.read("black-stabilizer").unwrap().current, 50);
}
