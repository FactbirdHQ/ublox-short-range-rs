//! Unsolicited responses for Data mode Commands
#[allow(unused_imports)]
use super::types::*;

/// 5.10 Peer connected +UUDPC
#[cfg(feature = "internal-network-stack")]
#[derive(Debug, PartialEq, Clone, atat::atat_derive::AtatResp)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct PeerConnected {
    #[at_arg(position = 0)]
    pub handle: ublox_sockets::PeerHandle,
    #[at_arg(position = 1)]
    pub connection_type: ConnectionType,
    #[at_arg(position = 2)]
    pub protocol: IPProtocol,
    // #[at_arg(position = 3)]
    // pub local_address: IpAddr,
    #[at_arg(position = 3)]
    #[cfg_attr(feature = "defmt", defmt(Debug2Format))]
    pub local_address: atat::heapless_bytes::Bytes<40>,
    #[at_arg(position = 4)]
    pub local_port: u16,
    // #[at_arg(position = 5)]
    // pub remote_address: IpAddr,
    #[at_arg(position = 5)]
    #[cfg_attr(feature = "defmt", defmt(Debug2Format))]
    pub remote_address: atat::heapless_bytes::Bytes<40>,
    #[at_arg(position = 6)]
    pub remote_port: u16,
}

/// 5.11 Peer disconnected +UUDPD
#[cfg(feature = "internal-network-stack")]
#[derive(Debug, PartialEq, Clone, atat::atat_derive::AtatResp)]
pub struct PeerDisconnected {
    #[at_arg(position = 0)]
    pub handle: ublox_sockets::PeerHandle,
}
