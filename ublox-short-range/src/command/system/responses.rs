//! Responses for System Commands
use super::types::*;
use atat::atat_derive::AtatResp;
use heapless::{consts, String};

/// 4.11 Software update +UFWUPD
#[derive(Debug, PartialEq, Clone, AtatResp)]
pub struct SoftwareUpdateResponse {
    /// Should contain CCC and then the software updater boots up
    #[at_arg(position = 0)]
    pub serial_number: String<consts::U64>,
}

/// 4.14 Read Local address +UMLA
#[derive(Debug, PartialEq, Clone, AtatResp)]
pub struct LocalAddressResponse {
    /// MAC address of the interface id. If the address is set to 000000000000, the local
    /// address will be restored to factory-programmed value.
    #[at_arg(position = 0)]
    pub mac: String<consts::U64>,
}

/// 4.15 System status +UMSTAT
#[derive(Debug, PartialEq, Clone, AtatResp)]
pub struct SystemStatusResponse {
    #[at_arg(position = 0)]
    pub status_id: StatusID,
    #[at_arg(position = 1)]
    pub status_val: u32,
}
/// 4.19 LPO detection +UMLPO
#[derive(Debug, PartialEq, Clone, AtatResp)]
pub struct LPODetectionResponse {
    #[at_arg(position = 0)]
    pub lpo_detection: LPODetection,
}
