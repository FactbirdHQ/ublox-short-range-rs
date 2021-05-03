use crate::command::edm::{
    calc_payload_len,
    types::{PayloadType, AT_COMMAND_POSITION, EDM_OVERHEAD, ENDBYTE, STARTBYTE},
};
// use atat::atat_log;
use atat::Error;
use atat::{helpers::SliceExt, DigestResult, Digester, UrcMatcher};
use heapless::{ArrayLength, Vec};

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

    fn digest<L: ArrayLength<u8>>(
        &mut self,
        buf: &mut Vec<u8, L>,
        _urc_matcher: &mut impl UrcMatcher,
    ) -> DigestResult<L> {
        
        // TODO Handle module restart, tests and set default startupmessage in client, and optimiz this!
        
        let start_pos = match buf.windows(1).position(|byte| byte[0] == STARTBYTE) {
            Some(pos) => pos,
            None => return DigestResult::None, //handle leading error data. //TODO handle error input before messagestart.
        };
        
        // Trim leading invalid data.
        if start_pos != 0 {
            *buf = Vec::from_slice(&buf[start_pos..buf.len()]).unwrap();
        }
        
        // Verify payload length and end byte position
        if buf.len() < EDM_OVERHEAD {
            return DigestResult::None;
        }
        let payload_len = calc_payload_len(&buf);
        
        let edm_len = payload_len + EDM_OVERHEAD;
        if buf.len() < edm_len {
            return DigestResult::None;
        } else if buf[edm_len - 1] != ENDBYTE {
            return DigestResult::None;
        }
        
        // Debug statement for trace properly
        if buf.len() != 0 {
            match core::str::from_utf8(&buf) {
                Ok(_s) => defmt::trace!("Recived: {=str}, state: {:?}", _s, self.state),
                // Ok(_s) => atat_log!(trace, "Recived: {:str}, state: {:?}", _s, self.state),
                Err(_) => defmt::trace!(
                    "Recived: {:?}, state: {:?}",
                    core::convert::AsRef::<[u8]>::as_ref(&buf),
                    self.state,
                ),
                // Err(_) => atat_log!(
                //     trace,
                //     "Recived: {:?}, state: {:?}",
                //     core::convert::AsRef::<[u8]>::as_ref(&buf),
                //     self.state,
                // ),
            };
        }
        // Filter message by payload
        match PayloadType::from(buf[4]) {
            PayloadType::ATConfirmation => {
                let (resp, remaining) = buf.split_at(edm_len);
                let mut return_val = DigestResult::None;
                if self.state == State::ReceivingResponse {
                    // Errors can arrive with and without leading whitespaces
                    if resp.windows(b"ERROR".len()).nth(AT_COMMAND_POSITION) == Some(b"ERROR")
                        || resp.windows(b"ERROR".len()).nth(AT_COMMAND_POSITION + 2)
                            == Some(b"ERROR")
                    {
                        return_val = DigestResult::Response(Err(Error::InvalidResponse));
                    } else {
                        return_val = DigestResult::Response(Ok(Vec::from_slice(resp).unwrap()));
                    }
                }
                *buf = Vec::from_slice(remaining).unwrap();
                self.state = State::Idle;
                return return_val;
            }
            PayloadType::StartEvent => {
                let (resp, remaining) = buf.split_at(edm_len);
                let mut return_val = DigestResult::None;
                if self.state == State::ReceivingResponse {
                    self.state = State::Idle;
                    return_val = DigestResult::Response(Ok(Vec::from_slice(resp).unwrap()));
                }
                *buf = Vec::from_slice(remaining).unwrap();
                return return_val;
            }
            PayloadType::ATEvent => {
                // Recived AT URC
                let (resp, remaining) = buf.split_at(edm_len);
                let (header, urc) = resp.split_at(AT_COMMAND_POSITION);

                let urc_trimmed = urc.trim_start(&[b'\n', b'\r']);

                let mut resp = Vec::from_slice(header).unwrap();
                resp.extend(urc_trimmed);
                resp[2] -= (urc.len() - urc_trimmed.len()) as u8;

                *buf = Vec::from_slice(remaining).unwrap();
                return DigestResult::Urc(resp);
            }
            PayloadType::ConnectEvent | PayloadType::DataEvent | PayloadType::DisconnectEvent => {
                // Recived EDM event
                let (resp, remaining) = buf.split_at(edm_len);
                let resp = Vec::from_slice(resp).unwrap();
                *buf = Vec::from_slice(remaining).unwrap();
                return DigestResult::Urc(resp);
            }
            _ => {
                // Wrong/Unsupported packet, thrown away.
                let (_, remaining) = buf.split_at(edm_len);
                *buf = Vec::from_slice(remaining).unwrap();
                return DigestResult::None;
            }
        }
    }
}

#[cfg(test)]
mod test {
    //TODO: Rewrite tests for new builder structure

    use super::*;
    use atat::{ComQueue, ResQueue, UrcQueue};
    use atat::{Command, DefaultUrcMatcher, IngressManager};
    use heapless::{consts, spsc::Queue};

    type TestRxBufLen = consts::U256;
    type TestUrcCapacity = consts::U10;

    macro_rules! setup_ingressmanager {
        () => {{
            static mut RES_Q: ResQueue<TestRxBufLen> = Queue(heapless::i::Queue::u8());
            let (res_p, res_c) = unsafe { RES_Q.split() };
            static mut URC_Q: UrcQueue<TestRxBufLen, TestUrcCapacity> =
                Queue(heapless::i::Queue::u8());
            let (urc_p, urc_c) = unsafe { URC_Q.split() };
            static mut COM_Q: ComQueue = Queue(heapless::i::Queue::u8());
            let (com_p, com_c) = unsafe { COM_Q.split() };
            (
                IngressManager::with_customs(
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

    // Removed functionality used to change OK responses to empty responses.
    #[test]
    fn ok_response() {
        let (mut at_pars, mut res_c, mut urc_c, mut com_p) = setup_ingressmanager!();

        com_p.enqueue(Command::ForceReceiveState).unwrap();
        at_pars.digest();

        //Payload: "OK\r\n"
        let data = &[0xAAu8, 0x00, 0x06, 0x00, 0x45, 0x4f, 0x4b, 0x0D, 0x0a, 0x55];
        let empty_ok_response = Vec::<u8, TestRxBufLen>::from_slice(&[
            0xAAu8, 0x00, 0x06, 0x00, 0x45, 0x4f, 0x4b, 0x0D, 0x0a, 0x55,
        ])
        .unwrap();

        at_pars.write(data);

        at_pars.digest();
        assert_eq!(res_c.dequeue(), Some(Ok(empty_ok_response)));
        assert_eq!(urc_c.dequeue(), None);
    }

    #[test]
    fn error_response() {
        let (mut at_pars, mut res_c, mut urc_c, mut com_p) = setup_ingressmanager!();

        //Payload: "ERROR\r\n"
        let data = &[
            0xAAu8, 0x00, 0x09, 0x00, 0x45, 0x45, 0x52, 0x52, 0x4f, 0x52, 0x0D, 0x0a, 0x55,
        ];

        com_p.enqueue(Command::ForceReceiveState).unwrap();
        at_pars.digest();
        at_pars.write(data);

        at_pars.digest();
        assert_eq!(res_c.dequeue(), Some(Err(Error::InvalidResponse)));
        assert_eq!(urc_c.dequeue(), None);
    }

    #[test]
    fn regular_response_with_trailing_ok() {
        let (mut at_pars, mut res_c, mut urc_c, mut com_p) = setup_ingressmanager!();
        com_p.enqueue(Command::ForceReceiveState).unwrap();
        at_pars.digest();

        //Payload: AT\r\n
        let response = &[0xAAu8, 0x00, 0x06, 0x00, 0x45, 0x41, 0x54, 0x0D, 0x0a, 0x55];
        // Data = response + trailing OK message
        let data = &[
            0xAAu8, 0x00, 0x06, 0x00, 0x45, 0x41, 0x54, 0x0D, 0x0a, 0x55, 0xAA, 0x00, 0x06, 0x00,
            0x45, 0x4f, 0x4b, 0x0D, 0x0a, 0x55,
        ];

        at_pars.write(data);
        at_pars.digest();
        assert_eq!(
            res_c.dequeue(),
            Some(Ok(Vec::<u8, TestRxBufLen>::from_slice(response).unwrap()))
        );
        assert_eq!(urc_c.dequeue(), None);
    }

    // Regular response with traling regular response..

    #[test]
    fn at_urc() {
        let (mut at_pars, mut res_c, mut urc_c, _) = setup_ingressmanager!();

        let type_byte = PayloadType::ATEvent as u8;
        //Payload: "OK\r\n"
        let data = &[
            0xAAu8, 0x00, 0x0E, 0x00, type_byte, 0x0D, 0x0A, 0x2B, 0x55, 0x55, 0x44, 0x50, 0x44,
            0x3A, 0x33, 0x0D, 0x0A, 0x55,
        ];
        let result = &[
            0xAAu8, 0x00, 0x0C, 0x00, type_byte, 0x2B, 0x55, 0x55, 0x44, 0x50, 0x44, 0x3A, 0x33,
            0x0D, 0x0A, 0x55,
        ];
        at_pars.write(data);
        at_pars.digest();
        assert_eq!(
            urc_c.dequeue(),
            Some(Vec::<u8, TestRxBufLen>::from_slice(result).unwrap())
        );
        assert_eq!(res_c.dequeue(), None);
    }

    #[test]
    fn data_event() {
        let (mut at_pars, mut res_c, mut urc_c, _) = setup_ingressmanager!();

        let type_byte = PayloadType::DataEvent as u8;
        //Payload: "OK\r\n"
        let data = &[
            0xAAu8,
            0x00,
            0x0E,
            0x00,
            type_byte as u8,
            0x0D,
            0x0A,
            0x2B,
            0x55,
            0x55,
            0x44,
            0x50,
            0x44,
            0x3A,
            0x33,
            0x0D,
            0x0A,
            0x55,
        ];
        let result = &[
            0xAAu8,
            0x00,
            0x0E,
            0x00,
            type_byte as u8,
            0x0D,
            0x0A,
            0x2B,
            0x55,
            0x55,
            0x44,
            0x50,
            0x44,
            0x3A,
            0x33,
            0x0D,
            0x0A,
            0x55,
        ];
        at_pars.write(data);
        at_pars.digest();
        assert_eq!(
            urc_c.dequeue(),
            Some(Vec::<u8, TestRxBufLen>::from_slice(result).unwrap())
        );
        assert_eq!(res_c.dequeue(), None);
    }

    #[test]
    fn connect_disconnect_events() {
        let (mut at_pars, mut res_c, mut urc_c, _) = setup_ingressmanager!();

        let type_byte = PayloadType::ConnectEvent as u8;
        //Payload: "OK\r\n"
        let data = &[
            0xAAu8,
            0x00,
            0x0E,
            0x00,
            type_byte as u8,
            0x0D,
            0x0A,
            0x2B,
            0x55,
            0x55,
            0x44,
            0x50,
            0x44,
            0x3A,
            0x33,
            0x0D,
            0x0A,
            0x55,
        ];
        let result = &[
            0xAAu8,
            0x00,
            0x0E,
            0x00,
            type_byte as u8,
            0x0D,
            0x0A,
            0x2B,
            0x55,
            0x55,
            0x44,
            0x50,
            0x44,
            0x3A,
            0x33,
            0x0D,
            0x0A,
            0x55,
        ];
        at_pars.write(data);
        at_pars.digest();
        assert_eq!(
            urc_c.dequeue(),
            Some(Vec::<u8, TestRxBufLen>::from_slice(result).unwrap())
        );
        assert_eq!(res_c.dequeue(), None);

        let type_byte = PayloadType::DisconnectEvent as u8;
        //Payload: "OK\r\n"
        let data = &[
            0xAAu8,
            0x00,
            0x0E,
            0x00,
            type_byte as u8,
            0x0D,
            0x0A,
            0x2B,
            0x55,
            0x55,
            0x44,
            0x50,
            0x44,
            0x3A,
            0x33,
            0x0D,
            0x0A,
            0x55,
        ];
        let result = &[
            0xAAu8,
            0x00,
            0x0E,
            0x00,
            type_byte as u8,
            0x0D,
            0x0A,
            0x2B,
            0x55,
            0x55,
            0x44,
            0x50,
            0x44,
            0x3A,
            0x33,
            0x0D,
            0x0A,
            0x55,
        ];
        at_pars.write(data);
        at_pars.digest();
        assert_eq!(
            urc_c.dequeue(),
            Some(Vec::<u8, TestRxBufLen>::from_slice(result).unwrap())
        );
        assert_eq!(res_c.dequeue(), None);
    }

    #[test]
    fn wrong_type_packet() {
        let (mut at_pars, mut res_c, mut urc_c, mut com_p) = setup_ingressmanager!();

        let type_byte = PayloadType::Unknown as u8;
        //Payload: "OK\r\n"
        let data = &[
            0xAAu8, 0x00, 0x06, 0x00, type_byte, 0x4f, 0x4b, 0x0D, 0x0a, 0x55,
        ];
        at_pars.write(data);
        at_pars.digest();
        assert_eq!(urc_c.dequeue(), None);
        assert_eq!(res_c.dequeue(), None);

        // In reciver state!
        com_p.enqueue(Command::ForceReceiveState).unwrap();
        at_pars.digest();

        at_pars.write(data);
        at_pars.digest();
        assert_eq!(urc_c.dequeue(), None);
        assert_eq!(res_c.dequeue(), None);

        // Recovered enough to recive normal data?
        com_p.enqueue(Command::ForceReceiveState).unwrap();
        at_pars.digest();

        //Payload: "OK\r\n"
        let data = &[0xAAu8, 0x00, 0x06, 0x00, 0x45, 0x4f, 0x4b, 0x0D, 0x0a, 0x55];
        let empty_ok_response = Vec::<u8, TestRxBufLen>::from_slice(&[
            0xAAu8, 0x00, 0x06, 0x00, 0x45, 0x4f, 0x4b, 0x0D, 0x0a, 0x55,
        ])
        .unwrap();

        at_pars.write(data);

        at_pars.digest();
        assert_eq!(res_c.dequeue(), Some(Ok(empty_ok_response)));
        assert_eq!(urc_c.dequeue(), None);
    }
}
