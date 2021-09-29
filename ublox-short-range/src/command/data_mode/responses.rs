//! Responses for Data Mode
use atat::atat_derive::AtatResp;
use heapless::String;
use ublox_sockets::SocketHandle;

/// 5.2 Connect peer +UDCP
#[derive(Clone, AtatResp)]
pub struct ConnectPeerResponse {
    #[at_arg(position = 0)]
    pub peer_handle: SocketHandle,
}

/// 5.5 Peer list +UDLP
#[derive(Clone, AtatResp)]
pub struct PeerListResponse {
    #[at_arg(position = 0)]
    pub peer_handle: usize,
    #[at_arg(position = 1)]
    pub protocol: String<64>,
    #[at_arg(position = 2)]
    pub local_address: String<64>,
    #[at_arg(position = 3)]
    pub remote_address: String<64>,
}

/// 5.12 Bind +UDBIND
#[derive(Clone, AtatResp)]
pub struct BindResponse {
    #[at_arg(position = 0)]
    pub channel_id_1: usize,
    #[at_arg(position = 1)]
    pub channel_id_2: usize,
}
