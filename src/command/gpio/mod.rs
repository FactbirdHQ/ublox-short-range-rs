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

/// 14.1 GPIO Configuration +UGPIOC
///
/// Configures the GPIOs as input or output, pull up or pull down resistors when
/// applicable, and modifies its value.
/// Note: Before changing a GPIO from input to output or vice versa, the GPIO must be
/// disabled.
/// Supported by ODIN-W2 from software version 3.0.0 onwards only.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UGPIOC", NoResponse, timeout_ms = 10000)]
pub struct ConfigureGPIO{
    #[at_arg(position = 0)]
    pub id: GPIOId,
    #[at_arg(position = 1)]
    pub mode: GPIOMode,
}

/// 14.2 GPIO Read +UGPIOR
///
/// Reads the current value of an enabled GPIO pin, independent of input or output
/// configuration.
/// Supported by ODIN-W2 from software version 3.0.0 onwards only.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UGPIOR", ReadGPIOResponse, timeout_ms = 10000)]
pub struct ReadGPIO{
    #[at_arg(position = 0)]
    pub id: GPIOId,
}

/// 14.3 GPIO Write +UGPIOW
///
/// Writes the value of an enabled GPIO pin configured as output.
/// Supported by ODIN-W2 from software version 3.0.0 onwards only.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UGPIOW", NoResponse, value_sep = false, timeout_ms = 10000)]
pub struct WriteGPIO{
    #[at_arg(position = 0)]
    pub id: GPIOId,
    #[at_arg(position = 1)]
    pub value: GPIOValue,
}