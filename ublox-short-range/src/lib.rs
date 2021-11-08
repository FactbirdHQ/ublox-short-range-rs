#![cfg_attr(not(test), no_std)]

mod client;
mod hex;

pub use atat;
pub use client::UbloxClient;

pub mod command;
pub mod config;
pub mod error;
pub mod wifi;

#[cfg(test)]
mod test_helper;

#[cfg(any(feature = "socket-udp", feature = "socket-tcp"))]
pub use wifi::tls::TLS;
