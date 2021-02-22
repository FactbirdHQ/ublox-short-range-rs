#![no_std]
// #![allow(
//     dead_code,
//     unused_mut,
//     unused_variables,
//     unused_imports,
//     non_camel_case_types
// )]
//TODO: remove this ^ IMPORTANT

extern crate heapless;

#[macro_use]
extern crate nb;
extern crate no_std_net;
extern crate packed_struct;
#[macro_use]
extern crate packed_struct_codegen;

// pub type ATClient<T> = at::client::ATClient<
//     T,
//     command::RequestType,
//     heapless::consts::U5,
//     heapless::consts::U5,
// >;

#[cfg(test)]
#[macro_use]
mod test_helpers;

mod client;
mod hex;

pub use atat;
pub use client::UbloxClient;

mod traits;

pub mod command;
pub mod error;
// pub mod socket;
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
    pub use embedded_nal::{TcpClient, UdpClient};
}
