//! AT Commands for U-Blox short range module family\
//! Following the [u-connect ATCommands Manual](https://www.u-blox.com/sites/default/files/u-connect-ATCommands-Manual_(UBX-14044127).pdf)

pub mod data_mode;
pub mod ethernet;
pub mod general;
pub mod gpio;
pub mod network;
pub mod ping;
pub mod security;
pub mod system;
pub mod wifi;
pub mod edm;
pub mod custom_digest;

use atat::atat_derive::{AtatCmd, AtatResp, AtatUrc};
use heapless::{consts, String};

#[derive(Debug, Clone, AtatResp, PartialEq)]
pub struct NoResponse;

#[derive(Debug, Clone, AtatCmd)]
#[at_cmd("", NoResponse, timeout_ms = 1000)]
pub struct AT;

#[derive(Debug, PartialEq, Clone, AtatUrc)]
pub enum Urc {
    /// 5.10 Peer connected +UUDPC
    #[at_urc("+UUDPC")]
    PeerConnected(data_mode::urc::PeerConnected),
    /// 5.11 Peer disconnected +UUDPD
    #[at_urc("+UUDPD")]
    PeerDisconnected(data_mode::urc::PeerDisconnected),
    /// 7.15 Wi-Fi Link connected +UUWLE
    #[at_urc("+UUWLE")]
    WifiLinkConnected(wifi::urc::WifiLinkConnected),
    /// 7.16 Wi-Fi Link disconnected +UUWLD
    #[at_urc("+UUWLD")]
    WifiLinkDisconnected(wifi::urc::WifiLinkDisconnected),
    /// 7.17 Wi-Fi Access point up +UUWAPU
    #[at_urc("+UUWAPU")]
    WifiAPUp(wifi::urc::WifiAPUp),
    /// 7.18 Wi-Fi Access point down +UUWAPD
    #[at_urc("+UUWAPD")]
    WifiAPDown(wifi::urc::WifiAPDown),
    /// 7.19 Wi-Fi Access point station connected +UUWAPSTAC
    #[at_urc("+UUWAPSTAC")]
    WifiAPStationConnected(wifi::urc::WifiAPStationConnected),
    /// 7.20 Wi-Fi Access point station disconnected +UUWAPSTAD
    #[at_urc("+UUWAPSTAD")]
    WifiAPStationDisconnected(wifi::urc::WifiAPStationDisconnected),
    /// 8.3 Ethernet link up +UUETHLU
    #[at_urc("+UUETHLU")]
    EthernetLinkUp(ethernet::urc::EthernetLinkUp),
    /// 8.4 Ethernet link down +UUETHLD
    #[at_urc("+UUETHLD")]
    EthernetLinkDown(ethernet::urc::EthernetLinkDown),
    /// 10.6 Network up +UUNU
    #[at_urc("+UUNU")]
    NetworkUp(network::urc::NetworkUp),
    /// 10.7 Network down +UUND
    #[at_urc("+UUND")]
    NetworkDown(network::urc::NetworkDown),
    /// 10.8 Network error +UUNERR
    #[at_urc("+UUNERR")]
    NetworkError(network::urc::NetworkError),
    #[at_urc("+UUPING")]
    PingResponse(ping::urc::PingResponse),
    #[at_urc("+UUPINGER")]
    PingErrorResponse(ping::urc::PingErrorResponse),
}
