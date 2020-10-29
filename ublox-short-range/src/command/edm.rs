/// Containing EDM structs with custom serialaization and deserilaisation.
use atat::{AtatCmd, AtatLen, AtatResp, AtatUrc};
use super::Urc;

/// Start byte, Length: u16, Id+Type: u16, Endbyte
// type EdmAtCmdOverhead = (u8, u16, u16, u8);

type EdmAtCmdOverhead = atat::heapless::consts::U6;

const STARTBYTE: u8 = 0xAA;
const ENDBYTE: u8 = 0x55;
const EDM_OVERHEAD: usize = 4;
const PAYLOAD_OVERHEAD: usize = 6;
/// Index in packet at which AT-command starts
const AT_COMMAND_POSITION: usize = 5;
/// Index in packet at which payload starts
const PAYLOAD_POSITION: usize = 3;

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
    /// The module sends one or many confirmations as a response to an AT Request. The
    /// number of confirmation packets depends on what AT command that is being
    /// executed.
    ATConfirmation = 0x45,
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
                (payload_len >> 8) as u8 & 0x0fu8,
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
            return Err(atat::Error::ParseString);
        };
        let payload_len = ((resp[1] as u16) << 8 + resp[2] as u16) & 0x0FFF;
        if resp.len() != payload_len as usize + EDM_OVERHEAD
            || resp[4] != PayloadType::ATConfirmation as u8
        {
            return Err(atat::Error::ParseString);
        }
        Err(atat::Error::Aborted)
        // let at_resp = &resp[4..2 + payload_len as usize];
        // atat::serde_at::from_slice::<Self::Response>(at_resp).map_err(|e| atat::Error::ParseString)

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
        let payload_len = (((resp[1] as u16) << 8) + resp[2] as u16) & 0x0FFF;
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
