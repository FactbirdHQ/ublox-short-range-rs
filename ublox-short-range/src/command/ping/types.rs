//! Argument and parameter types used by Ping Commands and Responses

use atat::atat_derive::AtatEnum;
use heapless::{consts, String, Vec};
use no_std_net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Indicates the number of iterations for the ping command.
/// • Range: 1-2147483647
/// • Default value: 4
pub type RetryNum = (u32, Option<PacketSize>);
/// Size in bytes of the echo packet payload.
/// • Range: 4-1472
/// • Default value: 32
pub type PacketSize = (u16, Option<Timeout>);
/// The maximum time in milliseconds to wait for an echo reply response.
/// • Range: 10-60000
/// • Default value: 5000
pub type Timeout = (u16, Option<TTL>);
/// The value of TTL to be set for the outgoing echo request packet. In the URC, it
/// provides the TTL value received in the incoming packet.
/// • Range: 1-255
/// • Default value: 32
pub type TTL = (u8, Option<Inteval>);
/// The time in milliseconds to wait after an echo reply response before sending the next
/// echo request.
/// • Range: 0-60000
/// • Default value: 1000
pub type Inteval = u16;
