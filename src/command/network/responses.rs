//! Responses for Network Commands
use super::types::*;
use atat::atat_derive::AtatResp;
use heapless::{consts, String};

/// 10.2 Network status +UNSTAT
#[derive(Clone, AtatResp)]
pub struct NetworkStatusResponse {
    #[at_arg(position = 0)]
    pub interface_id: u8,
    #[at_arg(position = 1)]
    pub status: NetworkStatus,
    #[at_arg(position = 3)]
    pub ipv6_status: Option<NetworkIpv6Status>,
}
