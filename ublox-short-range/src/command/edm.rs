/// Containing EDM structs with custom serialaization and deserilaisation.
use atat::{AtatCmd, AtatLen, AtatResp, AtatUrc};
use crate::command::{data_mode::ChangeMode, data_mode};
use crate::command::{Urc, NoResponse};

/// Start byte, Length: u16, Id+Type: u16, Endbyte
// type EdmAtCmdOverhead = (u8, u16, u16, u8);

type EdmAtCmdOverhead = atat::heapless::consts::U6;

pub(crate) const STARTBYTE: u8 = 0xAA;
pub(crate) const ENDBYTE: u8 = 0x55;
pub(crate) const EDM_SIZE_FILTER: u8 = 0x0F;
pub(crate) const EDM_FULL_SIZE_FILTER: u16 = 0x0FFF;
pub(crate) const EDM_OVERHEAD: usize = 4;
pub(crate) const PAYLOAD_OVERHEAD: usize = 6;
/// Index in packet at which AT-command starts
pub(crate) const AT_COMMAND_POSITION: usize = 5;
/// Index in packet at which payload starts
pub(crate) const PAYLOAD_POSITION: usize = 3;
pub(crate) const STARTUPMESSAGE: &[u8] = b"\r\n+STARTUP\r\n";

#[inline]
pub(crate) fn calc_payload_len(resp: &[u8]) -> usize{
    ((((resp[1] as u16) << 8) + resp[2] as u16) & EDM_FULL_SIZE_FILTER) as usize
}

#[derive(Debug, PartialEq)]
#[repr(u8)]
pub(crate) enum PayloadType {
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
pub struct EdmAtCmdWrapper<T>
where
    T: AtatCmd,
{
    at_command: T,
}

impl <T> EdmAtCmdWrapper<T> 
where
    T: AtatCmd,
{
    pub fn new(at_command: T) -> Self{
        EdmAtCmdWrapper{ at_command }
    }

    pub fn get_at(&self) -> &T{
        &self.at_command
    }
}

impl<T> atat::AtatCmd for EdmAtCmdWrapper<T>
where
    T: AtatCmd,
    <T as atat::AtatCmd>::CommandLen: core::ops::Add<EdmAtCmdOverhead>,
    <<T as atat::AtatCmd>::CommandLen as core::ops::Add<EdmAtCmdOverhead>>::Output:
        atat::heapless::ArrayLength<u8>,
{
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

    fn parse(&self, resp: &[u8]) -> core::result::Result<Self::Response, atat::Error> {
        if resp.len() < PAYLOAD_OVERHEAD
            || !resp.starts_with(&[STARTBYTE])
            || !resp.ends_with(&[ENDBYTE])
        {
            return Err(atat::Error::InvalidResponse);
        };
        let payload_len = calc_payload_len(resp);
        if resp.len() != payload_len + EDM_OVERHEAD
            || resp[4] != PayloadType::ATConfirmation as u8 {
            return Err(atat::Error::InvalidResponse);
        }
        // Isolate the AT_response
        let at_resp = &resp[AT_COMMAND_POSITION .. PAYLOAD_POSITION + payload_len];
        self.at_command.parse(at_resp)
    }

    fn force_receive_state(&self) -> bool {
        true
    }

    fn max_timeout_ms(&self) -> u32 {
        self.at_command.max_timeout_ms()
    }
}

#[derive(Debug, PartialEq)]
pub enum EdmUrc{
    ConnectEvent,
    DisconnectEvent,
    DataEvent,
    ATEvent(Urc),
    // TODO: Handle modlue restart. Especially to Digest
    StartUp,
}

impl AtatUrc for EdmUrc{
    /// The type of the response. Usually the enum this trait is implemented on.
    type Response = EdmUrc;

    /// Parse the response into a `Self::Response` instance.
    fn parse(resp: &[u8]) -> Result<Self::Response, atat::Error>{
        // #[cfg(feature = "logging")]
        // log::info!("[Parse URC] {:?}", resp);
        //Startup message?
        if resp.windows(STARTUPMESSAGE.len()).position(|window| window == STARTUPMESSAGE) == Some(0){
            return Ok(EdmUrc::StartUp);
        }

        if resp.len() < PAYLOAD_OVERHEAD
            || !resp.starts_with(&[STARTBYTE])
            || !resp.ends_with(&[ENDBYTE]) {
            // #[cfg(feature = "logging")]
            // log::info!("[Parse URC Error] {:?}", resp);
            return Err(atat::Error::InvalidResponse);
        };
        let payload_len = calc_payload_len(resp);
        if resp.len() != payload_len + EDM_OVERHEAD {
            // #[cfg(feature = "logging")]
            // log::info!("[Parse URC Error] {:?}", resp);
            return Err(atat::Error::InvalidResponse);
        }

        match resp[4].into() {
            PayloadType::ATEvent => {
                // #[cfg(feature = "logging")]
                // log::info!("[Parse URC AT-CMD]: {:?}", &resp[AT_COMMAND_POSITION .. PAYLOAD_POSITION + payload_len]);
                let cmd = Urc::parse(&resp[AT_COMMAND_POSITION .. PAYLOAD_POSITION + payload_len])?;
                Ok(EdmUrc::ATEvent(cmd))
            }
            
            _ => {
                // #[cfg(feature = "logging")]
                // log::info!("[Parse URC Error] {:?}", resp);
                Err(atat::Error::InvalidResponse)
            }
        }
    }
}

pub struct SwitchToEdmCommand;

impl atat::AtatCmd for SwitchToEdmCommand{
    type Response = NoResponse;
    type CommandLen = <ChangeMode as atat::AtatCmd>::CommandLen;

    fn as_bytes(&self) -> atat::heapless::Vec<u8, Self::CommandLen> {
        return ChangeMode{mode: data_mode::types::Mode::ExtendedDataMode}.as_bytes();
    }

    fn parse(&self, resp: &[u8]) -> core::result::Result<Self::Response, atat::Error> {
        let correct = &[0xAAu8, 0x00, 0x02, 0x00, 0x71, 0x55];      // &[0xAAu8,0x00,0x06,0x00,0x45,0x4f,0x4b,0x0D,0x0a,0x55]; //AA 00 06 00 44 41 54 0D 0A 0D 0A 4F 4B 0D 0A 55 ?
        if resp.len() != correct.len() {
            return Err(atat::Error::InvalidResponse)
        } else if resp.windows(correct.len()).position(|window| window == correct) != Some(0){
            return Err(atat::Error::InvalidResponse)
        }
        Ok(NoResponse)
    }

    fn force_receive_state(&self) -> bool {
        true
    }
}


#[cfg(test)]
mod test {
    use super::*;
    use atat::{AtatCmd, AtatLen, AtatResp, AtatUrc, Error, heapless::{Vec, consts}};
    use crate::command::{
        Urc, 
        AT,
        system::{
            SystemStatus,
            types::StatusID,
            responses::SystemStatusResponse,
        },
        data_mode::urc::PeerDisconnected,
    };
    
    #[test]
    fn parse_at_commands(){
        let parse = EdmAtCmdWrapper::new(AT);
        let correct_response = NoResponse;

        // AT-command: "AT"
        let correct_cmd = Vec::<u8, consts::U10>::from_slice(&[
                0xAAu8, 0x00, 0x06, 0x00, 0x44, 0x41, 0x54, 0x0D, 0x0a, 0x55,
            ]).unwrap();
        // AT-response: NoResponse
        let response = &[
                0xAAu8, 0x00, 0x02, 0x00, PayloadType::ATConfirmation as u8, 0x55,
            ];
        assert_eq!(parse.as_bytes(), correct_cmd);
        assert_eq!(parse.parse(response), Ok(correct_response));
        
        let parse = EdmAtCmdWrapper::new(SystemStatus{status_id: StatusID::SavedStatus});
        let correct_response = SystemStatusResponse{status_id: StatusID::SavedStatus, status_val: 100};
        // AT-command: "at+umstat=1"
        let correct = Vec::<u8, consts::U19>::from_slice(&[
                0xAAu8, 0x00, 0x0F, 0x00, 0x44, 0x41, 0x54, 0x2b, 0x55, 0x4d, 0x53, 0x54, 0x41, 0x54, 0x3d, 0x31,0x0D,0x0A,0x55,
            ]).unwrap();
        // AT-response: "at+umstat:1,100"
        let response = &[
                0xAAu8,0x00,0x11,0x00,PayloadType::ATConfirmation as u8,0x2B,0x55, 0x4D, 0x53, 0x54, 0x41, 0x54, 0x3A, 0x31, 0x2C, 
                0x31, 0x30, 0x30,0x0D,0x0A,0x55, 
            ];
        assert_eq!(parse.as_bytes(), correct);
        assert_eq!(parse.parse(response), Ok(correct_response));
    }

    #[test]
    fn parse_wrong_at_responses(){
        let parse = EdmAtCmdWrapper::new(AT);
        let correct_response = NoResponse;
        // AT-response: NoResponse
        let response = &[
                0xAAu8, 0x00, 0x06, 0x00, PayloadType::ATConfirmation as u8, 0x55,
            ];
        assert_eq!(parse.parse(response), Err(Error::InvalidResponse), "Response shorter than indicated not invalid");

        let parse = EdmAtCmdWrapper::new(SystemStatus{status_id: StatusID::SavedStatus});
        // AT-response: "at+umstat:1,100"
        let response =&[
                0xAAu8,0x00,0x01,0x00,PayloadType::ATConfirmation as u8,0x2B,0x55, 0x4D, 0x53, 0x54, 0x41, 0x54, 0x3A, 0x31, 0x2C, 
                0x31, 0x30, 0x30,0x0D,0x0A,0x55, 
            ];
        assert_eq!(parse.parse(response), Err(Error::InvalidResponse), "Response longer than indicated not invalid");

        let response =&[
                0xAAu8,0x00,0x11,0x00,PayloadType::ATConfirmation as u8,0x2B,0x55, 0x4D, 0x53, 0x54, 0x41, 0x54, 0x3A, 0x31, 0x2C, 
                0x31, 0x30, 0x30,0x0D,0x0A,0x00, 
            ];  
        assert_eq!(parse.parse(response), Err(Error::InvalidResponse), "Response wrong endbyte not invalid");

        let response =&[
                0x00u8,0x00,0x11,0x00,PayloadType::ATConfirmation as u8,0x2B,0x55, 0x4D, 0x53, 0x54, 0x41, 0x54, 0x3A, 0x31, 0x2C, 
                0x31, 0x30, 0x30,0x0D,0x0A,0x55, 
            ];
        assert_eq!(parse.parse(response), Err(Error::InvalidResponse), "Response wrong startbyte not invalid");

        let response = &[
                0xAAu8, 0x00, 0x02, 0x00, PayloadType::ATConfirmation as u8, 0x55,
            ];
        assert_eq!(parse.parse(response), Err(Error::ParseString), "Response wrong not invalid");
    }

    #[test]
    fn parse_urc(){
        // AT-urc: +UUDPD:3
        let resp = &[ 
            // 0xAAu8, 0x00, 0x0E, 0x00, PayloadType::ATEvent as u8, 0x0D, 0x0A, 0x2B, 0x55, 0x55, 0x44, 0x50, 0x44, 0x3A, 0x33, 0x0D, 0x0A, 0x55,
            // 0xAAu8, 0x00, 0x0C, 0x00, PayloadType::ATEvent as u8, 0x2B, 0x55, 0x55, 0x44, 0x50, 0x44, 0x3A, 0x33, 0x0D, 0x0A, 0x55,
            0xAAu8, 0x00, 0x1B, 0x00, 0x41, 0x2B, 0x55, 0x55, 0x57, 0x4C, 0x45, 0x3A, 0x30, 0x2C, 0x33, 0x32, 0x41, 0x42, 0x36, 0x41, 0x37, 0x41, 0x34, 0x30, 0x34, 0x34, 0x2C, 0x31, 0x0D, 0x0A, 0x55,
            ];
        let urc = EdmUrc::ATEvent(
            Urc::PeerDisconnected(PeerDisconnected{ handle: 3 })
        );    
        let parsed_urc = EdmUrc::parse(resp);
        assert_eq!(parsed_urc, Ok(urc), "Parsing URC failed");
    }

    #[test]
    fn change_to_edm_cmd(){
        let resp = &[0xAAu8, 0x00, 0x02, 0x00, 0x71, 0x55];
        let correct = Vec::<u8, <ChangeMode as atat::AtatCmd>::CommandLen>::from_slice(b"ATO2\r\n").unwrap();
        assert_eq!(SwitchToEdmCommand.as_bytes(), correct);
        assert_eq!(SwitchToEdmCommand.parse(resp).unwrap(), NoResponse);
    }
}