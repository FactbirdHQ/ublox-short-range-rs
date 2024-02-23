//! ### 10 - Network Commands
pub mod responses;
pub mod types;
pub mod urc;

use atat::atat_derive::AtatCmd;
use responses::*;
use types::*;

use super::NoResponse;

/// 10.1 Network host name +UNHN
///
/// Sets a new host name.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UNHN", NoResponse, timeout_ms = 1000)]
pub struct SetNetworkHostName<'a> {
    #[at_arg(position = 0, len = 64)]
    pub host_name: &'a str,
}

/// 10.2 Network status +UNSTAT
///
/// Shows current status of the network interface id.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UNSTAT", NetworkStatusResponse, timeout_ms = 3000)]
pub struct GetNetworkStatus {
    #[at_arg(position = 0)]
    pub interface_id: u8,
    #[at_arg(position = 1)]
    pub status: NetworkStatusParameter,
}

/// 10.3 Layer-2 routing +UNL2RCFG
///
/// Writes configuration for layer-2 routing.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UNL2RCFG", NoResponse, timeout_ms = 1000)]
pub struct Layer2Routing {
    #[at_arg(position = 0)]
    pub routing_tag: RoutingTag,
    #[at_arg(position = 1)]
    pub routing_value: bool,
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
#[at_cmd("+UBRGC", NoResponse, timeout_ms = 1000)]
pub struct SetBridgeConfiguration {
    #[at_arg(position = 0)]
    pub config_id: BridgeConfigId,
    #[at_arg(position = 1, len = 40)]
    pub config_tag: BridgeConfig,
}

/// 10.5 Bridge configuration action +UBRGCA
///
/// Executes an action for the network bridge configuration.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UBRGCA", NoResponse, timeout_ms = 1000)]
pub struct BridgeConfigurationAction {
    #[at_arg(position = 0)]
    pub config_id: BridgeConfigId,
    #[at_arg(position = 1)]
    pub action: BridgeAction,
}

/// 10.9 IPv4 address conflict detection timing +UNACDT
///
/// Sets parameters for IPv4 address conflict detection as described in RFC5227.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UNACDT", NoResponse, timeout_ms = 1000)]
pub struct AddressConflictDetectionTiming {
    #[at_arg(position = 0)]
    pub parameter: Timing,
}
