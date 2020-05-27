//! AT Commands for U-Blox short range module family\
//! Following the [u-connect ATCommands Manual](https://www.u-blox.com/sites/default/files/u-connect-ATCommands-Manual_(UBX-14044127).pdf)

pub mod data_mode;
pub mod ethernet;
pub mod general;
pub mod network;
pub mod security;
pub mod system;
pub mod wifi;
pub mod gpio;
pub mod ping;

use atat::atat_derive::{AtatCmd, AtatResp, AtatUrc};
use heapless::String;
use heapless::consts;

#[derive(Clone, AtatResp)]
pub struct NoResponse;

#[derive(Clone, AtatCmd)]
#[at_cmd("", NoResponse, timeout_ms = 1000)]
pub struct AT;

#[derive(Clone, AtatUrc)]
pub enum Urc {
    // /// 5.10 Peer connected +UUDPC
// #[at_urc("+UUDPC")]
// /// 5.11 Peer disconnected +UUDPD
// #[at_urc("+UUDPD")]
// 7.15 Wi-Fi Link connected +UUWLE
// #[at_urc("+UUWLE")]
// 7.16 Wi-Fi Link disconnected +UUWLD
// #[at_urc("+UUWLD")]
// 7.17 Wi-Fi Access point up +UUWAPU
// #[at_urc("+UUWAPU")]
// 7.18 Wi-Fi Access point down +UUWAPD
// #[at_urc("+UUWAPD")]
// 7.19 Wi-Fi Access point station connected +UUWAPSTAC
// #[at_urc("+UUWAPSTAC")]
// 7.20 Wi-Fi Access point station disconnected +UUWAPSTAD
// #[at_urc("+UUWAPSTAD")]
//
// 8.3 Ethernet link up +UUETHLU
// #[at_urc("+UUETHLU")]
// 8.4 Ethernet link down +UUETHLD
// #[at_urc("+UUETHLD")]
// #[at_urc("+UUWLD")]

// SocketDataAvailable(ip_transport_layer::urc::SocketDataAvailable),
// #[at_urc("+UUPSDD")]
// DataConnectionDeactivated(psn::urc::DataConnectionDeactivated),
// #[at_urc("+UUSOCL")]
// SocketClosed(ip_transport_layer::urc::SocketClosed),
// #[at_urc("+UMWI")]
// MessageWaitingIndication(sms::urc::MessageWaitingIndication),
}
