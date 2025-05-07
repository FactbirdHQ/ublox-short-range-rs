use core::net::Ipv4Addr;

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

/// Static IP address configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticConfigV4 {
    /// IP address and subnet mask.
    pub address: Ipv4Addr,
    /// Default gateway.
    pub gateway: Option<Ipv4Addr>,
    /// DNS servers.
    pub dns_servers: DnsServers,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsServers {
    pub primary: Option<Ipv4Addr>,
    pub secondary: Option<Ipv4Addr>,
}

pub struct WifiConnection {
    pub wifi_state: WiFiState,
    pub ipv6_link_local_up: bool,
    pub ipv4_up: bool,
    #[cfg(feature = "ipv6")]
    pub ipv6_up: bool,
    pub network: Option<WifiNetwork>,
}

impl WifiConnection {
    pub(crate) const fn new() -> Self {
        WifiConnection {
            wifi_state: WiFiState::Inactive,
            ipv6_link_local_up: false,
            network: None,
            ipv4_up: false,
            #[cfg(feature = "ipv6")]
            ipv6_up: false,
        }
    }

    #[allow(dead_code)]
    pub fn is_station(&self) -> bool {
        self.network
            .as_ref()
            .map(|n| n.mode == WifiMode::Station)
            .unwrap_or_default()
    }

    #[allow(dead_code)]
    pub fn is_access_point(&self) -> bool {
        !self.is_station()
    }

    /// Get whether the network stack has a valid IP configuration.
    /// This is true if the network stack has a static IP configuration or if DHCP has completed
    pub fn is_config_up(&self) -> bool {
        let v6_up;
        let v4_up = self.ipv4_up;

        #[cfg(feature = "ipv6")]
        {
            v6_up = self.ipv6_up;
        }
        #[cfg(not(feature = "ipv6"))]
        {
            v6_up = false;
        }

        (v4_up || v6_up) && self.ipv6_link_local_up
    }

    pub fn is_connected(&self) -> bool {
        self.is_config_up() && self.wifi_state == WiFiState::Connected
    }
    pub fn reset(&mut self) {
        self.wifi_state = WiFiState::Inactive;
        self.ipv6_link_local_up = false;
        self.network = None;
        self.ipv4_up = false;
        #[cfg(feature = "ipv6")]
        {
            self.ipv6_up = false;
        }
    }
}
