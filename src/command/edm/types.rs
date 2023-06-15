use heapless::Vec;
use no_std_net::{Ipv4Addr, Ipv6Addr};
use serde::{Deserialize, Serialize};
use ublox_sockets::ChannelId;

/// Start byte, Length: u16, Id+Type: u16, Endbyte
// type EdmAtCmdOverhead = (u8, u16, u16, u8);

pub const DATA_PACKAGE_SIZE: usize = 4096;
pub const STARTBYTE: u8 = 0xAA;
pub const ENDBYTE: u8 = 0x55;
pub const EDM_SIZE_FILTER: u8 = 0x0F;
pub const EDM_FULL_SIZE_FILTER: u16 = 0x0FFF;
pub const EDM_OVERHEAD: usize = 4;
pub const PAYLOAD_OVERHEAD: usize = 6;
/// Index in packet at which AT-command starts
pub const AT_COMMAND_POSITION: usize = 5;
/// Index in packet at which payload starts
pub const PAYLOAD_POSITION: usize = 3;
pub const STARTUPMESSAGE: &[u8] = b"\r\n+STARTUP\r\n";
pub const AUTOCONNECTMESSAGE: &[u8] = b"\r\n+UUWLE:0,XXXXXXXXXXXX,1\r\n";

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
    /// Sent on entering EDM mode
    StartEvent = 0x71,
    Unknown = 0x00,
}

impl From<u8> for PayloadType {
    fn from(num: u8) -> Self {
        match num {
            0x11 => PayloadType::ConnectEvent,
            0x21 => PayloadType::DisconnectEvent,
            0x31 => PayloadType::DataEvent,
            0x36 => PayloadType::DataCommand,
            0x44 => PayloadType::ATRequest,
            0x45 => PayloadType::ATConfirmation,
            0x41 => PayloadType::ATEvent,
            0x56 => PayloadType::ResendConnectEventsCommand,
            0x61 => PayloadType::IPhoneEvent,
            0x71 => PayloadType::StartEvent,
            _ => PayloadType::Unknown,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BluetoothConnectEvent {
    pub channel_id: ChannelId,
    pub profile: BluetoothConnectType,
    pub bd_address: Vec<u8, 6>,
    pub frame_size: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IPv4ConnectEvent {
    pub channel_id: ChannelId,
    pub protocol: Protocol,
    pub remote_ip: Ipv4Addr,
    pub remote_port: u16,
    pub local_ip: Ipv4Addr,
    pub local_port: u16,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IPv6ConnectEvent {
    pub channel_id: ChannelId,
    pub protocol: Protocol,
    pub remote_ip: Ipv6Addr,
    pub remote_port: u16,
    pub local_ip: Ipv6Addr,
    pub local_port: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataEvent {
    pub channel_id: ChannelId,
    pub data: Vec<u8, DATA_PACKAGE_SIZE>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum BluetoothConnectType {
    SSP = 0,
    DUN = 1,
    SerialPortServiceBLE = 14,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum ConnectType {
    Bluetooth = 0x01,
    IPv4 = 0x02,
    IPv6 = 0x03,
    Unknown = 0,
}

impl From<u8> for ConnectType {
    fn from(num: u8) -> Self {
        match num {
            1 => ConnectType::Bluetooth,
            2 => ConnectType::IPv4,
            3 => ConnectType::IPv6,
            _ => ConnectType::Unknown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Protocol {
    TCP = 0x00,
    UDP = 0x01,
    Unknown = 0xFF,
}

impl From<u8> for Protocol {
    fn from(num: u8) -> Self {
        match num {
            0 => Protocol::TCP,
            1 => Protocol::UDP,
            _ => Protocol::Unknown,
        }
    }
}
