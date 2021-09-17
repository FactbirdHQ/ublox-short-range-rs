//! ### 20 - Ping Commands
pub mod types;
pub mod urc;

use super::NoResponse;
use atat::atat_derive::AtatCmd;

/// 16.1 Ping command +UPING
///
/// The ping command is the common method to know if a remote host is reachable on the Internet.
/// The ping functionality is based on the Internet Control Message Protocol (ICMP); it is part of the Internet
/// Protocol Suite as defined in RFC 792 . The ICMP messages are typically generated in response to the errors in
/// IP datagrams or for diagnostic/routing purposes.
/// The ping command sends an ICMP echo request to the remote host and waits for its ICMP echo reply. If the
/// echo reply packet is not received, it means that the remote host is not reachable.
/// The ping command is also used to measure:
/// - The Round Trip Time (RTT), the time needed by a packet to go to the remote host and come back and
/// - The Time To Live (TTL), the value to understand how many gateway a packet has gone through.
/// The AT+UPING allows the user to execute a ping command from the module to a remote host. The results
/// of the ping command execution is notified through the +UUPING: URC, which reports the +UPING command
/// result (when there is no error).
/// OBS: Some remote hosts might not reply to the ICMP echo request for security reasons (for example, firewall
/// settings).
/// OBS: Some remote hosts might not reply to the ICMP echo request if the data size of the echo request is too big.
/// OBS: If a remote host does not reply to an ICMP echo request, it does not mean that the host cannot be reached
/// in another way.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UPING", NoResponse, timeout_ms = 10000)]
pub struct Ping<'a> {
    /// IP address (dotted decimal representation) or domain name of the remote host
    /// - Maximum length: 64 characters
    #[at_arg(position = 0, len = 64)]
    pub hostname: &'a str,
    /// Indicates the number of iterations for the ping command.
    /// - Range: 1-2147483647(i32 max)
    /// - Default value: 4
    #[at_arg(position = 1)]
    pub retry_num: i32,
}
