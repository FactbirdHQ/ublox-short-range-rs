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

/// 8.1 Ethernet configuration +UETHC
///
/// This command is used to set up an Ethernet configuration. After configuring the Ethernet, it must be activated
/// (Ethernet Configuration Action +UETHCA) before using.
/// The command will generate an error if the configuration is active. See "Ethernet Configuration Action
/// +UETHCA" for instructions on how to deactivate a configuration.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UETHC", NoResponse, timeout_ms = 10000)]
pub struct SetEthernetConfiguration{
    #[at_arg(position = 0)]
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
pub struct GetEthernetConfiguration{
    #[at_arg(position = 0)]
    pub param_tag: EthernetConfig,
}

/// 8.2 Ethernet configuration action +UETHCA
///
/// Sets network type.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UETHC", NoResponse, timeout_ms = 10000)]
pub struct EthernetConfigurationAction{
    #[at_arg(position = 0)]
    pub action: EthernetConfigAction,
}