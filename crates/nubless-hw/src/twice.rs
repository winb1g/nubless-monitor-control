//! Send each DDC/CI request twice.
//!
//! On Apple Silicon DisplayPort outputs (`DCPDP2XService`, seen on an M5 Max)
//! the monitor ignores the first write of a request, and the read after it
//! returns the previous reply with its length and opcode bytes overwritten
//! (`6e 80 be ...` where `6e 88 02 ...` is due), which the `ddc` crate rejects
//! as "invalid DDC/CI length". The second write gets the real reply.
//! MonitorControl and m1ddc write every request twice for the same reason; HDMI
//! outputs answer either way.
#![cfg_attr(not(target_os = "macos"), allow(dead_code))]

use std::time::Duration;

use ddc::{DdcCommandMarker, DdcCommandRaw, DdcCommandRawMarker, DdcHost, Delay, ErrorCode};

/// Pause between the two writes, as m1ddc does.
const BETWEEN_WRITES: Duration = Duration::from_millis(10);

pub(crate) struct Twice<'a, D>(pub &'a mut D);

impl<D: DdcHost> DdcHost for Twice<'_, D> {
    type Error = D::Error;

    fn sleep(&mut self) {
        self.0.sleep()
    }
}

impl<D: DdcCommandRaw> DdcCommandRaw for Twice<'_, D> {
    fn execute_raw<'a>(
        &mut self,
        data: &[u8],
        out: &'a mut [u8],
        response_delay: Duration,
    ) -> Result<&'a mut [u8], Self::Error> {
        self.0.execute_raw(data, &mut [], Duration::ZERO)?;
        std::thread::sleep(BETWEEN_WRITES);
        self.0.execute_raw(data, out, response_delay)
    }
}

impl<D: DdcCommandRawMarker> DdcCommandMarker for Twice<'_, D> where D::Error: From<ErrorCode> {}

impl<D: DdcCommandRawMarker> DdcCommandRawMarker for Twice<'_, D>
where
    D::Error: From<ErrorCode>,
{
    fn set_sleep_delay(&mut self, delay: Delay) {
        self.0.set_sleep_delay(delay)
    }
}

#[cfg(test)]
mod tests {
    use super::Twice;
    use ddc::{DdcCommandRaw, DdcCommandRawMarker, DdcHost, Delay, ErrorCode};
    use ddc_hi::Ddc;
    use std::time::Duration;

    /// A DisplayPort link that ignores the first write of each request: the
    /// read after a single write gets the previous reply, garbled.
    #[derive(Default)]
    struct DropsFirstWrite {
        writes: Vec<Vec<u8>>,
        unread_writes: usize,
    }

    impl DdcHost for DropsFirstWrite {
        type Error = ErrorCode;
    }

    impl DdcCommandRaw for DropsFirstWrite {
        fn execute_raw<'a>(
            &mut self,
            data: &[u8],
            out: &'a mut [u8],
            _: Duration,
        ) -> Result<&'a mut [u8], ErrorCode> {
            self.writes.push(data.to_vec());
            self.unread_writes += 1;
            if out.is_empty() {
                return Ok(out);
            }
            let heard = self.unread_writes >= 2;
            self.unread_writes = 0;
            if !heard {
                return Err(ErrorCode::InvalidLength);
            }
            // VCP reply for the requested code: current 85, maximum 100.
            let reply = [0x02, 0x00, data[1], 0x00, 0x00, 100, 0x00, 85];
            out[..reply.len()].copy_from_slice(&reply);
            Ok(&mut out[..reply.len()])
        }
    }

    impl ddc::DdcCommandMarker for DropsFirstWrite {}

    impl DdcCommandRawMarker for DropsFirstWrite {
        fn set_sleep_delay(&mut self, _: Delay) {}
    }

    #[test]
    fn a_single_write_gets_no_reply_from_such_a_link() {
        let mut link = DropsFirstWrite::default();
        assert_eq!(link.get_vcp_feature(0x10), Err(ErrorCode::InvalidLength));
    }

    #[test]
    fn reads_send_the_request_twice_and_get_the_reply() {
        let mut link = DropsFirstWrite::default();
        let v = Twice(&mut link).get_vcp_feature(0x10).unwrap();
        assert_eq!((v.value(), v.maximum()), (85, 100));
        assert_eq!(link.writes.len(), 2);
        assert_eq!(link.writes[0], link.writes[1]);
    }

    #[test]
    fn writes_are_sent_twice() {
        let mut link = DropsFirstWrite::default();
        Twice(&mut link).set_vcp_feature(0x10, 40).unwrap();
        assert_eq!(link.writes.len(), 2);
        assert_eq!(link.writes[0], link.writes[1]);
    }
}
