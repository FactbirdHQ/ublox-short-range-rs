use super::command::*;
use super::error::*;
use super::wifi::{
    connection::WifiConnection,
    network::WifiNetwork,
    options::{ConnectionOptions, HotspotOptions},
};

use at::ATInterface;

use heapless::Vec;
use embedded_hal::timer::CountDown;

/// Wireless network connectivity functionality.
pub trait WifiConnectivity<T>: ATInterface<Command, ResponseType>
where
    T: CountDown
{
    /// Makes an attempt to connect to a selected wireless network with password specified.
    fn connect(
        self,
        options: ConnectionOptions,
    ) -> Result<WifiConnection<T>, WifiConnectionError>;

    fn scan(&mut self) -> Result<Vec<WifiNetwork, at::MaxResponseLines>, WifiError>;
}

pub trait WifiHotspot<T>: ATInterface<Command, ResponseType>
where
    T: CountDown,
{
    /// Creates wireless hotspot service for host machine.
    fn create_hotspot(
        self,
        options: ConnectionOptions,
        configuration: HotspotOptions,
    ) -> Result<WifiConnection<T>, WifiHotspotError>;

    /// Stop serving a wireless network.
    ///
    /// **NOTE: All users connected will automatically be disconnected.**
    fn stop_hotspot(&mut self) -> Result<bool, WifiHotspotError>;
}
