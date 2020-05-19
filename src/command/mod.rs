//! AT Commands for U-Blox short range module family\
//! Following the [u-connect ATCommands Manual](https://www.u-blox.com/sites/default/files/u-connect-ATCommands-Manual_(UBX-14044127).pdf)

// pub mod control;
// pub mod device_data_security;
// pub mod device_lock;
// pub mod dns;
pub mod general;
// pub mod gpio;
// pub mod ip_transport_layer;
// pub mod mobile_control;
// pub mod network_service;
// pub mod psn;
// pub mod sms;
// pub mod system_features;

use atat::atat_derive::{AtatCmd, AtatResp, AtatUrc};

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
// #[at_urc("+UUWLD")]
// #[at_urc("+UUWLD")]
// #[at_urc("+UUWLD")]

// SocketDataAvailable(ip_transport_layer::urc::SocketDataAvailable),
// #[at_urc("+UUPSDD")]
// DataConnectionDeactivated(psn::urc::DataConnectionDeactivated),
// #[at_urc("+UUSOCL")]
// SocketClosed(ip_transport_layer::urc::SocketClosed),
// #[at_urc("+UMWI")]
// MessageWaitingIndication(sms::urc::MessageWaitingIndication),
}
