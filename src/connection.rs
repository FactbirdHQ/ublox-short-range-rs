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
    pub network: Option<WifiNetwork>,
    /// Number from 0-9. 255 used for unknown
    pub config_id: u8,
    /// Keeps track of activation of the config by driver
    pub activated: bool,
}

impl WifiConnection {
    pub(crate) const fn new() -> Self {
        WifiConnection {
            wifi_state: WiFiState::Inactive,
            network_up: false,
            network: None,
            config_id: 255,
            activated: false,
        }
    }

    pub fn is_station(&self) -> bool {
        match self.network {
            Some(ref n) => n.mode == WifiMode::Station,
            _ => false,
        }
    }

    pub fn is_access_point(&self) -> bool {
        !self.is_station()
    }

    pub fn is_connected(&self) -> bool {
        self.network_up
    }
}
