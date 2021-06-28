#![cfg_attr(not(test), no_std)]

#[cfg(test)]
mod test_helpers;

mod client;
mod hex;

pub use atat;
pub use client::UbloxClient;

mod traits;

pub mod command;
pub mod error;
pub mod wifi;

#[cfg(any(feature = "socket-udp", feature = "socket-tcp"))]
mod socket;

#[cfg(any(feature = "socket-udp", feature = "socket-tcp"))]
pub use wifi::tls::TLS;

#[cfg(any(feature = "socket-udp", feature = "socket-tcp"))]
pub mod sockets {
    pub use crate::socket::*;
    pub use crate::wifi::socket::*;
}

/// Prelude - Include traits
pub mod prelude {
    #[cfg(any(feature = "wifi_ap"))]
    pub use crate::wifi::ap::WifiHotspot;
    #[cfg(any(feature = "wifi_sta"))]
    pub use crate::wifi::sta::WifiConnectivity;
    #[cfg(any(feature = "socket-udp", feature = "socket-tcp"))]
    pub use embedded_nal::{TcpClientStack, UdpClientStack};
}
