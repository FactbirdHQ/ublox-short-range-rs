#![cfg_attr(not(test), no_std)]

mod client;
mod hex;

pub use atat;
pub use client::UbloxClient;
use client::{URC_CAPACITY, URC_SUBSCRIBERS};

pub mod command;
pub mod config;
pub mod error;
pub mod wifi;

use command::edm::urc::EdmEvent;
#[cfg(any(feature = "socket-udp", feature = "socket-tcp"))]
pub use wifi::tls::TLS;

pub type UbloxWifiBuffers<const INGRESS_BUF_SIZE: usize> =
    atat::Buffers<EdmEvent, INGRESS_BUF_SIZE, URC_CAPACITY, URC_SUBSCRIBERS>;

pub type UbloxWifiIngress<'a, const INGRESS_BUF_SIZE: usize> = atat::Ingress<
    'a,
    command::custom_digest::EdmDigester,
    EdmEvent,
    INGRESS_BUF_SIZE,
    URC_CAPACITY,
    URC_SUBSCRIBERS,
>;

pub type UbloxWifiUrcChannel = atat::UrcChannel<EdmEvent, URC_CAPACITY, URC_SUBSCRIBERS>;
