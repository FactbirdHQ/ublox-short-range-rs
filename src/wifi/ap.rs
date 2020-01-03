use crate::{
  ATClient,
  error::WifiHotspotError,
  prelude::WifiHotspot,
  command::*,
  wifi::{
    network::{WifiNetwork, WifiMode},
    connection::WifiConnection,
    options::{ConnectionOptions, HotspotOptions},
  },
};

use embedded_hal::timer::CountDown;
use heapless::{String, Vec};

impl<T> WifiHotspot<T> for ATClient<T>
where
  T: CountDown,
{
  /// Creates wireless hotspot service for host machine.
  fn create_hotspot(
    self,
    options: ConnectionOptions,
    configuration: HotspotOptions,
  ) -> Result<WifiConnection<T>, WifiHotspotError> {
    let network = WifiNetwork {
      bssid: String::new(),
      op_mode: OPMode::AdHoc,
      ssid: options.ssid,
      channel: configuration.channel.unwrap() as u8,
      rssi: 1,
      authentication_suites: Vec::new(),
      unicast_ciphers: Vec::new(),
      group_ciphers: Vec::new(),
      mode: WifiMode::AccessPoint
    };
    Ok(WifiConnection::new(self, network))
  }

  /// Stop serving a wireless network.
  ///
  /// **NOTE: All users connected will automatically be disconnected.**
  fn stop_hotspot(&mut self) -> Result<bool, WifiHotspotError> {
    Ok(true)
  }
}
