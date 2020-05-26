//! Responses for System Commands
use super::types::*;
use atat::atat_derive::AtatResp;
use heapless::{consts, String};

/// 10.2 Network status +UNSTAT
#[derive(Clone, AtatResp)]
pub struct NetworkStatusResponse {
    #[at_arg(position = 0)]
    pub interface_id: u8,
    #[at_arg(position = 1)]
    pub status_tag: NetworkStatusTag,
    #[at_arg(position = 2)]
    pub status_value: NetworkStatusValue,
    #[at_arg(position = 3)]
    pub ipv6_status: Option<NetworkIpv6Status>,
}

