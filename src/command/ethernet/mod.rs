//! ### 8 - Ethernet
pub mod responses;
pub mod types;
pub mod urc;

use atat::atat_derive::AtatCmd;
use heapless::{consts, String};
use no_std_net::IpAddr;
use responses::*;
use types::*;

use super::NoResponse;

/// 8.1 Ethernet configuration +UETHC
///
/// This command is used to set up an Ethernet configuration. After configuring the Ethernet, it must be activated
/// (Ethernet Configuration Action +UETHCA) before using.
/// The command will generate an error if the configuration is active. See "Ethernet Configuration Action
/// +UETHCA" for instructions on how to deactivate a configuration.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UETHC", NoResponse, timeout_ms = 10000)]
pub struct SetEthernetConfiguration {
    #[at_arg(position = 0, len = 40)]
    pub param_tag: EthernetConfig,
}

/// 8.1 Ethernet configuration +UETHC
///
/// This command is used to set up an Ethernet configuration. After configuring the Ethernet, it must be activated
/// (Ethernet Configuration Action +UETHCA) before using.
/// The command will generate an error if the configuration is active. See "Ethernet Configuration Action
/// +UETHCA" for instructions on how to deactivate a configuration.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UETHC", EthernetConfigurationResponse, timeout_ms = 10000)]
pub struct GetEthernetConfiguration {
    #[at_arg(position = 0)]
    pub param_tag: EthernetConfigParameter,
}

/// 8.2 Ethernet configuration action +UETHCA
///
/// Sets network type.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UETHC", NoResponse, timeout_ms = 10000)]
pub struct EthernetConfigurationAction {
    #[at_arg(position = 0)]
    pub action: EthernetConfigAction,
}
