use atat::AtatClient;
use crate::{
    client::UbloxClient,
    command::wifi::types::{OperationMode, Authentication},
    error::WifiHotspotError,
    prelude::WifiHotspot,
    wifi::{
        connection::WifiConnection,
        network::{WifiMode, WifiNetwork},
        options::{ConnectionOptions, HotspotOptions},
    },
};

use embedded_hal::timer::{Cancel, CountDown};
use heapless::{String, ArrayLength};

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

impl<T, N, L> WifiHotspot for UbloxClient<T, N, L>
where
    T: AtatClient,
    N: ArrayLength<Option<crate::sockets::SocketSetItem<L>>>,
    L: ArrayLength<u8>,
{
    /// Creates wireless hotspot service for host machine.
    fn create_hotspot(
        self,
        options: ConnectionOptions,
        configuration: HotspotOptions,
    ) -> Result<(), WifiHotspotError> {
        let network = WifiNetwork {
            bssid: String::new(),
            op_mode: OperationMode::AdHoc,
            ssid: options.ssid,
            channel: configuration.channel.unwrap() as u8,
            rssi: 1,
            authentication_suites: 0,
            unicast_ciphers: 0,
            group_ciphers: 0,
            mode: WifiMode::AccessPoint,
        };
        // self.wifi_connection.set(Some(WifiConnection::new(network)));
        Ok(())
    }

    /// Stop serving a wireless network.
    ///
    /// **NOTE: All users connected will automatically be disconnected.**
    fn stop_hotspot(&mut self) -> Result<bool, WifiHotspotError> {
        Ok(true)
    }
}
