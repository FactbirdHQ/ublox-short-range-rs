//! Responses for System Commands
use super::types::*;
use atat::atat_derive::AtatResp;
use heapless::{consts, String};

/// 8.1 Ethernet configuration +UETHC
#[derive(Clone, AtatResp)]
pub struct EthernetConfigurationResponse {
    #[at_arg(position = 0)]
    pub param_tag: EthernetConfigTag,
    #[at_arg(position = 1)]
    pub param_val: EthernetConfigValue,
}

