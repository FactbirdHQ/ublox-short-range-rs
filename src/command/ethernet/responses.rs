//! Responses for Ethernet
use super::types::*;
use atat::atat_derive::AtatResp;

/// 8.1 Ethernet configuration +UETHC
#[derive(Clone, AtatResp)]
pub struct EthernetConfigurationResponse {
    #[at_arg(position = 0)]
    pub param_tag: EthernetConfigR,
}
