use crate::command::edm::{
    calc_payload_len,
    types::{PayloadType, AT_COMMAND_POSITION, EDM_OVERHEAD, ENDBYTE, STARTBYTE},
};
use atat::InternalError;
use atat::{DigestResult, Digester, UrcMatcher};
use heapless::Vec;

/// State of the `EDMDigester`, used to filter responses
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, defmt::Format)]
enum State {
    Idle,
    ReceivingResponse,
}

impl Default for State {
    fn default() -> Self {
        Self::Idle
    }
}

/// Digester for EDM context
#[derive(Debug, Default)]
pub struct EdmDigester {
    /// Current processing state.
    state: State,

    /// A flag that is set to `true` when the buffer is cleared
    /// with an incomplete response.
    buf_incomplete: bool,
}

impl Digester for EdmDigester {
    /// Command line termination character S3 (Default = b'\r' ASCII: \[013\])
    const LINE_TERM_CHAR: u8 = ENDBYTE;

    /// Response formatting character S4 (Default = b'\n' ASCII: \[010\])
    const FORMAT_CHAR: u8 = STARTBYTE;

    fn reset(&mut self) {
        self.state = State::Idle;
        self.buf_incomplete = false;
    }

    fn force_receive_state(&mut self) {
        self.state = State::ReceivingResponse;
    }

    fn digest<const L: usize>(
        &mut self,
        buf: &mut Vec<u8, L>,
        _urc_matcher: &mut impl UrcMatcher,
    ) -> DigestResult<L> {
        // TODO: Handle module restart, tests and set default startupmessage in client, and optimize this!

        let start_pos = match buf.windows(1).position(|byte| byte[0] == STARTBYTE) {
            Some(pos) => pos,
            None => return DigestResult::None, //handle leading error data. //TODO handle error input before messagestart.
        };

        // Trim leading invalid data.
        if start_pos != 0 {
            buf.rotate_left(start_pos);
            buf.truncate(buf.len() - start_pos);
        }

        // Verify payload length and end byte position
        if buf.len() < EDM_OVERHEAD {
            return DigestResult::None;
        }
        let payload_len = calc_payload_len(&buf);

        let edm_len = payload_len + EDM_OVERHEAD;
        if buf.len() < edm_len || buf[edm_len - 1] != ENDBYTE {
            return DigestResult::None;
        }

        // Debug statement for trace properly
        if !buf.is_empty() {
            defmt::trace!("Digest {:?} / {=[u8]:a}", self.state, &buf);
        }

        // Filter message by payload
        match PayloadType::from(buf[4]) {
            PayloadType::ATConfirmation => {
                let resp = &buf[..edm_len];
                let return_val = match self.state {
                    State::ReceivingResponse
                        if resp.windows(b"ERROR".len()).nth(AT_COMMAND_POSITION)
                            == Some(b"ERROR")
                            || resp.windows(b"ERROR".len()).nth(AT_COMMAND_POSITION + 2)
                                == Some(b"ERROR") =>
                    {
                        DigestResult::Response(Err(InternalError::InvalidResponse))
                    }
                    State::ReceivingResponse => {
                        DigestResult::Response(Ok(Vec::from_slice(resp).unwrap()))
                    }
                    _ => DigestResult::None,
                };

                buf.rotate_left(edm_len);
                buf.truncate(buf.len() - edm_len);

                self.state = State::Idle;
                return_val
            }
            PayloadType::StartEvent => {
                let resp = &buf[..edm_len];
                let return_val = match self.state {
                    State::ReceivingResponse => {
                        self.state = State::Idle;
                        DigestResult::Response(Ok(Vec::from_slice(resp).unwrap()))
                    }
                    _ => DigestResult::None,
                };
                buf.rotate_left(edm_len);
                buf.truncate(buf.len() - edm_len);

                return_val
            }
            PayloadType::ATEvent => {
                // Received AT URC
                let resp = &mut buf[..edm_len];
                let (header, urc) = resp.split_at_mut(AT_COMMAND_POSITION);

                let is_not_whitespace = |c| ![b'\n', b'\r'].contains(c);
                let urc_trimmed = if let Some(idx) = urc.iter().position(is_not_whitespace) {
                    urc.rotate_left(idx);
                    &urc[..urc.len() - idx]
                } else {
                    &urc
                };

                let mut resp = Vec::from_slice(header).unwrap();
                resp.extend_from_slice(urc_trimmed).unwrap();
                resp[2] -= (urc.len() - urc_trimmed.len()) as u8;

                buf.rotate_left(edm_len);
                buf.truncate(buf.len() - edm_len);
                DigestResult::Urc(resp)
            }
            PayloadType::ConnectEvent | PayloadType::DataEvent | PayloadType::DisconnectEvent => {
                // Received EDM event
                let resp = Vec::from_slice(&buf[..edm_len]).unwrap();
                buf.rotate_left(edm_len);
                buf.truncate(buf.len() - edm_len);
                DigestResult::Urc(resp)
            }
            _ => {
                // Wrong/Unsupported packet, thrown away.
                buf.rotate_left(edm_len);
                buf.truncate(buf.len() - edm_len);
                DigestResult::None
            }
        }
    }
}

#[cfg(test)]
mod test {
    //TODO: Rewrite tests for new builder structure

    use super::*;
    use atat::bbqueue::BBBuffer;
    use atat::{ComQueue, ResponseHeader};
    use atat::{Command, DefaultUrcMatcher, IngressManager};
    use heapless::spsc::Queue;

    const TEST_RX_BUF_LEN: usize = 256;
    const TEST_RES_CAPACITY: usize = 3 * TEST_RX_BUF_LEN;
    const TEST_URC_CAPACITY: usize = 3 * TEST_RX_BUF_LEN;

    macro_rules! setup_ingressmanager {
        () => {{
            static mut RES_Q: BBBuffer<TEST_RES_CAPACITY> = BBBuffer::new();
            let (res_p, res_c) = unsafe { RES_Q.try_split_framed().unwrap() };

            static mut URC_Q: BBBuffer<TEST_URC_CAPACITY> = BBBuffer::new();
            let (urc_p, urc_c) = unsafe { URC_Q.try_split_framed().unwrap() };

            static mut COM_Q: ComQueue = Queue::new();
            let (com_p, com_c) = unsafe { COM_Q.split() };
            (
                IngressManager::<_, _, TEST_RX_BUF_LEN, TEST_RES_CAPACITY, TEST_URC_CAPACITY>::with_customs(
                    res_p,
                    urc_p,
                    com_c,
                    DefaultUrcMatcher::default(),
                    EdmDigester::default(),
                ),
                res_c,
                urc_c,
                com_p,
            )
        }};
    }

    /// Removed functionality used to change OK responses to empty responses.
    #[test]
    fn ok_response() {
        let (mut at_pars, mut res_c, mut urc_c, mut com_p) = setup_ingressmanager!();

        com_p.enqueue(Command::ForceReceiveState).unwrap();
        at_pars.digest();

        //Payload: "OK\r\n"
        let data = &[0xAA, 0x00, 0x06, 0x00, 0x45, 0x4f, 0x4b, 0x0D, 0x0a, 0x55];
        let empty_ok_response = &[0xAA, 0x00, 0x06, 0x00, 0x45, 0x4f, 0x4b, 0x0D, 0x0a, 0x55];

        at_pars.write(data);

        at_pars.digest();

        let mut grant = res_c.read().unwrap();
        grant.auto_release(true);
        assert_eq!(
            ResponseHeader::from_bytes(grant.as_ref()),
            Ok(&empty_ok_response[..])
        );
        assert_eq!(urc_c.read(), None);
    }

    #[test]
    fn error_response() {
        let (mut at_pars, mut res_c, mut urc_c, mut com_p) = setup_ingressmanager!();

        //Payload: "ERROR\r\n"
        let data = &[
            0xAA, 0x00, 0x09, 0x00, 0x45, 0x45, 0x52, 0x52, 0x4f, 0x52, 0x0D, 0x0a, 0x55,
        ];

        com_p.enqueue(Command::ForceReceiveState).unwrap();
        at_pars.digest();
        at_pars.write(data);

        at_pars.digest();
        let mut grant = res_c.read().unwrap();
        grant.auto_release(true);
        assert_eq!(
            ResponseHeader::from_bytes(grant.as_ref()),
            Err(InternalError::InvalidResponse)
        );
        assert_eq!(urc_c.read(), None);
    }

    #[test]
    fn regular_response_with_trailing_ok() {
        let (mut at_pars, mut res_c, mut urc_c, mut com_p) = setup_ingressmanager!();
        com_p.enqueue(Command::ForceReceiveState).unwrap();
        at_pars.digest();

        //Payload: AT\r\n
        let response = &[0xAA, 0x00, 0x06, 0x00, 0x45, 0x41, 0x54, 0x0D, 0x0a, 0x55];
        // Data = response + trailing OK message
        let data = &[
            0xAA, 0x00, 0x06, 0x00, 0x45, 0x41, 0x54, 0x0D, 0x0a, 0x55, 0xAA, 0x00, 0x06, 0x00,
            0x45, 0x4f, 0x4b, 0x0D, 0x0a, 0x55,
        ];

        at_pars.write(data);
        at_pars.digest();

        let mut grant = res_c.read().unwrap();
        grant.auto_release(true);
        assert_eq!(
            ResponseHeader::from_bytes(grant.as_ref()),
            Ok(&response[..])
        );
        assert_eq!(urc_c.read(), None);
    }

    /// Regular response with traling regular response..
    #[test]
    fn at_urc() {
        let (mut at_pars, mut res_c, mut urc_c, _) = setup_ingressmanager!();

        let type_byte = PayloadType::ATEvent as u8;
        //Payload: "OK\r\n"
        let data = &[
            0xAA, 0x00, 0x0E, 0x00, type_byte, 0x0D, 0x0A, 0x2B, 0x55, 0x55, 0x44, 0x50, 0x44,
            0x3A, 0x33, 0x0D, 0x0A, 0x55,
        ];
        let result = &[
            0xAA, 0x00, 0x0C, 0x00, type_byte, 0x2B, 0x55, 0x55, 0x44, 0x50, 0x44, 0x3A, 0x33,
            0x0D, 0x0A, 0x55,
        ];
        at_pars.write(data);
        at_pars.digest();

        let mut grant = urc_c.read().unwrap();
        grant.auto_release(true);
        assert_eq!(grant.as_ref(), result);
        assert_eq!(res_c.read(), None);
    }

    #[test]
    fn data_event() {
        let (mut at_pars, mut res_c, mut urc_c, _) = setup_ingressmanager!();

        let type_byte = PayloadType::DataEvent as u8;
        //Payload: "OK\r\n"
        let data = &[
            0xAA, 0x00, 0x0E, 0x00, type_byte, 0x0D, 0x0A, 0x2B, 0x55, 0x55, 0x44, 0x50, 0x44,
            0x3A, 0x33, 0x0D, 0x0A, 0x55,
        ];
        let result = &[
            0xAA, 0x00, 0x0E, 0x00, type_byte, 0x0D, 0x0A, 0x2B, 0x55, 0x55, 0x44, 0x50, 0x44,
            0x3A, 0x33, 0x0D, 0x0A, 0x55,
        ];
        at_pars.write(data);
        at_pars.digest();

        let mut grant = urc_c.read().unwrap();
        grant.auto_release(true);
        assert_eq!(grant.as_ref(), result);
        assert_eq!(res_c.read(), None);
    }

    #[test]
    fn connect_disconnect_events() {
        let (mut at_pars, mut res_c, mut urc_c, _) = setup_ingressmanager!();

        let type_byte = PayloadType::ConnectEvent as u8;
        //Payload: "OK\r\n"
        let data = &[
            0xAA, 0x00, 0x0E, 0x00, type_byte, 0x0D, 0x0A, 0x2B, 0x55, 0x55, 0x44, 0x50, 0x44,
            0x3A, 0x33, 0x0D, 0x0A, 0x55,
        ];
        let result = &[
            0xAA, 0x00, 0x0E, 0x00, type_byte, 0x0D, 0x0A, 0x2B, 0x55, 0x55, 0x44, 0x50, 0x44,
            0x3A, 0x33, 0x0D, 0x0A, 0x55,
        ];
        at_pars.write(data);
        at_pars.digest();
        let mut grant = urc_c.read().unwrap();
        grant.auto_release(true);
        assert_eq!(grant.as_ref(), result);
        assert_eq!(res_c.read(), None);
        drop(grant);

        let type_byte = PayloadType::DisconnectEvent as u8;
        //Payload: "OK\r\n"
        let data = &[
            0xAA, 0x00, 0x0E, 0x00, type_byte, 0x0D, 0x0A, 0x2B, 0x55, 0x55, 0x44, 0x50, 0x44,
            0x3A, 0x33, 0x0D, 0x0A, 0x55,
        ];
        let result = &[
            0xAA, 0x00, 0x0E, 0x00, type_byte, 0x0D, 0x0A, 0x2B, 0x55, 0x55, 0x44, 0x50, 0x44,
            0x3A, 0x33, 0x0D, 0x0A, 0x55,
        ];
        at_pars.write(data);
        at_pars.digest();

        let mut grant = urc_c.read().unwrap();
        grant.auto_release(true);
        assert_eq!(grant.as_ref(), result);
        assert_eq!(res_c.read(), None);
    }

    #[test]
    fn wrong_type_packet() {
        let (mut at_pars, mut res_c, mut urc_c, mut com_p) = setup_ingressmanager!();

        let type_byte = PayloadType::Unknown as u8;
        //Payload: "OK\r\n"
        let data = &[
            0xAA, 0x00, 0x06, 0x00, type_byte, 0x4f, 0x4b, 0x0D, 0x0a, 0x55,
        ];
        at_pars.write(data);
        at_pars.digest();

        assert_eq!(urc_c.read(), None);
        assert_eq!(res_c.read(), None);

        // In receiver state!
        com_p.enqueue(Command::ForceReceiveState).unwrap();
        at_pars.digest();

        at_pars.write(data);
        at_pars.digest();
        assert_eq!(urc_c.read(), None);
        assert_eq!(res_c.read(), None);

        // Recovered enough to receive normal data?
        com_p.enqueue(Command::ForceReceiveState).unwrap();
        at_pars.digest();

        //Payload: "OK\r\n"
        let data = &[0xAA, 0x00, 0x06, 0x00, 0x45, 0x4f, 0x4b, 0x0D, 0x0a, 0x55];
        let empty_ok_response = Vec::<u8, TEST_RX_BUF_LEN>::from_slice(&[
            0xAA, 0x00, 0x06, 0x00, 0x45, 0x4f, 0x4b, 0x0D, 0x0a, 0x55,
        ])
        .unwrap();

        at_pars.write(data);
        at_pars.digest();

        let mut grant = res_c.read().unwrap();
        grant.auto_release(true);
        assert_eq!(
            ResponseHeader::from_bytes(grant.as_ref()),
            Ok(&empty_ok_response[..])
        );
        assert_eq!(urc_c.read(), None);
    }
}
