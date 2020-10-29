/// Containing EDM structs with custom serialaization and deserilaisation.
use atat::{AtatCmd, AtatLen, AtatResp, AtatUrc};
use super::Urc;

/// Start byte, Length: u16, Id+Type: u16, Endbyte
// type EdmAtCmdOverhead = (u8, u16, u16, u8);

type EdmAtCmdOverhead = atat::heapless::consts::U6;

const STARTBYTE: u8 = 0xAA;
const ENDBYTE: u8 = 0x55;
const EDM_SIZE_FILTER: u8 = 0x0F;
const EDM_FULL_SIZE_FILTER: u16 = 0x0FFF;
const EDM_OVERHEAD: usize = 4;
const PAYLOAD_OVERHEAD: usize = 6;
/// Index in packet at which AT-command starts
const AT_COMMAND_POSITION: usize = 5;
/// Index in packet at which payload starts
const PAYLOAD_POSITION: usize = 3;

#[derive(Debug, PartialEq)]
#[repr(u8)]
enum PayloadType {
    /// Sent by the module to inform the host about a new connection.
    ConnectEvent = 0x11,
    /// Sent by the module to inform the host about the loss of connection.
    DisconnectEvent = 0x21,
    /// Sent by the module when data is received over air.
    DataEvent = 0x31,
    /// Sent to the module to send data over air. No acknowledge is transmitted by the module.
    DataCommand = 0x36,
    /// Special packet to execute an AT command. One or many AT Confirmation packets are transmitted back by the module.
    ATRequest = 0x44,
    /// AT Response.
    /// The module sends one or many confirmations as a response to an AT Request. The
    /// number of confirmation packets depends on what AT command that is being
    /// executed.
    ATConfirmation = 0x45,
    /// AT URC.
    /// There are a number of AT events that can be sent by the module. See the
    /// u-connect AT Commands Manual [1] for details.
    ATEvent = 0x41,
    /// Special command to make the module re-transmit Connect Events for connections
    /// still active. This can be useful, for example, when the host has reset or just been
    /// started.
    ResendConnectEventsCommand = 0x56,
    /// Special iPhone events, for example, session status and power state.
    IPhoneEvent = 0x61,
    /// Sent when the module recovers from reset or at power on. This packet may need
    /// special module configuration to be transmitted.
    StartEvent = 0x71,
    Unknown = 0x00,
}

impl From<u8> for PayloadType{
    fn from(num: u8) -> Self{
        match num {
            0x11u8 => PayloadType::ConnectEvent,
            0x21u8 => PayloadType::DisconnectEvent,
            0x31u8 => PayloadType::DataEvent,
            0x36u8 => PayloadType::DataCommand,
            0x44u8 => PayloadType::ATRequest,
            0x45u8 => PayloadType::ATConfirmation,
            0x41u8 => PayloadType::ATEvent,
            0x56u8 => PayloadType::ResendConnectEventsCommand,
            0x61u8 => PayloadType::IPhoneEvent,
            0x71u8 => PayloadType::StartEvent,
            _ => PayloadType::Unknown,
        }
    }
}

/// EDM wrapper for AT-Commands
// Note:
// The AT+UMRS command to change serial settings does not work exactly the same as in command
// mode. When executed in the extended data mode, it is not possible to change the settings directly
// using the <change_after_confirm> parameter. Instead the <change_after_confirm> parameter must
// be set to 0 and the serial settings will take effect when the module is reset.
#[derive(Debug, Clone)]
pub struct EdmAtCmdWrapper<'a, T>
where
    T: AtatCmd,
    <T as atat::AtatCmd>::CommandLen: core::ops::Add<EdmAtCmdOverhead>,
    <<T as atat::AtatCmd>::CommandLen as core::ops::Add<EdmAtCmdOverhead>>::Output:
        atat::heapless::ArrayLength<u8>,
    <T as atat::AtatCmd>::Response: atat::serde_at::serde::Deserialize<'static>,
    // <T as atat::AtatCmd>::Response: atat::serde_at::serde::Deserialize<'b>,
{
    pub at_command: &'a T,
}

impl <'a, T> EdmAtCmdWrapper<'a, T> 
where
    T: AtatCmd,
    <T as atat::AtatCmd>::CommandLen: core::ops::Add<EdmAtCmdOverhead>,
    <<T as atat::AtatCmd>::CommandLen as core::ops::Add<EdmAtCmdOverhead>>::Output:
        atat::heapless::ArrayLength<u8>,
    <T as atat::AtatCmd>::Response: atat::serde_at::serde::Deserialize<'static>,
    // <T as atat::AtatCmd>::Response: atat::serde_at::serde::Deserialize<'b>,
{
    fn new(at_command: &'a T) -> Self{
        EdmAtCmdWrapper{ at_command }
    }
}

// impl <'a, T> atat::AtatLen for EdmAtCmdWrapper<'a, T>
// where T: AtatCmd + AtatLen,
// {
//     type Len = <T as atat::AtatLen>::Len;
//     // type Len = <T as atat::AtatCmd>::CommandLen;
// }

// impl<'b, 'a: 'b, T> atat::AtatCmd for EdmAtCmdWrapper<'a, T>
impl<'a , T> atat::AtatCmd for EdmAtCmdWrapper<'a, T>
where
    T: AtatCmd,
    <T as atat::AtatCmd>::CommandLen: core::ops::Add<EdmAtCmdOverhead>,
    <<T as atat::AtatCmd>::CommandLen as core::ops::Add<EdmAtCmdOverhead>>::Output:
        atat::heapless::ArrayLength<u8>,
    <T as atat::AtatCmd>::Response: atat::serde_at::serde::Deserialize<'static>,
    // <T as atat::AtatCmd>::Response: atat::serde_at::serde::Deserialize<'b>,
{
    // type Response = EdmAtResponseWrapper<'a, T::Response>;
    type Response = T::Response;
    type CommandLen =
        <<T as atat::AtatCmd>::CommandLen as core::ops::Add<EdmAtCmdOverhead>>::Output;

    fn as_bytes(&self) -> atat::heapless::Vec<u8, Self::CommandLen> {
        let mut s: atat::heapless::Vec<u8, Self::CommandLen> = atat::heapless::Vec::new();
        let at_vec = self.at_command.as_bytes();
        let payload_len = (at_vec.len() + 2) as u16;
        s.extend(
            [
                STARTBYTE,
                (payload_len >> 8) as u8 & EDM_SIZE_FILTER,
                (payload_len & 0xffu16) as u8,
                0x00,
                PayloadType::ATRequest as u8,
            ]
            .iter()
        );
        s.extend(at_vec.iter());
        s.push(ENDBYTE).unwrap_or_else(|_| core::unreachable!());
        return s;
    }

    // TODO: handle NoResponse
    fn parse(&self, resp: &[u8]) -> core::result::Result<Self::Response, atat::Error> {
        if resp.len() < PAYLOAD_OVERHEAD
            || !resp.starts_with(&[STARTBYTE])
            || !resp.ends_with(&[ENDBYTE])
        {
            return Err(atat::Error::ParseString);
        };
        let payload_len = ((resp[1] as u16) << 8 + resp[2] as u16) & EDM_FULL_SIZE_FILTER;
        if resp.len() != payload_len as usize + EDM_OVERHEAD
            || resp[4] != PayloadType::ATConfirmation as u8
        {
            return Err(atat::Error::ParseString);
        }
        Err(atat::Error::Aborted)
        // let at_resp = &resp[4..2 + payload_len as usize];
        // atat::serde_at::from_slice::<Self::Response>(at_resp).map_err(|e| atat::Error::ParseString)

    }

    fn force_receive_state(&self) -> bool {
        true
    }
}

#[derive(Debug, PartialEq)]
enum EdmUrc{
    ConnectEvent(),
    DisconnectEvent(),
    DataEvent(),
    ATEvent(Urc)
}

impl AtatUrc for EdmUrc{
    /// The type of the response. Usually the enum this trait is implemented on.
    type Response = EdmUrc;

    /// Parse the response into a `Self::Response` instance.
    fn parse(resp: &[u8]) -> Result<Self::Response, atat::Error>{
        if resp.len() < PAYLOAD_OVERHEAD
            || !resp.starts_with(&[STARTBYTE])
            || !resp.ends_with(&[ENDBYTE])
        {
            return Err(atat::Error::ParseString);
        };
        let payload_len = (((resp[1] as u16) << 8) + resp[2] as u16) & EDM_FULL_SIZE_FILTER;
        if resp.len() != payload_len as usize + EDM_OVERHEAD {
            return Err(atat::Error::ParseString);
        }

        match resp[4].into() {
            PayloadType::ATEvent => {
                Ok(EdmUrc::ATEvent(Urc::parse(&resp[AT_COMMAND_POSITION .. PAYLOAD_POSITION + payload_len as usize - 1 ])?))
            }
            
            _ => Err(atat::Error::ParseString)
        }
    }
}

pub(crate) mod custom_digest {
    use atat::atat_log;
    use atat::error::Error;
    use atat::ingress_manager::{
        get_line, IngressManager, SliceExt, State, UrcMatcher, UrcMatcherResult, ByteVec,
    };
    use atat::queues::{ComItem, ResItem, UrcItem};
    use atat::heapless::{ArrayLength, Vec};
    use super::{EDM_OVERHEAD, EDM_FULL_SIZE_FILTER, PayloadType};

    /// Custom function to process the receive buffer, checking for AT responses, URC's or errors
    ///
    /// This function should be called regularly for the ingress manager to work
    pub(crate) fn custom_digest<BufLen, U, ComCapacity, ResCapacity, UrcCapacity>(
        ingress: &mut IngressManager<BufLen, U, ComCapacity, ResCapacity, UrcCapacity>,
    ) where
        U: UrcMatcher<BufLen>,
        BufLen: ArrayLength<u8>,
        ComCapacity: ArrayLength<ComItem>,
        ResCapacity: ArrayLength<ResItem<BufLen>>,
        UrcCapacity: ArrayLength<UrcItem<BufLen>>,
    {
        // Handle commands
        ingress.handle_com();

        let end_byte = ingress.get_line_term_char();
        let start_byte = ingress.get_format_char();
        // Echo is currently not suported in EDM
        if ingress.get_echo_enabled() {
            unimplemented!("Enabeling echo is currently unsupported for EDM");
        }

        let start_pos = match ingress.buf.windows(1).position(|byte| byte == &[start_byte]){
            Some(pos) => pos,
            None => return,
        };

        // Trim leading invalid data.
        if start_pos != 0 {
            ingress.buf = Vec::from_slice(&ingress.buf[start_pos.. ingress.buf.len()]).unwrap();
        }

        // Verify payload length and end byte position
        if ingress.buf.len() < EDM_OVERHEAD{
            return;
        }
        let payload_len = (((ingress.buf[1] as u16) << 8 + ingress.buf[2] as u16) & EDM_FULL_SIZE_FILTER) as usize;
        let edm_len = payload_len + EDM_OVERHEAD;
        if ingress.buf.len() < edm_len {
            return;
        } else if ingress.buf[edm_len -1] != end_byte{
            return;
        }

        match PayloadType::from(ingress.buf[4]) {
            PayloadType::ATConfirmation => {
                let (resp, mut remaining) = ingress.buf.split_at(edm_len - 1);
                let mut return_val: Option<Result<ByteVec<BufLen>, Error>> = None;
                if ingress.get_state() == State::ReceivingResponse {    
                    if let Some(_) = resp.windows(b"ERROR".len()).position(|window| window == b"ERROR" ) {
                        return_val = Some(Err(Error::InvalidResponse));
                    } else if let Some(_) = resp.windows(b"OK".len()).position(|window| window == b"OK" ) {
                        return_val = Some(Ok(ByteVec::<BufLen>::from_slice(&[
                            0xAAu8,
                            0x00,
                            0x02,
                            0x00,
                            PayloadType::ATConfirmation as u8,
                            0x55,
                            ]).unwrap()));
                    } else {
                        //Normal response check if OK recived at end? else return to wait for OK to be received at end.
                        let start_pos = match remaining.windows(1).position(|byte| byte == &[start_byte]){
                            Some(pos) => pos,
                            None => return,
                        };
                
                        // Trim leading invalid data.
                        if start_pos != 0 {
                            remaining = &remaining[start_pos.. remaining.len()];
                        }
                
                        // Verify payload length and end byte position
                        if remaining.len() < EDM_OVERHEAD{
                            return;
                        }
                        let payload_len = (((remaining[1] as u16) << 8 + remaining[2] as u16) & EDM_FULL_SIZE_FILTER) as usize;
                        let edm_len = payload_len + EDM_OVERHEAD;
                        if remaining.len() < edm_len {
                            return;
                        } else if remaining[edm_len -1] != end_byte{
                            return;
                        }
                        if PayloadType::from(remaining[4]) == PayloadType::ATConfirmation 
                            && remaining.windows(b"OK".len()).position(|window| window == b"OK" ) != None {
                                // Found trailing OK response remove from remaining
                                
                            }

                    }
                }
                ingress.buf = Vec::from_slice(remaining).unwrap();
                if let Some(resp) = return_val {
                    ingress.notify_response(resp)
                }
            },
            PayloadType::ATEvent=> {
                let (resp, remaining) = ingress.buf.split_at(edm_len - 1);
                let resp = ByteVec::<BufLen>::from_slice(resp).unwrap();
                ingress.buf = Vec::from_slice(remaining).unwrap();
                ingress.notify_urc(resp);
            }
            _ => {
                // Error packet
            }
        }


        // Check packet if valid:
        // Length
        // Endbyte 
        // TypeID

        // Add to queue according to type



        // Trim leading whitespace
        if ingress.buf.starts_with(&[start_byte]) || ingress.buf.starts_with(&[end_byte]) {
            
        }

        #[allow(clippy::single_match)]
        match core::str::from_utf8(&ingress.buf) {
            Ok(_s) => {
                #[cfg(not(feature = "log-logging"))]
                atat_log!(trace, "Digest / {:str}", _s);
                #[cfg(feature = "log-logging")]
                atat_log!(trace, "Digest / {:?}", _s);
            }
            Err(_) => atat_log!(
                trace,
                "Digest / {:?}",
                core::convert::AsRef::<[u8]>::as_ref(&buf)
            ),
        };

        match ingress.get_state() {
            State::Idle => {
                // The minimal buffer length that is required to identify all
                // types of responses (e.g. `AT` and `+`).
                let min_length = 2;

                // Echo is currently required
                if !ingress.get_echo_enabled() {
                    unimplemented!("Disabling AT echo is currently unsupported");
                }

                // Handle AT echo responses
                if !ingress.get_buf_incomplete() && ingress.buf.get(0..2) == Some(b"AT") {
                    if get_line::<BufLen, _>(
                        &mut ingress.buf,
                        &[end_byte],
                        end_byte,
                        end_byte,
                        false,
                        false,
                    )
                    .is_some()
                    {
                        ingress.set_state(State::ReceivingResponse);
                        ingress.set_buf_incomplete(false);
                        atat_log!(trace, "Switching to state ReceivingResponse");
                    }

                // Handle URCs
                } else if !ingress.get_buf_incomplete() && ingress.buf.get(0) == Some(&b'+') {
                    // Try to apply the custom URC matcher
                    let handled = match ingress.custom_urc_matcher {
                        Some(ref mut matcher) => match matcher.process(&mut ingress.buf) {
                            UrcMatcherResult::NotHandled => false,
                            UrcMatcherResult::Incomplete => true,
                            UrcMatcherResult::Complete(urc) => {
                                ingress.notify_urc(urc);
                                true
                            }
                        },
                        None => false,
                    };
                    if !handled {
                        if let Some(line) = get_line(
                            &mut ingress.buf,
                            &[end_byte],
                            end_byte,
                            end_byte,
                            false,
                            false,
                        ) {
                            ingress.set_buf_incomplete(false);
                            ingress.notify_urc(line);
                        }
                    }

                // Text sent by the device that is not a valid response type (e.g. starting
                // with "AT" or "+") can be ignored. Clear the buffer, but only if we can
                // ensure that we don't accidentally break a valid response.
                } else if ingress.get_buf_incomplete() || ingress.buf.len() > min_length {
                    atat_log!(
                        trace,
                        "Clearing buffer with invalid response (incomplete: {:?}, buflen: {:?})",
                        buf_incomplete,
                        ingress.buf.len()
                    );
                    ingress.set_buf_incomplete(
                        ingress.buf.is_empty()
                            || (ingress.buf.len() > 0
                                && ingress.buf.get(ingress.buf.len() - 1) != Some(&end_byte)
                                && ingress.buf.get(ingress.buf.len() - 1) != Some(&end_byte)),
                    );

                    ingress.clear_buf(false);

                    // If the buffer wasn't cleared completely, that means that
                    // a newline was found. In that case, the buffer cannot be
                    // in an incomplete state.
                    if !ingress.buf.is_empty() {
                        ingress.set_buf_incomplete(false);
                    }
                }
            }
            State::ReceivingResponse => {
                let resp = if let Some(mut line) = get_line::<BufLen, _>(
                    &mut ingress.buf,
                    b"OK",
                    end_byte,
                    end_byte,
                    true,
                    false,
                ) {
                    Ok(get_line(
                        &mut line,
                        &[end_byte],
                        end_byte,
                        end_byte,
                        true,
                        true,
                    )
                    .unwrap_or_else(Vec::new))
                } else if get_line::<BufLen, _>(
                    &mut ingress.buf,
                    b"ERROR",
                    end_byte,
                    end_byte,
                    false,
                    false,
                )
                .is_some()
                {
                    Err(Error::InvalidResponse)
                } else if get_line::<BufLen, _>(
                    &mut ingress.buf,
                    b">",
                    end_byte,
                    end_byte,
                    false,
                    false,
                )
                .is_some()
                    || get_line::<BufLen, _>(
                        &mut ingress.buf,
                        b"@",
                        end_byte,
                        end_byte,
                        false,
                        false,
                    )
                    .is_some()
                {
                    Ok(Vec::new())
                } else {
                    return;
                };

                ingress.notify_response(resp);
                atat_log!(trace, "Switching to state Idle");
                ingress.set_state(State::Idle);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use atat::{AtatCmd, AtatLen, AtatResp, AtatUrc};
    use crate::command::{
        Urc, 
        AT,
        system::{
            SystemStatus,
            types::StatusID,
        },
        data_mode::urc::PeerDisconnected,
    };
    
    #[test]
    fn parse_at_commands(){
        let parse = EdmAtCmdWrapper::new(&AT);

        // AT-command:"at"
        let correct = atat::heapless::Vec::<u8, atat::heapless::consts::U10>::from_slice(&[
                0xAAu8,
                0x00,
                0x06,
                0x00,
                0x44,
                0x41,
                0x54,
                0x0D,
                0x0a,
                0x55,
            ]).unwrap();
        assert_eq!(parse.as_bytes(), correct, "Parsing packet incorrect.\nExpectet: {:?}\nRecived:  {:?}", correct, parse.as_bytes());
        
        let parse = EdmAtCmdWrapper::new(&SystemStatus{status_id: Some(StatusID::SavedStatus)});

        // AT-command:"at+umstat=1"
        let correct = atat::heapless::Vec::<u8, atat::heapless::consts::U19>::from_slice(&[
                0xAAu8,
                0x00,
                0x0F,
                0x00,
                0x44,
                0x41, 
                0x54, 
                0x2b, 
                0x55, 
                0x4d, 
                0x53, 
                0x54, 
                0x41, 
                0x54, 
                0x3d, 
                0x31,
                0x0D,
                0x0A,
                0x55,
            ]).unwrap();
        assert_eq!(parse.as_bytes(), correct, "Parsing packet incorrect.");
       
    }

    #[test]
    fn parse_urc(){
        let resp = &[
            0xAAu8,
            0x00,
            0x0C,
            0x00,
            0x41, //[4]
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
        let urc = EdmUrc::ATEvent(
            Urc::PeerDisconnected(PeerDisconnected{ handle: 3 })
        );    
        let parsed_urc = EdmUrc::parse(resp).unwrap();
        assert_eq!(parsed_urc, urc, "Parsing URC failed");
        
        
        // assert_eq!(1,2, "resp.len: {}", resp.len());
    }
}

// #[derive(Clone)]
// pub struct EdmAtResponseWrapper<'a, T> where T: AtatResp {
//     pub at_resp: &'a T,
// }

// impl <'a, T> AtatResp for EdmAtResponseWrapper<'a, T> where T: AtatResp{}
