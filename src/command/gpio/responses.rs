//! Responses for System Commands
use super::types::*;
use atat::atat_derive::AtatResp;
use heapless::{consts, String};



/// 14.2 GPIO Read +UGPIOR
#[derive(Clone, PartialEq, AtatResp)]
pub struct ReadGPIOResponse {
    #[at_arg(position = 0)]
    id: GPIOId,
    #[at_arg(position = 1)]
    value: GPIOValue,
}