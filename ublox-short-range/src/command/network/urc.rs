//! Unsolicited responses for Network Commands
use crate::socket::SocketHandle;
use atat::atat_derive::AtatResp;
use heapless::{consts, String};
use super::types::*;
use no_std_net::IpAddr;

/// 10.6 Network up +UUNU
#[derive(Debug, PartialEq, Clone, AtatResp)]
pub struct NetworkUp {
    #[at_arg(position = 0)]
    pub interface_id: u8,
}

/// 10.7 Network down +UUND
#[derive(Debug, PartialEq, Clone, AtatResp)]
pub struct NetworkDown {
    #[at_arg(position = 0)]
    pub interface_id: u8,
}

/// 10.8 Network error +UUNERR
#[derive(Debug, PartialEq, Clone, AtatResp)]
pub struct NetworkError {
    #[at_arg(position = 0)]
    pub interface_id: u8,
    #[at_arg(position = 1)]
    pub error: ErrorType,
}