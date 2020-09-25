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
    EthernetUp,
}

//Fold into wifi connectivity
pub struct WifiConnection
{
    pub state: WiFiState,
    pub network: WifiNetwork,
    pub (crate) sockets: SocketSet<consts::U8>,
}

impl WifiConnection
{
    pub(crate) fn new(network: WifiNetwork, state: WiFiState) -> Self {
        WifiConnection {
            state: state,
            network,
            sockets: SocketSet::default(),
        }
    }

    // pub fn disconnect(mut self) -> Result<> {
    //     self.connected = false;
    //     self.sockets.prune();
    //     self.client
    // }

    // pub fn try_reconnect(&mut self) -> Result<&WifiNetwork, ()> {
    //     if self.connected {
    //         Ok(&self.network)
    //     } else {
    //         Err(())
    //     }
    // }

    pub fn is_station(&self) -> bool {
        self.network.mode == WifiMode::Station
    }

    pub fn is_access_point(&self) -> bool {
        !self.is_station()
    }

    // pub fn tcp_socket(&mut self) -> SocketHandle {
    //     let tcp_socket = TcpSocket::new();
    //     let socket = Socket::Tcp(tcp_socket);
    //     let h = self.sockets.add(socket);
    //     {
    //         let _socket = self.sockets.get::<TcpSocket>(h);
    //         // socket.connect((address, port), 49500).unwrap();
    //     }
    //     h
    // }
}
