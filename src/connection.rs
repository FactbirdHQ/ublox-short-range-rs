use crate::network::{WifiMode, WifiNetwork};

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum WiFiState {
    Inactive,
    /// Searching for Wifi
    NotConnected,
    Connected,
}

// Fold into wifi connectivity
pub struct WifiConnection {
    /// Keeps track of connection state on module
    pub wifi_state: WiFiState,
    pub network_up: bool,
    pub network: WifiNetwork,
    /// Number from 0-9. 255 used for unknown
    pub config_id: u8,
    /// Keeps track of activation of the config by driver
    pub activated: bool,
}

impl WifiConnection {
    pub(crate) fn new(network: WifiNetwork, wifi_state: WiFiState, config_id: u8) -> Self {
        WifiConnection {
            wifi_state,
            network_up: false,
            network,
            config_id,
            activated: false,
        }
    }

    pub fn is_station(&self) -> bool {
        self.network.mode == WifiMode::Station
    }

    pub fn is_access_point(&self) -> bool {
        !self.is_station()
    }

    pub(crate) fn activate(mut self) -> Self {
        self.activated = true;
        self
    }

    pub(crate) fn deactivate(&mut self) {
        self.activated = false;
    }
}
