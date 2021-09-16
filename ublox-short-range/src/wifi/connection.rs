use crate::wifi::network::{WifiMode, WifiNetwork};

#[derive(Debug, Clone, Copy, PartialEq, defmt::Format)]
pub enum WiFiState {
    /// Disconnected, Wifi off
    Inactive,
    /// Searching for Wifi
    NotConnected,
    Connected,
}

/// Describes whether device is connected to a network and has an IP or not.
/// It is possible to be attached to a network but have no Wifi connection.
#[derive(Debug, PartialEq, defmt::Format)]
pub enum NetworkState {
    Attached,
    AlmostAttached,
    Unattached,
}

// Fold into wifi connectivity
pub struct WifiConnection {
    pub wifi_state: WiFiState,
    pub network_state: NetworkState,
    pub network: WifiNetwork,
    pub config_id: u8,
}

impl WifiConnection {
    pub(crate) fn new(network: WifiNetwork, wifi_state: WiFiState, config_id: u8) -> Self {
        WifiConnection {
            wifi_state,
            network_state: NetworkState::Unattached,
            network,
            config_id,
        }
    }

    pub(crate) fn is_connected(&self) -> bool {
        self.network_state == NetworkState::Attached && self.wifi_state == WiFiState::Connected
    }

    pub fn is_station(&self) -> bool {
        self.network.mode == WifiMode::Station
    }

    pub fn is_access_point(&self) -> bool {
        !self.is_station()
    }
}
