#![no_std]

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

#[cfg(test)]
mod tests {
    //! This module is required in order to satisfy the requirements of defmt, while running tests.
    //! Note that this will cause all log `defmt::` log statements to be thrown away.
    use core::ptr::NonNull;
    #[defmt::global_logger]
    struct Logger;
    impl defmt::Write for Logger {
        fn write(&mut self, _bytes: &[u8]) {}
    }
    unsafe impl defmt::Logger for Logger {
        fn acquire() -> Option<NonNull<dyn defmt::Write>> {
            Some(NonNull::from(&Logger as &dyn defmt::Write))
        }
        unsafe fn release(_: NonNull<dyn defmt::Write>) {}
    }
    defmt::timestamp!("");
    #[export_name = "_defmt_panic"]
    fn panic() -> ! {
        panic!()
    }
}
