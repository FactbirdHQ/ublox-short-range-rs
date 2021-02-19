//! Unsolicited responses for Ethernet Commands
use super::types::*;
use crate::socket::SocketHandle;
use atat::atat_derive::AtatResp;
use heapless::{consts, String};
use no_std_net::IpAddr;

/// 8.3 Ethernet link up +UUETHLU
#[derive(Debug, PartialEq, Clone, AtatResp)]
pub struct EthernetLinkUp;

/// 8.4 Ethernet link down +UUETHLU
#[derive(Debug, PartialEq, Clone, AtatResp)]
pub struct EthernetLinkDown;
