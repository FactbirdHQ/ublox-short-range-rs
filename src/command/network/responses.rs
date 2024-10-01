//! Responses for Network Commands
use crate::command::wifi::types::AccessPointStatus;

use super::types::*;
use atat::atat_derive::AtatResp;

/// 7.10 WiFi AP status +UWAPSTAT
#[derive(Clone, AtatResp)]
pub struct APStatusResponse {
    pub status_val: AccessPointStatus,
}

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
