//! A simulated 27GM950B, seeded with the values read from a real unit.
//! Used by tests and by `nubless --mock` so the UI can be developed without
//! touching a monitor.

use std::collections::BTreeMap;

use crate::profiles::gm950b::CAPABILITIES_SAMPLE;
use crate::safety::ValidatedWrite;
use crate::transport::{Transport, TransportError, VcpReading};

#[derive(Debug, Clone)]
pub struct MockMonitor {
    pub values: BTreeMap<u8, VcpReading>,
    pub caps: String,
    /// Codes whose writes are silently ignored (the monitor "refuses" them, as a
    /// real one does when a mode locks a setting). Exercises the rollback path.
    pub ignore_writes_to: Vec<u8>,
    /// Fail every Nth read (0 = never), to exercise retries.
    pub flaky_every: u32,
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
            reads: 0,
            writes: Vec::new(),
            ai: BTreeMap::from([(2, 2), (4, 1), (6, 1)]),
        }
    }
}

impl Transport for MockMonitor {
    fn capabilities(&mut self) -> Result<String, TransportError> {
        Ok(self.caps.clone())
    }

    fn get_vcp(&mut self, code: u8) -> Result<VcpReading, TransportError> {
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
        entry.current = write.value();
        Ok(())
    }
}
