//! Talks to one monitor: paces DDC/CI traffic, retries flaky reads, and turns
//! every write into *read old → journal → write → read back → roll back if not taken*.

use std::collections::BTreeMap;
use std::thread::sleep;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::caps::{Capabilities, CapsError};
use crate::journal::{EntryState, Journal};
use crate::profile::{Confidence, Control, Profile, Risk, ValueKind, WritePolicy};
use crate::rules::{self, Availability, Notice};
use crate::safety::{self, SafetyError};
use crate::transport::{Transport, TransportError, VcpReading};

/// DDC/CI pacing. Monitors drop messages that arrive too quickly.
#[derive(Debug, Clone, Copy)]
pub struct Timing {
    /// Minimum gap between any two messages to the monitor.
    pub min_gap: Duration,
    /// Wait after a write before reading back.
    pub settle: Duration,
    /// Attempts per read before the value counts as unknown.
    pub read_attempts: u32,
    pub retry_delay: Duration,
    /// After a write that makes the panel resync (VRR, response time) the monitor
    /// can stop answering DDC/CI for a few seconds. When the read-back gets no
    /// answer at all, ask again up to this many times, `resync_gap` apart, before
    /// calling the write failed. An answer that shows the old value is a refusal
    /// and is not waited out.
    pub resync_polls: u32,
    pub resync_gap: Duration,
}

impl Timing {
    pub const HARDWARE: Timing = Timing {
        min_gap: Duration::from_millis(60),
        settle: Duration::from_millis(150),
        read_attempts: 3,
        retry_delay: Duration::from_millis(80),
        resync_polls: 10,
        resync_gap: Duration::from_millis(500),
    };

    pub const fn instant() -> Timing {
        Timing {
            min_gap: Duration::ZERO,
            settle: Duration::ZERO,
            read_attempts: 3,
            retry_delay: Duration::ZERO,
            resync_polls: 10,
            resync_gap: Duration::ZERO,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ControlError {
    #[error("unknown control '{0}' (see `nubless controls`)")]
    UnknownControl(String),
    #[error(transparent)]
    Safety(#[from] SafetyError),
    #[error(transparent)]
    Transport(#[from] TransportError),
    #[error("could not parse the monitor's capabilities: {0}")]
    Caps(#[from] CapsError),
    #[error("could not read the current value of '{0}', so the change could not be undone; nothing was written")]
    CannotReadCurrent(&'static str),
    #[error("the monitor did not apply '{id}' = {wanted} ({}); {outcome}. Another setting may be locking it (for example a picture mode or AI Scene Optimization).", describe_reading(.got))]
    NotApplied {
        id: &'static str,
        wanted: u16,
        got: Option<u16>,
        outcome: &'static str,
    },
    /// A write that resyncs the panel went out, then the monitor stopped
    /// answering for longer than we wait. It may well have applied; nothing was
    /// rolled back, because a blind write into a resyncing panel proves nothing.
    #[error("sent '{id}' = {wanted}, but the monitor did not answer afterwards (it may still be switching); nothing was rolled back")]
    Silent { id: &'static str, wanted: u16 },
    /// Another setting has to change first (see [`crate::rules`]). Nothing was sent.
    #[error("{}", .0.text())]
    Interlocked(Notice),
    /// A rule needs another setting's value and the monitor would not give it.
    /// Nothing was sent.
    #[error("could not check {depends_on} (the monitor did not answer), so '{id}' was not changed; try again in a moment")]
    CannotCheck {
        id: &'static str,
        depends_on: &'static str,
    },
    #[error("nothing to undo")]
    NothingToUndo,
    #[error("these settings belong to {saved}. Switch your monitor to {saved} first (it's on {current} now), then try again.")]
    WrongMode { saved: String, current: String },
}

/// What the monitor said when asked for the value after a write.
fn describe_reading(got: &Option<u16>) -> String {
    match got {
        Some(v) => format!("it reads {v}"),
        None => "it did not answer".to_string(),
    }
}

/// Result of a successful write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Applied {
    /// `None` when the value was already set and nothing was written.
    pub journal_index: Option<usize>,
    pub old: u16,
    pub new: u16,
    /// The caller must ask "Keep this change?" and call [`Controller::revert`]
    /// if the user does not answer in time.
    pub needs_confirmation: bool,
}

/// Values of every writable control, used as a restorable baseline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub model: String,
    pub values: BTreeMap<String, u16>,
    /// Raw value of the profile's mode control (picture mode) when the
    /// snapshot was taken. The values only belong to that mode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<u16>,
}

const CAPS_ATTEMPTS: u32 = 6;

pub struct Controller<T: Transport> {
    transport: T,
    profile: &'static Profile,
    caps: Capabilities,
    policy: WritePolicy,
    timing: Timing,
    journal: Journal,
    last_io: Option<Instant>,
}

impl<T: Transport> Controller<T> {
    pub fn new(
        transport: T,
        profile: &'static Profile,
        policy: WritePolicy,
        timing: Timing,
        journal: Journal,
    ) -> Result<Self, ControlError> {
        let mut c = Controller {
            transport,
            profile,
            caps: Capabilities::default(),
            policy,
            timing,
            journal,
            last_io: None,
        };
        // The capabilities string is the longest DDC/CI exchange and the one most
        // likely to collide with another client (LG's own service polls the
        // monitor). Retry harder than for single reads, including on garbled text.
        let mut last_err = None;
        for attempt in 0..CAPS_ATTEMPTS {
            if attempt > 0 {
                sleep(c.timing.retry_delay * 3);
            }
            c.pace();
            match c.transport.capabilities() {
                Ok(raw) => match Capabilities::parse(&raw) {
                    Ok(caps) => {
                        c.caps = caps;
                        return Ok(c);
                    }
                    Err(e) => last_err = Some(ControlError::Caps(e)),
                },
                Err(e) => last_err = Some(ControlError::Transport(e)),
            }
        }
        Err(last_err.expect("at least one attempt"))
    }

    pub fn profile(&self) -> &'static Profile {
        self.profile
    }
    pub fn capabilities(&self) -> &Capabilities {
        &self.caps
    }
    pub fn journal(&self) -> &Journal {
        &self.journal
    }
    pub fn policy(&self) -> WritePolicy {
        self.policy
    }
    pub fn set_policy(&mut self, policy: WritePolicy) {
        self.policy = policy;
    }

    pub fn control(&self, id: &str) -> Result<&'static Control, ControlError> {
        self.profile
            .control(id)
            .ok_or_else(|| ControlError::UnknownControl(id.to_string()))
    }

    /// Read a control. A failed read is an error, never "0". For a sub-command
    /// control the reading is the argument byte.
    pub fn read(&mut self, id: &str) -> Result<VcpReading, ControlError> {
        let c = self.control(id)?;
        self.read_control(c)
    }

    /// The value a change would be undone to. An answer of `0 / 0` is not a
    /// value: some LG firmware answers codes it does not implement that way, and
    /// "restoring" to that zero would be a made-up write.
    fn read_undoable(&mut self, c: &'static Control) -> Option<VcpReading> {
        let r = self.read_control(c).ok()?;
        (r.max != 0 || r.current != 0).then_some(r)
    }

    fn read_control(&mut self, c: &'static Control) -> Result<VcpReading, ControlError> {
        self.read_control_as(c, self.policy)
    }

    /// Read back after a write or undo. Uses the relaxed policy for the query
    /// so a change nubless made can always be verified and undone.
    fn verify_read(&mut self, c: &'static Control) -> Option<u16> {
        self.read_control_as(c, WritePolicy::Experimental)
            .ok()
            .map(|r| r.current)
    }

    fn read_control_as(
        &mut self,
        c: &'static Control,
        policy: WritePolicy,
    ) -> Result<VcpReading, ControlError> {
        let ValueKind::SubCommand { sub, .. } = c.kind else {
            return self.read_code(c.vcp);
        };
        // LG's read protocol: ask with (sub << 8) | 0xFF, then read the answer.
        let query = safety::check_query(c, policy)?;
        let mut last = None;
        for attempt in 0..self.timing.read_attempts.max(1) {
            if attempt > 0 {
                sleep(self.timing.retry_delay);
            }
            self.pace();
            if let Err(e) = self.transport.set_vcp(&query) {
                last = Some(e);
                continue;
            }
            self.settle();
            match self.read_code(c.vcp) {
                Ok(r) if (r.current >> 8) as u8 == sub && (r.current & 0xFF) != 0xFF => {
                    return Ok(VcpReading {
                        current: r.current & 0xFF,
                        max: 0xFE,
                    });
                }
                Ok(_) => last = Some(TransportError::NoReply(c.vcp)),
                Err(ControlError::Transport(e)) => last = Some(e),
                Err(e) => return Err(e),
            }
        }
        Err(last.expect("at least one attempt").into())
    }

    pub fn read_code(&mut self, code: u8) -> Result<VcpReading, ControlError> {
        Ok(self.with_retries(|t| t.get_vcp(code))?)
    }

    /// Change a control. See the module docs for the sequence.
    pub fn write(&mut self, id: &str, value: u16) -> Result<Applied, ControlError> {
        let control = self.control(id)?;
        let value = control.snap(value);
        // Validate first so a forbidden write costs no DDC traffic at all.
        let validated = safety::check_write(control, value, &self.caps, self.policy)?;
        let old = self
            .read_undoable(control)
            .ok_or(ControlError::CannotReadCurrent(control.id))?;
        let needs_confirmation = control.needs_confirm_or_revert();
        if old.current == value {
            return Ok(Applied {
                journal_index: None,
                old: old.current,
                new: value,
                needs_confirmation: false,
            });
        }
        self.check_interlock(control)?;
        let index = self
            .journal
            .record(control.id, control.vcp, old.current, value);
        self.apply(control, validated, old.current, index)?;
        Ok(Applied {
            journal_index: Some(index),
            old: old.current,
            new: value,
            needs_confirmation,
        })
    }

    /// Rules between settings (`rules.rs`), checked after the safety guard and
    /// before any write that changes something. A reading that fails refuses the
    /// write instead of guessing: the monitor ignores a locked write anyway, and
    /// sending one into a panel that is still resyncing proves nothing.
    fn check_interlock(&mut self, control: &Control) -> Result<(), ControlError> {
        if control.id != "motion-blur" {
            return Ok(());
        }
        let cannot = ControlError::CannotCheck {
            id: control.id,
            depends_on: "Adaptive-Sync",
        };
        let sync = self
            .profile
            .control("adaptive-sync")
            .ok_or_else(|| ControlError::UnknownControl("adaptive-sync".into()))?;
        // A 0/0 answer is a monitor that has nothing to say, not an Off.
        let reading = match self.read_code(sync.vcp) {
            Ok(r) if r.max != 0 || r.current != 0 => r,
            _ => return Err(cannot),
        };
        match rules::motion_blur_availability(Some(reading.current)) {
            Availability::Unavailable { reason } => Err(ControlError::Interlocked(reason)),
            Availability::Available => Ok(()),
        }
    }

    /// After [`ControlError::Silent`]: read the control again (the caller has
    /// reconnected by now) and settle the pending journal entry by what the
    /// monitor says. `Ok` means the change did land.
    pub fn settle_silent(&mut self, id: &str, wanted: u16) -> Result<Applied, ControlError> {
        let control = self.control(id)?;
        let index = self
            .journal
            .entries
            .iter()
            .rposition(|e| e.control == control.id && e.state == EntryState::Pending)
            .ok_or(ControlError::NothingToUndo)?;
        let old = self.journal.entries[index].old;
        match self.verify_read(control) {
            Some(v) if v == wanted => {
                self.journal.set_state(index, EntryState::Applied);
                Ok(Applied {
                    journal_index: Some(index),
                    old,
                    new: wanted,
                    needs_confirmation: control.needs_confirm_or_revert(),
                })
            }
            Some(v) => {
                self.journal.set_state(index, EntryState::RolledBack);
                Err(ControlError::NotApplied {
                    id: control.id,
                    wanted,
                    got: Some(v),
                    outcome: "the monitor kept its previous value",
                })
            }
            None => Err(ControlError::Silent {
                id: control.id,
                wanted,
            }),
        }
    }

    /// Undo one journal entry by writing back the value it replaced.
    pub fn revert(&mut self, index: usize) -> Result<(), ControlError> {
        let entry = self
            .journal
            .entries
            .get(index)
            .cloned()
            .ok_or(ControlError::NothingToUndo)?;
        let control = self
            .profile
            .controls
            .iter()
            .find(|c| c.id == entry.control)
            .ok_or_else(|| ControlError::UnknownControl(entry.control.clone()))?;
        let reading = self
            .read_control_as(control, WritePolicy::Experimental)
            .ok();
        let max = reading.map_or(u16::MAX, |r| r.max);
        if reading.map(|r| r.current) != Some(entry.old) {
            self.check_interlock(control)?;
        }
        let write = check_restore(control, entry.old, max, &self.caps)?;
        self.pace();
        self.transport.set_vcp(&write)?;
        self.settle();
        let back = self.verify_read(control);
        if back == Some(entry.old) {
            self.journal.set_state(index, EntryState::Reverted);
            Ok(())
        } else {
            Err(ControlError::NotApplied {
                id: control.id,
                wanted: entry.old,
                got: back,
                outcome: "undo did not take",
            })
        }
    }

    pub fn undo_last(&mut self) -> Result<(), ControlError> {
        let (index, _) = self
            .journal
            .last_undoable()
            .ok_or(ControlError::NothingToUndo)?;
        self.revert(index)
    }

    /// Read every writable control that answers.
    pub fn snapshot(&mut self) -> Snapshot {
        let mut values = BTreeMap::new();
        for c in self
            .profile
            .controls
            .iter()
            .filter(|c| c.writable && !matches!(c.kind, ValueKind::Info))
        {
            if let Ok(r) = self.read_control(c) {
                values.insert(c.id.to_string(), r.current);
            }
        }
        Snapshot {
            model: self.profile.model.to_string(),
            values,
            mode: self.current_mode(),
        }
    }

    /// Put back a snapshot's values. Uses the restore rules (any value the
    /// monitor itself reported is allowed back), so a baseline can always be
    /// restored even after the policy changed.
    pub fn restore(&mut self, snap: &Snapshot) -> Vec<(String, Result<(), ControlError>)> {
        if let (Some(saved), Some(id)) = (snap.mode, self.profile.mode_control) {
            let current = self.current_mode();
            if current != Some(saved) {
                let name = |v: Option<u16>| self.mode_name(v);
                let err = ControlError::WrongMode {
                    saved: name(Some(saved)),
                    current: name(current),
                };
                return vec![(id.to_string(), Err(err))];
            }
        }
        let mut results = Vec::new();
        // A setting that depends on another goes back after the one it depends on.
        let mut items: Vec<_> = snap.values.iter().collect();
        items.sort_by_key(|(id, _)| rules::restore_rank(id));
        for (id, &value) in items {
            let result = (|| {
                let control = self.control(id)?;
                let old = self
                    .read_undoable(control)
                    .ok_or(ControlError::CannotReadCurrent(control.id))?;
                if old.current == value {
                    return Ok(());
                }
                self.check_interlock(control)?;
                let write = check_restore(control, value, old.max, &self.caps)?;
                let index = self
                    .journal
                    .record(control.id, control.vcp, old.current, value);
                self.apply(control, write, old.current, index)
            })();
            results.push((id.clone(), result));
        }
        results
    }

    /// Raw value of the profile's mode control, if it has one and it answers.
    pub fn current_mode(&mut self) -> Option<u16> {
        let id = self.profile.mode_control?;
        self.read(id).ok().map(|r| r.current)
    }

    /// Human name of a mode value ("Gamer 1"), for messages.
    pub fn mode_name(&self, raw: Option<u16>) -> String {
        let label = self
            .profile
            .mode_control
            .and_then(|id| self.profile.control(id))
            .and_then(|c| raw.and_then(|r| c.choice_label(r)));
        match (label, raw) {
            (Some(l), _) => l.to_string(),
            (None, Some(r)) => format!("the picture mode you were using (value {r})"),
            (None, None) => "an unknown picture mode".to_string(),
        }
    }

    fn apply(
        &mut self,
        control: &'static Control,
        write: safety::ValidatedWrite,
        old: u16,
        index: usize,
    ) -> Result<(), ControlError> {
        self.pace();
        let sent = self.transport.set_vcp(&write);
        self.settle();
        let mut back = self.verify_read(control);
        // For a sub-command the monitor reports only the argument byte.
        let expected = match control.kind {
            ValueKind::SubCommand { .. } => write.value() & 0xFF,
            _ => write.value() * control.write_div(),
        };
        // A monitor that is still applying the change (response time and VRR
        // resync the panel) can answer with the old value once. Look again before
        // calling it refused, or a change that did land would be reported as undone.
        if sent.is_ok() && back != Some(expected) {
            self.settle();
            back = self.verify_read(control);
        }
        // No answer at all after a write that resyncs the panel: the monitor is
        // busy switching, not refusing. Give it time before concluding anything.
        if sent.is_ok() && back.is_none() && control.risk == Risk::Resync {
            for _ in 0..self.timing.resync_polls {
                sleep(self.timing.resync_gap);
                back = self.verify_read(control);
                if back.is_some() {
                    break;
                }
            }
        }
        if sent.is_ok() && back == Some(expected) {
            self.journal.set_state(index, EntryState::Applied);
            return Ok(());
        }
        // Still silent after the whole wait: leave the entry pending (so it stays
        // undoable) and let the caller reconnect and look again with `settle_silent`.
        if sent.is_ok() && back.is_none() && control.risk == Risk::Resync {
            return Err(ControlError::Silent {
                id: control.id,
                wanted: expected,
            });
        }
        // Not taken, or we cannot tell: put the old value back so the user is
        // never left in a state the app did not intend.
        let restored = if back == Some(old) {
            true
        } else {
            match check_restore(control, old, u16::MAX, &self.caps) {
                Ok(undo) => {
                    self.pace();
                    let _ = self.transport.set_vcp(&undo);
                    self.settle();
                    self.verify_read(control) == Some(old)
                }
                Err(_) => false,
            }
        };
        if restored {
            self.journal.set_state(index, EntryState::RolledBack);
        }
        if let Err(e) = sent {
            return Err(e.into());
        }
        Err(ControlError::NotApplied {
            id: control.id,
            wanted: expected,
            got: back,
            outcome: if restored {
                "the previous value was restored"
            } else {
                "the previous value could not be confirmed; run `nubless undo`"
            },
        })
    }

    fn with_retries<R>(
        &mut self,
        mut f: impl FnMut(&mut T) -> Result<R, TransportError>,
    ) -> Result<R, TransportError> {
        let mut last = None;
        for attempt in 0..self.timing.read_attempts.max(1) {
            if attempt > 0 {
                sleep(self.timing.retry_delay);
            }
            self.pace();
            match f(&mut self.transport) {
                Ok(v) => return Ok(v),
                Err(e) => last = Some(e),
            }
        }
        Err(last.expect("at least one attempt"))
    }

    fn pace(&mut self) {
        if let Some(t) = self.last_io {
            let since = t.elapsed();
            if since < self.timing.min_gap {
                sleep(self.timing.min_gap - since);
            }
        }
        self.last_io = Some(Instant::now());
    }

    fn settle(&mut self) {
        sleep(self.timing.settle);
    }
}

/// Restoring a value the monitor itself reported earlier: relaxes the value
/// table (the old value may lie outside our UI range) and the policy (an
/// experimental write must stay undoable), but never the deny-list, read-only,
/// dead-end or declaration checks.
fn check_restore(
    control: &'static Control,
    value: u16,
    reported_max: u16,
    caps: &Capabilities,
) -> Result<safety::ValidatedWrite, SafetyError> {
    let relaxed = WritePolicy::Experimental;
    match safety::check_write(control, value, caps, relaxed) {
        Ok(w) => Ok(w),
        Err(SafetyError::InvalidValue { .. })
            if value <= reported_max && control.confidence >= Confidence::Strong =>
        {
            // Same checks passed except the value table; re-run with an in-range
            // value to obtain the token, then substitute the reported value.
            let (probe, full) = match control.kind {
                ValueKind::Range { min, .. } => (min, value),
                ValueKind::Choice { choices } => (choices[0].raw, value),
                ValueKind::SubCommand { sub, choices } if value < 0xFF => {
                    (choices[0].raw, (u16::from(sub) << 8) | value)
                }
                _ => return Err(SafetyError::ReadOnly { id: control.id }),
            };
            let full = full / control.write_div();
            safety::check_write(control, probe, caps, relaxed).map(|w| w.with_value(full))
        }
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod fault_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::MockMonitor;
    use crate::profiles::gm950b::PROFILE;

    fn ctl(m: MockMonitor, policy: WritePolicy) -> Controller<MockMonitor> {
        Controller::new(m, &PROFILE, policy, Timing::instant(), Journal::in_memory()).unwrap()
    }

    #[test]
    fn write_reads_back_and_journals() {
        let mut c = ctl(MockMonitor::gm950b(), WritePolicy::Safe);
        let a = c.write("brightness", 40).unwrap();
        assert_eq!((a.old, a.new), (80, 40));
        assert!(!a.needs_confirmation);
        assert_eq!(c.read("brightness").unwrap().current, 40);
        assert_eq!(c.journal().entries[0].state, EntryState::Applied);
    }

    #[test]
    fn same_value_sends_nothing() {
        let mut c = ctl(MockMonitor::gm950b(), WritePolicy::Safe);
        let a = c.write("brightness", 80).unwrap();
        assert_eq!(a.journal_index, None);
        assert!(c.transport.writes.is_empty());
    }

    #[test]
    fn refused_write_is_rolled_back() {
        let mut m = MockMonitor::gm950b();
        m.ignore_writes_to.push(0x12);
        let mut c = ctl(m, WritePolicy::Safe);
        let err = c.write("contrast", 30).unwrap_err();
        assert!(matches!(err, ControlError::NotApplied { .. }), "{err}");
        assert_eq!(c.read("contrast").unwrap().current, 70);
        assert_eq!(c.journal().entries[0].state, EntryState::RolledBack);
    }

    #[test]
    fn undo_restores_previous_value() {
        let mut c = ctl(MockMonitor::gm950b(), WritePolicy::Safe);
        c.write("brightness", 10).unwrap();
        c.write("brightness", 20).unwrap();
        c.undo_last().unwrap();
        assert_eq!(c.read("brightness").unwrap().current, 10);
        c.undo_last().unwrap();
        assert_eq!(c.read("brightness").unwrap().current, 80);
        assert!(matches!(c.undo_last(), Err(ControlError::NothingToUndo)));
    }

    #[test]
    fn experimental_write_stays_undoable_after_policy_changes() {
        let mut c = ctl(MockMonitor::gm950b(), WritePolicy::Experimental);
        let a = c.write("adaptive-sync", 0).unwrap();
        assert!(a.needs_confirmation, "VRR changes resync the panel");
        c.policy = WritePolicy::Safe;
        c.undo_last().unwrap();
        assert_eq!(c.read("adaptive-sync").unwrap().current, 3);
    }

    fn snapshot_of(values: &[(&str, u16)]) -> Snapshot {
        Snapshot {
            model: PROFILE.model.to_string(),
            values: values.iter().map(|&(k, v)| (k.to_string(), v)).collect(),
            mode: None,
        }
    }

    #[test]
    fn restore_puts_adaptive_sync_back_before_motion_blur() {
        // Alphabetical order would also work here; the rule must not depend on it.
        let mut c = ctl(MockMonitor::gm950b(), WritePolicy::Safe);
        let results = c.restore(&snapshot_of(&[("motion-blur", 1), ("adaptive-sync", 0)]));
        assert!(results.iter().all(|(_, r)| r.is_ok()), "{results:?}");
        assert_eq!(c.read("adaptive-sync").unwrap().current, 0);
        assert_eq!(c.read("motion-blur").unwrap().current, 1);
        assert_eq!(crate::rules::restore_rank("adaptive-sync"), 0);
        assert_eq!(crate::rules::restore_rank("motion-blur"), 1);
    }

    #[test]
    fn restore_explains_motion_blur_when_adaptive_sync_would_not_go_back() {
        let mut m = MockMonitor::gm950b();
        m.ignore_writes_to.push(0xF8);
        let mut c = ctl(m, WritePolicy::Safe);
        let results = c.restore(&snapshot_of(&[("adaptive-sync", 0), ("motion-blur", 1)]));
        let blur = results.iter().find(|(id, _)| id == "motion-blur").unwrap();
        assert!(
            matches!(blur.1, Err(ControlError::Interlocked(_))),
            "{results:?}"
        );
        assert!(
            !c.transport.writes.iter().any(|&(code, _)| code == 0xFA),
            "no write into a locked control"
        );
    }

    #[test]
    fn undo_of_motion_blur_is_refused_while_adaptive_sync_is_on() {
        let mut c = ctl(MockMonitor::gm950b(), WritePolicy::Safe);
        c.write("adaptive-sync", 0).unwrap();
        c.write("motion-blur", 1).unwrap();
        let off = c.write("motion-blur", 2).unwrap().journal_index.unwrap();
        c.write("adaptive-sync", 3).unwrap();
        let err = c.revert(off).unwrap_err();
        assert!(matches!(err, ControlError::Interlocked(_)), "{err}");
        assert_eq!(c.read("motion-blur").unwrap().current, 2);
    }

    #[test]
    fn undo_that_changes_nothing_is_not_blocked_by_the_lock() {
        let mut c = ctl(MockMonitor::gm950b(), WritePolicy::Safe);
        c.write("adaptive-sync", 0).unwrap();
        let on = c.write("motion-blur", 1).unwrap().journal_index.unwrap();
        // Adaptive-Sync coming on cleared it; the entry's old value (Off) is already true.
        c.write("adaptive-sync", 3).unwrap();
        assert_eq!(c.read("motion-blur").unwrap().current, 2);
        c.revert(on).unwrap();
    }

    #[test]
    fn a_monitor_that_cannot_tell_us_adaptive_sync_gets_no_motion_blur_write() {
        for unknown in [
            None,
            Some(crate::transport::VcpReading { current: 0, max: 0 }),
        ] {
            let mut m = MockMonitor::gm950b();
            match unknown {
                None => {
                    m.values.remove(&0xF8);
                }
                Some(r) => {
                    m.values.insert(0xF8, r);
                }
            }
            let mut c = ctl(m, WritePolicy::Safe);
            let err = c.write("motion-blur", 1).unwrap_err();
            assert!(matches!(err, ControlError::CannotCheck { .. }), "{err}");
            assert!(
                err.to_string().contains("could not check Adaptive-Sync"),
                "{err}"
            );
            assert!(c.transport.writes.is_empty(), "nothing was sent");
        }
    }

    #[test]
    fn motion_blur_already_where_you_asked_is_not_told_off() {
        let mut c = ctl(MockMonitor::gm950b(), WritePolicy::Safe);
        // Adaptive-Sync is on and Motion Blur reads Off: asking for Off is a no-op.
        let a = c.write("motion-blur", 2).unwrap();
        assert_eq!(a.journal_index, None);
        assert!(c.transport.writes.is_empty());
    }

    #[test]
    fn motion_blur_waits_for_adaptive_sync_off() {
        let mut c = ctl(MockMonitor::gm950b(), WritePolicy::Safe);
        // The mock starts on Adaptive-Sync Extended (3).
        let err = c.write("motion-blur", 1).unwrap_err();
        assert!(matches!(err, ControlError::Interlocked(_)), "{err}");
        assert!(
            err.to_string().contains("Turn Adaptive-Sync off first"),
            "{err}"
        );
        assert!(
            c.transport.writes.is_empty(),
            "a locked write sends nothing"
        );
        c.write("adaptive-sync", 0).unwrap();
        c.write("motion-blur", 1).unwrap();
        assert_eq!(c.read("motion-blur").unwrap().current, 1);
    }

    #[test]
    fn unreadable_current_value_blocks_the_write() {
        let mut m = MockMonitor::gm950b();
        m.values.remove(&0x10);
        let mut c = ctl(m, WritePolicy::Safe);
        assert!(matches!(
            c.write("brightness", 50),
            Err(ControlError::CannotReadCurrent(_))
        ));
        assert!(c.transport.writes.is_empty());
    }

    #[test]
    fn flaky_reads_are_retried() {
        let mut m = MockMonitor::gm950b();
        m.flaky_every = 2;
        let mut c = ctl(m, WritePolicy::Safe);
        for _ in 0..5 {
            assert_eq!(c.read("brightness").unwrap().current, 80);
        }
    }

    #[test]
    fn snapshot_restore_round_trip_including_out_of_range_values() {
        let mut m = MockMonitor::gm950b();
        // Contrast's safe range starts at 20; a lower value the monitor itself
        // reported must still be restorable, or the user would need the OSD to get it back.
        m.values.get_mut(&0x12).unwrap().current = 10;
        let mut c = ctl(m, WritePolicy::Experimental);
        let snap = c.snapshot();
        assert_eq!(snap.values["contrast"], 10);
        c.write("contrast", 60).unwrap();
        c.write("brightness", 5).unwrap();
        let results = c.restore(&snap);
        assert!(results.iter().all(|(_, r)| r.is_ok()), "{results:?}");
        assert_eq!(c.read("contrast").unwrap().current, 10);
        assert_eq!(c.read("brightness").unwrap().current, 80);
    }

    #[test]
    fn forbidden_write_sends_no_traffic() {
        let mut c = ctl(MockMonitor::gm950b(), WritePolicy::Experimental);
        assert!(c.write("input", 0x11).is_err());
        assert!(c.write("picture-mode", 39).is_err());
        assert!(c.transport.writes.is_empty());
    }
}

#[cfg(test)]
mod ai_channel_tests {
    use super::*;
    use crate::mock::MockMonitor;
    use crate::profiles::gm950b::PROFILE;

    fn ctl(policy: WritePolicy) -> Controller<MockMonitor> {
        Controller::new(
            MockMonitor::gm950b(),
            &PROFILE,
            policy,
            Timing::instant(),
            Journal::in_memory(),
        )
        .unwrap()
    }

    #[test]
    fn unconfirmed_ai_sub_commands_need_labs_to_read() {
        // Nothing on 0xA6 is unconfirmed any more, so use a Candidate copy.
        let candidate: &'static Control = Box::leak(Box::new(Control {
            confidence: crate::profile::Confidence::Candidate,
            ..*PROFILE.control("ai-upscaling").unwrap()
        }));
        let mut c = ctl(WritePolicy::Safe);
        assert!(matches!(
            c.read_control_as(candidate, WritePolicy::Safe),
            Err(ControlError::Safety(SafetyError::NotConfident { .. }))
        ));
        assert!(
            c.transport.writes.is_empty(),
            "not even a query is sent without Labs"
        );
        assert_eq!(c.read("ai-scene-optimization").unwrap().current, 2);
        c.write("ai-sound", 2).unwrap();
    }

    #[test]
    fn ai_scene_optimization_uses_lg_query_then_set_protocol() {
        let mut c = ctl(WritePolicy::Experimental);
        assert_eq!(c.read("ai-scene-optimization").unwrap().current, 2);
        assert_eq!(c.transport.writes, vec![(0xA6, 0x02FF)]);
        c.write("ai-scene-optimization", 1).unwrap();
        assert_eq!(c.transport.ai[&2], 1);
        assert!(c.transport.writes.contains(&(0xA6, 0x0201)));
        c.undo_last().unwrap();
        assert_eq!(c.transport.ai[&2], 2);
    }

    #[test]
    fn ai_sound_round_trips() {
        let mut c = ctl(WritePolicy::Experimental);
        c.write("ai-sound", 2).unwrap();
        assert_eq!(c.read("ai-sound").unwrap().current, 2);
        assert!(c.write("ai-sound", 3).is_err(), "only off/on");
    }

    #[test]
    fn upscaling_writes_its_own_sub_command_without_labs() {
        let mut c = ctl(WritePolicy::Safe);
        let scene = c.transport.ai[&2];
        assert_eq!(c.read("ai-upscaling").unwrap().current, 1);
        for value in [2, 4, 1] {
            c.write("ai-upscaling", value).unwrap();
            assert_eq!(c.read("ai-upscaling").unwrap().current, value);
        }
        assert!(c.write("ai-upscaling", 3).is_err(), "3 is not an option");
        assert!(c.transport.writes.contains(&(0xA6, 0x0404)));
        assert_eq!(
            c.transport.ai[&2], scene,
            "AI Scene Optimization is untouched"
        );
    }

    #[test]
    fn refused_ai_change_is_rolled_back() {
        let mut m = MockMonitor::gm950b();
        m.ignore_writes_to.push(0xA6);
        let mut c = Controller::new(
            m,
            &PROFILE,
            WritePolicy::Experimental,
            Timing::instant(),
            Journal::in_memory(),
        )
        .unwrap();
        assert!(matches!(
            c.write("ai-scene-optimization", 1),
            Err(ControlError::NotApplied { .. })
        ));
        assert_eq!(c.transport.ai[&2], 2);
    }

    #[test]
    fn unmapped_sub_commands_stay_denied() {
        static PC_AI: Control = Control {
            id: "pc-ai",
            label: "PC AI",
            vcp: 0xA6,
            kind: ValueKind::SubCommand {
                sub: 3,
                choices: &[crate::profile::Choice {
                    raw: 2,
                    label: "On",
                }],
            },
            confidence: Confidence::WriteConfirmed,
            standard: false,
            writable: true,
            risk: crate::profile::Risk::None,
            note: "",
        };
        let caps = Capabilities::parse(crate::profiles::gm950b::CAPABILITIES_SAMPLE).unwrap();
        assert!(matches!(
            safety::check_write(&PC_AI, 2, &caps, WritePolicy::Experimental),
            Err(SafetyError::Denied { .. })
        ));
        assert!(safety::check_query(&PC_AI, WritePolicy::Experimental).is_err());
    }
}
