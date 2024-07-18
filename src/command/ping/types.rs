//! Argument and parameter types used by Ping Commands and Responses

use atat::atat_derive::AtatEnum;

/// Indicates the number of iterations for the ping command.
/// - Range: 1-2147483647
/// - Default value: 4
// pub type RetryNum = (u32, Option<PacketSize>);
/// Size in bytes of the echo packet payload.
/// - Range: 4-1472
/// - Default value: 32
// pub type PacketSize = (u16, Option<Timeout>);
/// The maximum time in milliseconds to wait for an echo reply response.
/// - Range: 10-60000
/// - Default value: 5000
// pub type Timeout = (u16, Option<TTL>);
/// The value of TTL to be set for the outgoing echo request packet. In the URC, it
/// provides the TTL value received in the incoming packet.
/// - Range: 1-255
/// - Default value: 32
// pub type TTL = (u8, Option<Interval>);
/// The time in milliseconds to wait after an echo reply response before sending the next
/// echo request.
/// - Range: 0-60000
/// - Default value: 1000
// pub type Interval = u16;

#[derive(Debug, PartialEq, Clone, Copy, AtatEnum)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
pub enum PingError {
    /// 1 - 6: Internal error (ping level)
    InternalError1 = 1,
    /// 1 - 6: Internal error (ping level)
    InternalError2 = 2,
    /// 1 - 6: Internal error (ping level)
    InternalError3 = 3,
    /// 1 - 6: Internal error (ping level)
    InternalError4 = 4,
    /// 1 - 6: Internal error (ping level)
    InternalError5 = 5,
    /// 1 - 6: Internal error (ping level)
    InternalError6 = 6,
    /// 7: Empty remote host
    EmptyRemoteHost = 7,
    /// 8: Cannot resolve host
    CannotResolveHost = 8,
    /// 9: Unsupported IP version (RFU)
    UnsupportedIPVersion = 9,
    /// 10: Invalid IPv4 address
    InvalidIPv4 = 10,
    /// 11: Invalid IPv6 address (RFU)
    InvalidIPv6 = 11,
    /// 12: Remote host too long
    RemoteHostTooLong = 12,
    /// 13: Invalid payload size
    InvalidPayloadSize = 13,
    /// 14: Invalid TTL value
    InvalidTTL = 14,
    /// 15: Invalid timeout value
    InvalidTimeout = 15,
    /// 16: Invalid retries number
    InvalidRetries = 16,
    /// 17: PSD or CSD connection not established
    PSDorCSDNotEstablished = 17,
    /// 100 - 105: Internal error (ICMP level)
    ICMPLevel0 = 100,
    ICMPLevel1 = 101,
    ICMPLevel2 = 102,
    ICMPLevel3 = 103,
    ICMPLevel4 = 104,
    ICMPLevel5 = 105,
    /// 106: Error creating socket for ICMP
    SocketCreateError = 106,
    /// 107: Error settings socket options for ICMP
    SocektSettingsError = 107,
    /// 108: Cannot end ICMP packet
    CannotEndIMCP = 108,
    /// 109: Read for ICMP packet failed
    IMCPReadFailed = 109,
    /// 110: Received unexpected ICMP packet
    UnexpectedIMCPPacket = 110,
    /// 111-115: Internal error (socket level)
    SocketErrorLevel1 = 111,
    /// 111-115: Internal error (socket level)
    SocketErrorLevel2 = 112,
    /// 111-115: Internal error (socket level)
    SocketErrorLevel3 = 113,
    /// 111-115: Internal error (socket level)
    SocketErrorLevel4 = 114,
    /// 111-115: Internal error (socket level)
    SocketErrorLevel5 = 115,
    Timeout,
    Other,
}
