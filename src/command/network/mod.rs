//! ### 20 - GPIO Commands
//! The section describes the AT commands used to configure the GPIO pins provided by u-blox cellular modules
//! ### GPIO functions
//! On u-blox cellular modules, GPIO pins can be opportunely configured as general purpose input or output.
//! Moreover GPIO pins of u-blox cellular modules can be configured to provide custom functions via +UGPIOC
//! AT command. The custom functions availability can vary depending on the u-blox cellular modules series and
//! version: see Table 53 for an overview of the custom functions supported by u-blox cellular modules. \
//! The configuration of the GPIO pins (i.e. the setting of the parameters of the +UGPIOC AT command) is saved
//! in the NVM and used at the next power-on.
pub mod responses;
pub mod types;

use atat::atat_derive::AtatCmd;
use heapless::{consts, String};
use responses::*;
use types::*;
use no_std_net::IpAddr;


use super::NoResponse;

/// 10.1 Network host name +UNHN
///
/// Sets a new host name.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UNHN", NoResponse, timeout_ms = 10000)]
pub struct SetNetworkHostName{
    #[at_arg(position = 0)]
    pub host_name: String<consts::U64>,
}

/// 10.2 Network status +UNSTAT
///
/// Shows current status of the network interface id.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UNSTAT", NetworkStatusResponse, timeout_ms = 10000)]
pub struct GetNetworkStatus {
    #[at_arg(position = 0)]
    pub interface_id: u8,
    #[at_arg(position = 1)]
    pub status: NetworkStatus,
}

/// 10.3 Layer-2 routing +UNL2RCFG
///
/// Writes configuration for layer-2 routing.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UNL2RCFG", NoResponse, timeout_ms = 10000)]
pub struct Layer2Routing{
    #[at_arg(position = 0)]
    pub routing_tag: RoutingTag,
    #[at_arg(position = 1)]
    pub routing_value: OnOff,
}

/// 10.4 Bridge configuration +UBRGC
///
/// This command is used to configure a network bridge. After configuring a network bridge, it must be activated
/// using Bridge Configuration Action +UBRGCA command.
/// A bridge is used to connect two or more layers of two interfaces together. The bridge can also have a network
/// interface attached.
/// This command will generate an error if the bridge configuration is already active. Refer to Bridge
/// Configuration Action +UBRGCA command for instructions on how to deactivate a configuration.
/// ODIN-W2-SW3.0.x onwards
#[derive(Clone, AtatCmd)]
#[at_cmd("+UBRGC", NoResponse, timeout_ms = 10000)]
pub struct SetBridgeConfiguration{
    #[at_arg(position = 0)]
    pub config_id: BridgeConfigId,
    #[at_arg(position = 1)]
    pub config_tag: BridgeConfig,
}