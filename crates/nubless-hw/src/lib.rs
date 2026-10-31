//! Real DDC/CI access through `ddc-hi`: Windows `dxva2`, Linux `/dev/i2c-*`,
//! macOS IOKit. Contains no policy; everything goes through `nubless-core`.
//!
//! Besides the DDC/CI handle, each display carries what the OS knows about the
//! physical monitor: its [`Identity`] (EDID serial and port), so each monitor
//! keeps its own original settings, and the name the OS shows for it.

use ddc_hi::{Ddc, Display};
use nubless_core::caps::Capabilities;
use nubless_core::monitors::Identity;
use nubless_core::profile::{Match, Profile};
use nubless_core::safety::ValidatedWrite;
use nubless_core::transport::{Transport, TransportError, VcpReading};

mod os;

/// What the OS says about a monitor, beyond its DDC/CI handle.
#[derive(Debug, Clone, Default)]
pub struct OsInfo {
    /// Where it is plugged in, stable across reboots (see [`Identity::port`]).
    pub port: String,
    /// The OS's short name for that place: `DP-1` (Linux), `DISPLAY2` (Windows).
    pub port_name: Option<String>,
    /// macOS `CGDirectDisplayID`, to look up the name System Settings shows.
    pub display_id: Option<u32>,
    pub serial_number: Option<String>,
    pub serial: Option<u32>,
    /// The monitor's own name from its EDID, as the OS shows it ("LG ULTRAGEAR+").
    pub name: Option<String>,
}

/// One connected display and what we know about it.
pub struct HwDisplay {
    display: Display,
    pub index: usize,
    pub backend: String,
    pub description: String,
    pub matched: Match,
    pub os: OsInfo,
}

impl HwDisplay {
    /// Which physical monitor this is (see [`nubless_core::monitors`]).
    pub fn identity(&self) -> Identity {
        Identity::new(
            self.os.serial_number.as_deref(),
            self.os.serial,
            self.os.port.clone(),
        )
    }

    pub fn summary(&self) -> String {
        let m = &self.display.info;
        format!(
            "#{} [{}] {}{}{}{}",
            self.index,
            self.backend,
            self.description,
            self.os
                .name
                .as_deref()
                .or(m.model_name.as_deref())
                .map(|n| format!(" \"{n}\""))
                .unwrap_or_default(),
            self.os
                .port_name
                .as_deref()
                .map(|p| format!(" on {p}"))
                .unwrap_or_default(),
            m.model_id
                .map(|p| format!(" product 0x{p:04X}"))
                .unwrap_or_default(),
        )
    }
}

/// Read the capabilities string and match it against `profile`. The
/// capabilities read is the flakiest DDC/CI message; give it a few tries.
fn identify(display: &mut Display, profile: &Profile) -> Match {
    let caps = (0..5).find_map(|attempt| {
        if attempt > 0 {
            std::thread::sleep(std::time::Duration::from_millis(300 * attempt as u64));
        }
        let raw = display.handle.capabilities_string().ok()?;
        Capabilities::parse(&String::from_utf8_lossy(&raw)).ok()
    });
    let info = &display.info;
    profile.identity.matches(
        info.manufacturer_id.as_deref(),
        info.model_id,
        info.model_name.as_deref(),
        caps.as_ref(),
    )
}

fn wrap(index: usize, display: Display, os: OsInfo, matched: Match) -> HwDisplay {
    HwDisplay {
        index,
        backend: display.info.backend.to_string(),
        description: display.info.id.clone(),
        matched,
        os,
        display,
    }
}

/// Enumerate displays and match each against `profile`. Reads each display's
/// capabilities string (about a second per display), read-only.
pub fn enumerate(profile: &Profile) -> Vec<HwDisplay> {
    os::displays()
        .into_iter()
        .enumerate()
        .map(|(index, (mut display, os))| {
            let matched = identify(&mut display, profile);
            wrap(index, display, os, matched)
        })
        .collect()
}

/// Find one monitor again (after it went quiet, or the OS re-detected it),
/// reading capabilities only from the display that is it, not from every one.
/// `None` if it isn't there (yet), or no longer answers as a supported model.
pub fn reopen(profile: &Profile, id: &Identity) -> Option<HwDisplay> {
    let candidates: Vec<_> = os::displays().into_iter().enumerate().collect();
    let identity =
        |os: &OsInfo| Identity::new(os.serial_number.as_deref(), os.serial, os.port.clone());
    // The serial when both sides know it; else the port.
    let same = |os: &OsInfo| {
        let other = identity(os);
        match (&id.serial, &other.serial) {
            (Some(a), Some(b)) => a == b,
            _ => id.port == other.port,
        }
    };
    let (index, (mut display, os)) = candidates.into_iter().find(|(_, (_, os))| same(os))?;
    let matched = identify(&mut display, profile);
    (matched != Match::No).then(|| wrap(index, display, os, matched))
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
