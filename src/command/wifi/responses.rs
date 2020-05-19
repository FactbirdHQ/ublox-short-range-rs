//! Responses for System Commands
use super::types::*;
use atat::atat_derive::AtatResp;
use heapless::{consts, String};

/// 7.3 Scan +UWSCAN
#[derive(Clone, AtatResp)]
pub struct WifiScanResponse {
    #[at_arg(position = 0)]
    pub bssid: String<consts::U64>,
    #[at_arg(position = 1)]
    pub op_mode: OperationMode,
    #[at_arg(position = 2)]
    pub ssid: String<consts::U64>,
    #[at_arg(position = 3)]
    pub channel: u8,
    #[at_arg(position = 4)]
    pub rssi: i32,
    #[at_arg(position = 5)]
    pub authentication_suite: &[u8],
    #[at_arg(position = 6)]
    pub unicast_ciphers: &[u8], //Comes as hex..
    #[at_arg(position = 7)]
    pub group_ciphers: &[u8], //Comes as hex..
}


/// 7.5 Wi-Fi station status +UWSSTAT
#[derive(Clone, AtatResp)]
pub struct WifiStatusResponse {
    #[at_arg(position = 0)]
    pub status_id: StatusId,
    #[at_arg(position = 1)]
    pub status_val: i32,
}


/// 7.6 Wi-Fi Configuration +UWCFG
#[derive(Clone, AtatResp)]
pub struct WifiConfigResponse {
    #[at_arg(position = 0)]
    pub config_param: WifiConfigParameter,
    #[at_arg(position = 1)]
    pub config_value: ConfigValue,
}

/// 7.8 Wi-Fi Access point configuration +UWAPC
#[derive(Clone, AtatResp)]
pub struct WifiAPConfigResponse {
    #[at_arg(position = 0)]
    pub ap_id: AccessPointId,
    #[at_arg(position = 1)]
    pub ap_config_param: AccessPointConfig,
    #[at_arg(position = 2)]
    pub ap_config_val: AccessPointConfigValue,
}

/// 7.10 Wi-Fi Access point status +UWAPSTAT
#[derive(Clone, AtatResp)]
pub struct WifiAPStatusResponse {
    #[at_arg(position = 0)]
    pub ap_status_id: AccessPointStatusId,
    #[at_arg(position = 1)]
    pub ap_status_val: AccessPointConfigValue,
}

/// 7.11 Wi-Fi Access point station list +UWAPSTALIST
#[derive(Clone, AtatResp)]
pub struct WiFiAPStationListResponse {
    #[at_arg(position = 0)]
    pub id: u32,
    #[at_arg(position = 1)]
    pub mac_addr: String<consts::U64>,
    #[at_arg(position = 2)]
    pub rssi: i32,
}

/// 7.11 Wi-Fi Access point station list +UWAPSTALIST
#[derive(Clone, AtatResp)]
pub struct WifiMacResponse {
    #[at_arg(position = 0)]
    pub mac_addr: String<consts::U64>,
}