//! The boundary to the hardware. Implemented by `nubless-hw` (real DDC/CI) and
//! [`crate::mock::MockMonitor`] (tests, `--mock`).

use crate::safety::ValidatedWrite;

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
}

pub trait Transport {
    /// Raw MCCS capabilities string.
    fn capabilities(&mut self) -> Result<String, TransportError>;
    fn get_vcp(&mut self, code: u8) -> Result<VcpReading, TransportError>;
    /// Only accepts writes that went through [`crate::safety::check_write`].
    fn set_vcp(&mut self, write: &ValidatedWrite) -> Result<(), TransportError>;
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
}
