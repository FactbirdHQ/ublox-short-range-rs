//! Responses for Data Mode 
use super::types::*;
use atat::atat_derive::AtatResp;
use heapless::{consts, String};

/// 5.2 Connect peer +UDCP
#[derive(Clone, AtatResp)]
pub struct ConnectPeerResponse {
    #[at_arg(position = 0)]
    pub peer_handle: u32,
}

/// 5.5 Peer list +UDLP
#[derive(Clone, AtatResp)]
pub struct PeerListResponse {
    #[at_arg(position = 0)]
    pub peer_handle: u32,
    #[at_arg(position = 1)]
    pub protocol: String<consts::U64>,
    #[at_arg(position = 2)]
    pub local_address: String<consts::U64>,
    #[at_arg(position = 3)]
    pub remote_address: String<consts::U64>,
}

/// 5.12 Bind +UDBIND
#[derive(Clone, AtatResp)]
pub struct BindResponse {
    #[at_arg(position = 0)]
    pub channel_id_1: u32,
    #[at_arg(position = 1)]
    pub channel_id_2: u32,
}
