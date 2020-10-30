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
    <T as atat::AtatCmd>::CommandLen: core::ops::Add<EdmAtCmdOverhead>,
    <<T as atat::AtatCmd>::CommandLen as core::ops::Add<EdmAtCmdOverhead>>::Output:
        atat::heapless::ArrayLength<u8>,
    <T as atat::AtatCmd>::Response: atat::serde_at::serde::Deserialize<'static>,
    // <T as atat::AtatCmd>::Response: atat::serde_at::serde::Deserialize<'b>,
{
    at_command: T,
}

impl <T> EdmAtCmdWrapper<T> 
where
    T: AtatCmd,
    <T as atat::AtatCmd>::CommandLen: core::ops::Add<EdmAtCmdOverhead>,
    <<T as atat::AtatCmd>::CommandLen as core::ops::Add<EdmAtCmdOverhead>>::Output:
        atat::heapless::ArrayLength<u8>,
    <T as atat::AtatCmd>::Response: atat::serde_at::serde::Deserialize<'static>,
    // <T as atat::AtatCmd>::Response: atat::serde_at::serde::Deserialize<'b>,
{
    pub fn new(at_command: T) -> Self{
        EdmAtCmdWrapper{ at_command }
    }

    pub fn get_at(&self) -> &T{
        &self.at_command
    }
}

// impl<'b, 'a: 'b, T> atat::AtatCmd for EdmAtCmdWrapper<'a, T>
impl<T> atat::AtatCmd for EdmAtCmdWrapper<T>
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

    // TODO: handle NoResponse in form of empty payload. Should do already
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
                Ok(EdmUrc::ATEvent(Urc::parse(&resp[AT_COMMAND_POSITION .. PAYLOAD_POSITION + payload_len as usize])?))
            }
            
            _ => Err(atat::Error::ParseString)
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

    // TODO: handle NoResponse in form of empty payload. Should do already
    fn parse(&self, resp: &[u8]) -> core::result::Result<Self::Response, atat::Error> {
        let correct = &[0xAAu8,0x00,0x06,0x00,0x45,0x4f,0x4b,0x0D,0x0a,0x55];
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
    use atat::{AtatCmd, AtatLen, AtatResp, AtatUrc, heapless::{Vec, consts}};
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
        let parse = EdmAtCmdWrapper::new(AT);

        // AT-command:"AT"
        let correct = Vec::<u8, consts::U10>::from_slice(&[
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
        
        let parse = EdmAtCmdWrapper::new(SystemStatus{status_id: Some(StatusID::SavedStatus)});

        // AT-command:"at+umstat=1"
        let correct = Vec::<u8, consts::U19>::from_slice(&[
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
    }

    #[test]
    fn change_to_edm_cmd(){
        let resp = &[0xAAu8,0x00,0x06,0x00,0x45,0x4f,0x4b,0x0D,0x0a,0x55];
        let correct = Vec::<u8, <ChangeMode as atat::AtatCmd>::CommandLen>::from_slice(b"ATO=2\r\n").unwrap();
        assert_eq!(SwitchToEdmCommand.as_bytes(), correct);
        assert_eq!(SwitchToEdmCommand.parse(resp).unwrap(), NoResponse);
    }
}

// #[derive(Clone)]
// pub struct EdmAtResponseWrapper<'a, T> where T: AtatResp {
//     pub at_resp: &'a T,
// }

// impl <'a, T> AtatResp for EdmAtResponseWrapper<'a, T> where T: AtatResp{}
