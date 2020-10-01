use atat::AtatClient;
use embedded_hal::timer::{Cancel, CountDown};

use heapless::consts;

use crate::{
    client::UbloxClient,
    socket::{tcp::TcpSocket, Socket, SocketHandle, SocketSet},
    wifi::network::{WifiMode, WifiNetwork},
};

#[derive(Debug, PartialEq)]
pub enum WiFiState{
    Disconnecting,
    Disconnected,
    Connecting,
    Connected,
}

/// Describes whether device is connected to a network and has an IP or not.
/// It is possible to be attached to a network but have no Wifi connection.
#[derive(Debug, PartialEq)]
pub enum NetworkState{
    Attached,
    Unattached,
}

//Fold into wifi connectivity
pub struct WifiConnection
{
    pub wifi_state: WiFiState,
    pub network_state: NetworkState,
    pub network: WifiNetwork,
    pub config_id: u8,
    // pub (crate) sockets: SocketSet<consts::U8>,
}

impl WifiConnection
{
    pub(crate) fn new(network: WifiNetwork, wifi_state: WiFiState, config_id: u8) -> Self {
        WifiConnection {
            wifi_state: wifi_state,
            network_state: NetworkState::Unattached,
            network,
            // sockets: SocketSet::default(),
            config_id: config_id,
        }
    }

    pub fn is_station(&self) -> bool {
        self.network.mode == WifiMode::Station
    }

    pub fn is_access_point(&self) -> bool {
        !self.is_station()
    }
}
