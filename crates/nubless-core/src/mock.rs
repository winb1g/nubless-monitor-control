//! A simulated 27GM950B, seeded with the values read from a real unit.
//! Used by tests and by `nubless --mock` so the UI can be developed without
//! touching a monitor.

use std::collections::BTreeMap;

use crate::profiles::gm950b::CAPABILITIES_SAMPLE;
use crate::safety::ValidatedWrite;
use crate::transport::{Transport, TransportError, VcpReading};

/// Scripted misbehaviour for tests, modelled on what real DDC/CI links do:
/// dropped replies, all-zero answers for codes the firmware does not implement
/// (seen on LG monitors), writes that are lost or acknowledged but not applied,
/// read-backs that lag behind a write, and a link that goes away.
#[derive(Debug, Clone, Default)]
pub struct Faults {
    /// The next N reads of a code get no reply.
    pub drop_reads: BTreeMap<u8, u32>,
    /// Reads of these codes answer `0 / 0` instead of failing.
    pub zero_reply: Vec<u8>,
    /// The next N writes (any code) fail on the wire; nothing is applied.
    pub fail_writes: u32,
    /// The next N writes are acknowledged but not applied.
    pub swallow_writes: u32,
    /// After every write to a code, its next N reads still show the old value
    /// while the new one has already been applied.
    pub stale_reads: BTreeMap<u8, u32>,
    /// After every write to a code, its next N reads get no reply at all: the
    /// monitor is busy resyncing the panel (turning VRR off does this for seconds).
    pub quiet_after_write: BTreeMap<u8, u32>,
    /// The next N capabilities requests fail.
    pub caps_fail: u32,
    /// The next N capabilities requests return cut-off text.
    pub caps_garbled: u32,
    /// Every request fails (cable pulled, DDC/CI switched off in the OSD).
    pub offline: bool,
}

#[derive(Debug, Clone)]
pub struct MockMonitor {
    pub values: BTreeMap<u8, VcpReading>,
    pub caps: String,
    /// Codes whose writes are silently ignored (the monitor "refuses" them, as a
    /// real one does when a mode locks a setting). Exercises the rollback path.
    pub ignore_writes_to: Vec<u8>,
    /// Fail every Nth read (0 = never), to exercise retries.
    pub flaky_every: u32,
    pub faults: Faults,
    /// Old readings still being served after a write (see [`Faults::stale_reads`]).
    stale: BTreeMap<u8, (u32, VcpReading)>,
    pub(crate) quiet: BTreeMap<u8, u32>,
    reads: u32,
    pub writes: Vec<(u8, u16)>,
    /// LG AI channel state per sub-command (0xA6): 2 = AI Scene Optimization,
    /// 4 = AI Upscaling, 6 = AI Sound; 1 = off, 2 = on. Seeded from a real unit.
    pub ai: BTreeMap<u8, u8>,
}

impl Default for MockMonitor {
    fn default() -> Self {
        Self::gm950b()
    }
}

impl MockMonitor {
    pub fn gm950b() -> Self {
        let v = |current, max| VcpReading { current, max };
        let values = BTreeMap::from([
            (0x10, v(80, 100)),
            (0x12, v(70, 100)),
            (0x14, v(11, 0xFFFF)),
            (0x15, v(65, 255)),
            (0x16, v(50, 100)),
            (0x18, v(50, 100)),
            (0x1A, v(50, 100)),
            (0x4D, v(0x8002, 0xFFFF)),
            (0x4E, v(0x420C, 0xFFFF)),
            (0x4F, v(0x9B82, 0xFFFF)),
            (0x60, v(0x0F, 18)),
            (0x62, v(20, 100)),
            (0x69, v(6500, 0xFFFF)),
            (0x8D, v(2, 2)),
            (0xB1, v(0x2200, 0xFFFF)),
            (0xC0, v(1000, 0xFFFF)), // placeholder, not a real unit's hours
            (0xC9, v(0x0305, 0xFFFF)),
            (0xE2, v(1, 255)),
            (0xF5, v(1, 255)),
            (0xF7, v(2, 255)),
            (0xF8, v(3, 255)),
            (0xF9, v(50, 255)),
            (0xFA, v(2, 255)),
        ]);
        MockMonitor {
            values,
            caps: CAPABILITIES_SAMPLE.to_string(),
            ignore_writes_to: Vec::new(),
            flaky_every: 0,
            faults: Faults::default(),
            stale: BTreeMap::new(),
            quiet: BTreeMap::new(),
            reads: 0,
            writes: Vec::new(),
            ai: BTreeMap::from([(2, 2), (4, 1), (6, 1)]),
        }
    }
}

impl Transport for MockMonitor {
    fn capabilities(&mut self) -> Result<String, TransportError> {
        if self.faults.offline {
            return Err(link_down());
        }
        if take(&mut self.faults.caps_fail) {
            return Err(TransportError::Io("capabilities request timed out".into()));
        }
        if take(&mut self.faults.caps_garbled) {
            return Ok(self.caps[..self.caps.len() / 2].to_string());
        }
        Ok(self.caps.clone())
    }

    fn get_vcp(&mut self, code: u8) -> Result<VcpReading, TransportError> {
        if self.faults.offline {
            return Err(link_down());
        }
        if let Some(n) = self.faults.drop_reads.get_mut(&code).filter(|n| **n > 0) {
            *n -= 1;
            return Err(TransportError::NoReply(code));
        }
        if let Some(n) = self.quiet.get_mut(&code).filter(|n| **n > 0) {
            *n -= 1;
            return Err(TransportError::NoReply(code));
        }
        if self.faults.zero_reply.contains(&code) {
            return Ok(VcpReading { current: 0, max: 0 });
        }
        if let Some((left, old)) = self.stale.get_mut(&code).filter(|(n, _)| *n > 0) {
            *left -= 1;
            return Ok(*old);
        }
        self.reads += 1;
        if self.flaky_every != 0 && self.reads % self.flaky_every == 0 {
            return Err(TransportError::NoReply(code));
        }
        self.values
            .get(&code)
            .copied()
            .ok_or(TransportError::NoReply(code))
    }

    fn set_vcp(&mut self, write: &ValidatedWrite) -> Result<(), TransportError> {
        self.writes.push((write.code(), write.value()));
        if self.faults.offline {
            return Err(link_down());
        }
        if take(&mut self.faults.fail_writes) {
            return Err(TransportError::Io("write was not acknowledged".into()));
        }
        if take(&mut self.faults.swallow_writes) {
            return Ok(());
        }
        if let Some(&n) = self.faults.quiet_after_write.get(&write.code()) {
            self.quiet.insert(write.code(), n);
        }
        if let (Some(&n), Some(old)) = (
            self.faults.stale_reads.get(&write.code()),
            self.values.get(&write.code()),
        ) {
            self.stale.insert(write.code(), (n, *old));
        }
        if write.code() == crate::safety::AI_CHANNEL {
            // LG's AI channel: (sub << 8) | 0xFF asks for a value, anything else sets it.
            let (sub, arg) = ((write.value() >> 8) as u8, (write.value() & 0xFF) as u8);
            if arg != crate::safety::SUB_QUERY && !self.ignore_writes_to.contains(&write.code()) {
                self.ai.insert(sub, arg);
            }
            let answer = self.ai.get(&sub).copied().unwrap_or(0xFF);
            self.values.insert(
                write.code(),
                VcpReading {
                    current: (u16::from(sub) << 8) | u16::from(answer),
                    max: 0xFFFF,
                },
            );
            return Ok(());
        }
        if self.ignore_writes_to.contains(&write.code()) {
            return Ok(());
        }
        let entry = self.values.entry(write.code()).or_insert(VcpReading {
            current: 0,
            max: 0xFFFF,
        });
        // Like the real unit, Black Stabilizer is written in fifths and read back whole;
        // it refuses anything past 20.
        if write.code() == 0xF9 {
            if write.value() > 20 {
                return Ok(());
            }
            entry.current = write.value() * 5;
            return Ok(());
        }
        entry.current = write.value();
        Ok(())
    }
}

fn link_down() -> TransportError {
    TransportError::Io("no DDC/CI link to the monitor".into())
}

/// Counts a fault down; true while there are faults left to deliver.
fn take(n: &mut u32) -> bool {
    let fire = *n > 0;
    *n = n.saturating_sub(1);
    fire
}
