use no_std_net::{Ipv4Addr, Ipv6Addr};

use crate::network::{WifiMode, WifiNetwork};

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum WiFiState {
    Inactive,
    /// Searching for Wifi
    NotConnected,
    SecurityProblems,
    Connected,
}

pub struct WifiConnection {
    pub wifi_state: WiFiState,
    pub network_up: bool,
    pub ipv4: Option<Ipv4Addr>,
    pub ipv6: Option<Ipv6Addr>,
    pub network: Option<WifiNetwork>,
}

impl WifiConnection {
    pub(crate) const fn new() -> Self {
        WifiConnection {
            wifi_state: WiFiState::Inactive,
            network_up: false,
            network: None,
            ipv4: None,
            ipv6: None,
        }
    }

    #[allow(dead_code)]
    pub fn is_station(&self) -> bool {
        match self.network {
            Some(ref n) => n.mode == WifiMode::Station,
            _ => false,
        }
    }

    #[allow(dead_code)]
    pub fn is_access_point(&self) -> bool {
        !self.is_station()
    }

    pub fn is_connected(&self) -> bool {
        self.network_up && self.wifi_state == WiFiState::Connected
    }
}