//! Unsolicited responses for Ethernet Commands
use atat::atat_derive::AtatResp;

/// 8.3 Ethernet link up +UUETHLU
#[derive(Debug, PartialEq, Clone, AtatResp)]
pub struct EthernetLinkUp;

/// 8.4 Ethernet link down +UUETHLU
#[derive(Debug, PartialEq, Clone, AtatResp)]
pub struct EthernetLinkDown;
