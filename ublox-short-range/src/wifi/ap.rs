use crate::{
    client::UbloxClient,
    command::wifi::types::OperationMode,
    error::WifiHotspotError,
    wifi::{
        network::{WifiMode, WifiNetwork},
        options::{ConnectionOptions, HotspotOptions},
    },
};
use atat::heapless_bytes::Bytes;
use atat::AtatClient;
use embedded_hal::digital::OutputPin;
use embedded_time::Clock;

pub trait WifiHotspot {
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

impl<C, CLK, RST, const N: usize, const L: usize> WifiHotspot for UbloxClient<C, CLK, RST, N, L>
where
    C: AtatClient,
    CLK: Clock,
    RST: OutputPin,
{
    /// Creates wireless hotspot service for host machine.
    fn create_hotspot(
        self,
        options: ConnectionOptions,
        configuration: HotspotOptions,
    ) -> Result<(), WifiHotspotError> {
        let _network = WifiNetwork {
            bssid: Bytes::new(),
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
