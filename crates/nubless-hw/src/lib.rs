//! Real DDC/CI access through `ddc-hi`: Windows `dxva2`, Linux `/dev/i2c-*`,
//! macOS IOKit. Contains no policy; everything goes through `nubless-core`.

use ddc_hi::{Ddc, Display};
use nubless_core::caps::Capabilities;
use nubless_core::profile::{Match, Profile};
use nubless_core::safety::ValidatedWrite;
use nubless_core::transport::{Transport, TransportError, VcpReading};

/// One connected display and what we know about it.
pub struct HwDisplay {
    display: Display,
    pub index: usize,
    pub backend: String,
    pub description: String,
    pub matched: Match,
}

impl HwDisplay {
    pub fn summary(&self) -> String {
        let m = &self.display.info;
        format!(
            "#{} [{}] {}{}{}",
            self.index,
            self.backend,
            self.description,
            m.model_name
                .as_deref()
                .map(|n| format!(" \"{n}\""))
                .unwrap_or_default(),
            m.model_id
                .map(|p| format!(" product 0x{p:04X}"))
                .unwrap_or_default(),
        )
    }
}

/// Enumerate displays and match each against `profile`. Reads each display's
/// capabilities string (about a second per display), read-only.
pub fn enumerate(profile: &Profile) -> Vec<HwDisplay> {
    Display::enumerate()
        .into_iter()
        .enumerate()
        .map(|(index, mut display)| {
            // The capabilities read is the flakiest DDC/CI message; give it a few tries.
            let caps = (0..5).find_map(|attempt| {
                if attempt > 0 {
                    std::thread::sleep(std::time::Duration::from_millis(300 * attempt as u64));
                }
                let raw = display.handle.capabilities_string().ok()?;
                Capabilities::parse(&String::from_utf8_lossy(&raw)).ok()
            });
            let info = &display.info;
            let matched = profile.identity.matches(
                info.manufacturer_id.as_deref(),
                info.model_id,
                info.model_name.as_deref(),
                caps.as_ref(),
            );
            HwDisplay {
                index,
                backend: info.backend.to_string(),
                description: info.id.clone(),
                matched,
                display,
            }
        })
        .collect()
}

fn io(e: impl std::fmt::Display) -> TransportError {
    TransportError::Io(e.to_string())
}

impl Transport for HwDisplay {
    fn capabilities(&mut self) -> Result<String, TransportError> {
        let raw = self.display.handle.capabilities_string().map_err(io)?;
        Ok(String::from_utf8_lossy(&raw).into_owned())
    }

    fn get_vcp(&mut self, code: u8) -> Result<VcpReading, TransportError> {
        let v = self
            .display
            .handle
            .get_vcp_feature(code)
            .map_err(|_| TransportError::NoReply(code))?;
        Ok(VcpReading {
            current: v.value(),
            max: v.maximum(),
        })
    }

    fn set_vcp(&mut self, write: &ValidatedWrite) -> Result<(), TransportError> {
        self.display
            .handle
            .set_vcp_feature(write.code(), write.value())
            .map_err(io)
    }
}
