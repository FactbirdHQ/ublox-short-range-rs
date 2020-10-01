use atat::AtatClient;
use super::error::*;
use super::wifi::{
    connection::WifiConnection,
    network::WifiNetwork,
    options::{ConnectionOptions, HotspotOptions},
};

use embedded_hal::timer::{Cancel, CountDown};
use heapless::{Vec, consts, ArrayLength};

/// Wireless network connectivity functionality.
pub trait WifiConnectivity{
    /// Makes an attempt to connect to a selected wireless network with password specified.
    fn connect(self, options: ConnectionOptions) -> Result<(), WifiConnectionError>;

    fn scan(&mut self) -> Result<Vec<WifiNetwork, consts::U32>, WifiError>;

    fn disconnect(&mut self) -> Result<(), WifiConnectionError>;
}

pub trait WifiHotspot{
    /// Creates wireless hotspot service for host machine.
    fn create_hotspot(
        self,
        options: ConnectionOptions,
        configuration: HotspotOptions,
    ) -> Result<(), WifiHotspotError>;

    /// Stop serving a wireless network.
    ///
    /// **NOTE: All users connected will automatically be disconnected.**
    fn stop_hotspot(&mut self) -> Result<bool, WifiHotspotError>;
}
