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

use super::NoResponse;

/// 3.2 Manufacturer identification +CGMI
///
/// Text string that identifies the manufacturer.
#[derive(Clone, AtatCmd)]
#[at_cmd("+CGMI", ManufacturerIdentificationResponse, timeout_ms = 10000)]
pub struct ManufacturerIdentification;

/// 3.3 Model identification +CGMM
///
/// Text string that identifies the model.
#[derive(Clone, AtatCmd)]
#[at_cmd("+CGMM", ModelIdentificationResponse, timeout_ms = 10000)]
pub struct ModelIdentification;

/// 3.4 Software version identification +CGMR
///
/// Text string that identifies the software version.
#[derive(Clone, AtatCmd)]
#[at_cmd("+CGMR", SoftwareVersionResponse, timeout_ms = 10000)]
pub struct SoftwareVersion;

/// 3.5 Serial number +CGSN
///
/// The product serial number.
#[derive(Clone, AtatCmd)]
#[at_cmd("+CGSN", SerialNumberResponse, timeout_ms = 10000)]
pub struct SerialNumber;

/// 3.6 Manufacturer identification +GMI
///
/// Text string that identifies the manufacturer.
#[derive(Clone, AtatCmd)]
#[at_cmd("+GMI", ManufacturerIdentificationResponse, timeout_ms = 10000)]
pub struct ManufacturerIdentification2;

/// 3.7 Software version identification +CGMR
///
/// Text string that identifies the software version.
#[derive(Clone, AtatCmd)]
#[at_cmd("+GMR", SoftwareVersionResponse, timeout_ms = 10000)]
pub struct SoftwareVersion2;

/// 3.8 Serial number +CGSN
///
/// The product serial number.
#[derive(Clone, AtatCmd)]
#[at_cmd("+GSN", SerialNumberResponse, timeout_ms = 10000)]
pub struct SerialNumber2;

/// 3.9 Identification information I0
///
/// Identificationinformation.
#[derive(Clone, AtatCmd)]
#[at_cmd("I0", IdentificationInfomationTypeCodeResponse, timeout_ms = 10000)]
pub struct IdentificationInfomationTypeCode {}

/// 3.9 Identification information I9
///
/// Identificationinformation.
#[derive(Clone, AtatCmd)]
#[at_cmd(
    "I9",
    IdentificationInfomationSoftwareVersionResponse,
    timeout_ms = 10000
)]
pub struct IdentificationInfomationSoftwareVersion {}

/// 3.9 Identification information I10
///
/// Identificationinformation.
#[derive(Clone, AtatCmd)]
#[at_cmd("I10", IdentificationInfomationMCUIDResponse, timeout_ms = 10000)]
pub struct IdentificationInfomationMCUID {}

/// 3.11 Set greeting text +CSGT
///
/// Sets the greeting text. Max 48 characters.
/// Configures and activates/deactivates the greeting text. The configuration change
/// in the greeting text will be applied at the subsequent boot. If active, the greeting
/// text is shown at boot once, on any AT interface, if the module start up mode is set to
/// command mode.
#[derive(Clone, AtatCmd)]
#[at_cmd("+CSGT", NoResponse, timeout_ms = 10000)]
pub struct SetGreetingText {
    #[at_arg(position = 0)]
    pub mode: Mode,
    #[at_arg(position = 1)]
    pub text: Option<String<consts::U64>>,
}
