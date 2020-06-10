//! Unsolicited responses for Data mode Commands
use crate::socket::SocketHandle;
use atat::atat_derive::AtatResp;
use heapless::{consts, String};
use super::types::*;
use no_std_net::IpAddr;

/// 5.10 Peer connected +UUDPC
#[derive(Clone, AtatResp)]
pub struct PeerConnected {
    #[at_arg(position = 0)]
    pub handle: u32,
    #[at_arg(position = 1)]
    pub connection_type: ConnectionType,
    #[at_arg(position = 2)]
    pub protocol: IPProtocol,
    #[at_arg(position = 3)]
    pub local_address: IpAddr,
    #[at_arg(position = 4)]
    pub local_port: u16,
    #[at_arg(position = 5)]
    pub remote_address: IpAddr,
    #[at_arg(position = 6)]
    pub remote_port: u16,
}

/// 5.11 Peer disconnected +UUDPD
#[derive(Clone, AtatResp)]
pub struct PeerDisconnected {
    #[at_arg(position = 0)]
    pub handle: u32,
}