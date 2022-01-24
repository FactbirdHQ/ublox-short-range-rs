//! Responses for WiFi Commands
use super::types::*;
use atat::atat_derive::AtatResp;
use atat::heapless_bytes::Bytes;
use heapless::Vec;

/// 7.1 Wi-Fi station configuration +UWSC
#[derive(Clone, AtatResp)]
pub struct GetWifiStationConfigResponse {
    #[at_arg(position = 0)]
    pub config_id: u8,
    #[at_arg(position = 1)]
    pub parameter: WifiStationConfigR,
}

/// 7.3 Scan +UWSCAN
#[derive(Clone, AtatResp)]
pub struct WifiScanResponse {
    #[at_arg(position = 0)]
    pub network_list: Vec<ScanedWifiNetwork, 32>,
}

/// 7.5 Wi-Fi station status +UWSSTAT
#[derive(Clone, AtatResp)]
pub struct WifiStatusResponse {
    #[at_arg(position = 0)]
    pub status_id: WifiStatus,
}

/// 7.6 Wi-Fi Configuration +UWCFG
#[derive(Clone, AtatResp)]
pub struct WifiConfigResponse {
    #[at_arg(position = 0)]
    pub config_param: WifiConfig,
}

/// 7.8 Wi-Fi Access point configuration +UWAPC
#[derive(Clone, AtatResp)]
pub struct WifiAPConfigResponse {
    #[at_arg(position = 0)]
    pub ap_id: AccessPointId,
    #[at_arg(position = 1)]
    pub ap_config_param: AccessPointConfigResponse,
}

/// 7.10 Wi-Fi Access point status +UWAPSTAT
#[derive(Clone, AtatResp)]
pub struct WifiAPStatusResponse {
    #[at_arg(position = 0)]
    pub ap_status_id: AccessPointStatus,
}

/// 7.11 Wi-Fi Access point station list +UWAPSTALIST
#[derive(Clone, AtatResp)]
pub struct WiFiAPStationListResponse {
    #[at_arg(position = 0)]
    pub id: u32,
    #[at_arg(position = 1)]
    pub mac_addr: Bytes<12>,
    #[at_arg(position = 2)]
    pub rssi: i32,
}

/// 7.11 Wi-Fi Access point station list +UWAPSTALIST
#[derive(Clone, AtatResp)]
pub struct WifiMacResponse {
    #[at_arg(position = 0)]
    pub mac_addr: Bytes<12>,
}
