//! ### 20 - WiFi Commands
pub mod responses;
pub mod types;
pub mod urc;

use atat::atat_derive::AtatCmd;
use heapless::Vec;
use responses::*;
use types::*;

use super::NoResponse;

/// 7.1 Wi-Fi station configuration +UWSC
///
/// This command is used to configure up to 10 different Wi-Fi networks. After configuring a network, it must be
/// activated (Wi-Fi Station Configuration Action +UWSCA) before use.
/// If more than one configuration has active on start up parameter enabled, the behaviour is undefined.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UWSC", NoResponse, timeout_ms = 1000)]
pub struct SetWifiStationConfig {
    /// Wi-Fi configuration id. 0-9
    #[at_arg(position = 0)]
    pub config_id: u8,
    #[at_arg(position = 1)]
    pub config_param: WifiStationConfig,
}

/// 7.1 Wi-Fi station configuration +UWSC
///
/// This command is used to configure up to 10 different Wi-Fi networks. After configuring a network, it must be
/// activated (Wi-Fi Station Configuration Action +UWSCA) before use.
/// If more than one configuration has active on start up parameter enabled, the behaviour is undefined.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UWSC", GetWifiStationConfigResponse, timeout_ms = 1000)]
pub struct GetWifiStationConfig {
    /// Wi-Fi configuration id. 0-9
    #[at_arg(position = 0)]
    pub config_id: u8,
    #[at_arg(position = 1)]
    pub parameter: Option<WifiStationConfigParameter>,
}

/// 7.2 Wi-Fi station configuration action +UWSCA
/// Executes an action for the Wi-Fi network.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UWSCA", NoResponse, timeout_ms = 5000)]
pub struct ExecWifiStationAction {
    /// Wi-Fi configuration id. 0-9
    #[at_arg(position = 0)]
    pub config_id: u8,
    #[at_arg(position = 1)]
    pub action: WifiStationAction,
}

/// 7.3 Scan +UWSCAN
///
/// Scan the surroundings for network. This command will return the available networks
/// in the immediate surroundings, then return OK or ERROR if unable to start scan.
/// Channels scanned is given by the channel list. See +UWCL for more information. If
/// the SSID is defined, a directed scan will be performed.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UWSCAN", WifiScanResponse, timeout_ms = 1000)]
pub struct WifiScan<'a> {
    #[at_arg(position = 0, len = 64)]
    pub ssid: Option<&'a str>,
}

/// 7.4 Channel list +UWCL
///
/// Writes the required channel list for station mode.
/// Example: AT+UWCL=1,6,11
/// The channel list is restored to the default value by passing the command without
/// parameters: AT+UWCL
///
/// Note:
/// The actual channel list may differ from the wanted channel list. Depending on the physical location, the
/// radio environment, and the product version, the actual channel list in use may be limited to comply with
/// the regulatory approvals. Some sample scenarios are listed below:
/// - Channels 12 and 13 will be disabled until it has been determined that the module operates outside the
///   FCC region.
/// - Channels 120, 124, and 128 will be disabled until it has been determined that the module operates outside
///   the FCC region.
/// - Channels 149, 153, 157, 161, and 165 will be disabled until it has been determined that these are allowed
///   for the current region.
/// - Any DFS channel will be disabled for active use until an appropriate authoritative source has been found
///   for clearing each specific channel.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UWCL", WifiScanResponse, timeout_ms = 1000)]
pub struct SetChannelList {
    #[at_arg(position = 0)]
    pub channels: Vec<u8, 10>,
}

/// 7.5 Wi-Fi station status +UWSSTAT
///
/// Writes the required channel list for station mode.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UWSSTAT", WifiStatusResponse, timeout_ms = 1000)]
pub struct GetWifiStatus {
    /// Wi-Fi configuration id. 0-9
    #[at_arg(position = 0)]
    pub status_id: StatusId,
}

/// 7.6 Wi-Fi Configuration +UWCFG
///
/// Writes configuration parameter.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UWCFG", NoResponse, timeout_ms = 1000)]
pub struct SetWifiConfig {
    #[at_arg(position = 0)]
    pub config_param: WifiConfig,
}

/// 7.6 Wi-Fi Configuration +UWCFG
///
/// Reads configuration parameter.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UWCFG", WifiConfigResponse, timeout_ms = 1000)]
pub struct GetWifiConfig {
    #[at_arg(position = 0)]
    pub config_param: WifiConfigParameter,
}

/// 7.7 Wi-Fi Watchdog settings +UWWS
///
/// Writes watchdog parameters.
/// This command is deprecated and kept for backwards compatibility. Use +UDWS instead.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UWWS", NoResponse, timeout_ms = 1000)]
pub struct GetWatchdogConfig {
    #[at_arg(position = 0)]
    pub watchdog_setting: WatchdogSetting,
    #[at_arg(position = 1)]
    pub value: bool,
}

/// 7.8 Wi-Fi Access point configuration +UWAPC
///
/// This command is used to set up an access point network configuration. After configuring a network, it must
/// be activated (Wi-Fi Access Point Configuration Action +UWAPCA) before using.
/// The command will generate an error if the configuration id is active. See "Wi-Fi Access Point Configuration
/// Action +UWAPCA" for instructions on how to deactivate a configuration.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UWAPC", NoResponse, timeout_ms = 1000)]
pub struct SetWifiAPConfig {
    #[at_arg(position = 0)]
    pub ap_config_id: AccessPointId,
    #[at_arg(position = 1)]
    pub ap_config_param: AccessPointConfig,
}

/// 7.8 Wi-Fi Access point configuration +UWAPC
///
/// This command is used to set up an access point network configuration. After configuring a network, it must
/// be activated (Wi-Fi Access Point Configuration Action +UWAPCA) before using.
/// The command will generate an error if the configuration id is active. See "Wi-Fi Access Point Configuration
/// Action +UWAPCA" for instructions on how to deactivate a configuration.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UWAPC", WifiAPConfigResponse, timeout_ms = 1000)]
pub struct GetWifiAPConfig {
    #[at_arg(position = 0)]
    pub ap_id: AccessPointId,
    #[at_arg(position = 1)]
    pub ap_config_param: AccessPointConfigParameter,
}

/// 7.9 Wi-Fi Access point configuration action +UWAPCA
///
/// Executes an action for the Wi-Fi network.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UWAPCA", NoResponse, timeout_ms = 1000)]
pub struct WifiAPAction {
    #[at_arg(position = 0)]
    pub ap_config_id: AccessPointId,
    #[at_arg(position = 1)]
    pub ap_action: AccessPointAction,
}

/// 7.10 Wi-Fi Access point status +UWAPSTAT
///
/// Reads current status of the Wi-Fi interface.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UWAPSTAT", WifiAPStatusResponse, timeout_ms = 1000)]
pub struct WifiAPStatus {
    #[at_arg(position = 0)]
    pub ap_status_id: AccessPointStatusId,
}

/// 7.11 Wi-Fi Access point station list +UWAPSTALIST
///
/// Lists all the stations connected to the Wireless access point.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UWAPSTALIST?", WiFiAPStationListResponse, timeout_ms = 1000)]
pub struct WiFiAPStationList;

/// 7.12 Wi-Fi MAC address +UWAPMACADDR
///
/// Lists the currently used MAC address.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UWAPMACADDR", WifiMacResponse, timeout_ms = 1000)]
pub struct GetWifiMac;
