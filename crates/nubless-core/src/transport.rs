//! The boundary to the hardware. Implemented by `nubless-hw` (real DDC/CI) and
//! [`crate::mock::MockMonitor`] (tests, `--mock`).

use crate::safety::{ValidatedInputSwitch, ValidatedWrite};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VcpReading {
    pub current: u16,
    pub max: u16,
}

#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("no reply from the monitor for VCP 0x{0:02X}")]
    NoReply(u8),
    #[error("DDC/CI error: {0}")]
    Io(String),
    #[error("{0}")]
    Unsupported(&'static str),
}

/// Why [`Transport::switch_input`] isn't available, when a transport has no way
/// to send LG's side-channel write.
pub const NO_SIDE_CHANNEL: &str =
    "switching inputs isn't supported on this platform yet (only on Apple Silicon Macs so far)";

pub trait Transport {
    /// Raw MCCS capabilities string.
    fn capabilities(&mut self) -> Result<String, TransportError>;
    fn get_vcp(&mut self, code: u8) -> Result<VcpReading, TransportError>;
    /// Only accepts writes that went through [`crate::safety::check_write`].
    fn set_vcp(&mut self, write: &ValidatedWrite) -> Result<(), TransportError>;
    /// Whether [`Transport::switch_input`] can be sent at all on this link.
    fn can_switch_input(&self) -> bool {
        false
    }
    /// Send an input switch over LG's side channel. Only accepts switches that
    /// went through [`crate::safety::check_input_switch`]. `Ok` means the write
    /// went out; the monitor never confirms that it switched.
    fn switch_input(&mut self, _switch: &ValidatedInputSwitch) -> Result<(), TransportError> {
        Err(TransportError::Unsupported(NO_SIDE_CHANNEL))
    }
}

impl<T: Transport + ?Sized> Transport for Box<T> {
    fn capabilities(&mut self) -> Result<String, TransportError> {
        (**self).capabilities()
    }
    fn get_vcp(&mut self, code: u8) -> Result<VcpReading, TransportError> {
        (**self).get_vcp(code)
    }
    fn set_vcp(&mut self, write: &ValidatedWrite) -> Result<(), TransportError> {
        (**self).set_vcp(write)
    }
    fn can_switch_input(&self) -> bool {
        (**self).can_switch_input()
    }
    fn switch_input(&mut self, switch: &ValidatedInputSwitch) -> Result<(), TransportError> {
        (**self).switch_input(switch)
    }
}
