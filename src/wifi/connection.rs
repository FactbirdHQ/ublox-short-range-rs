use embedded_hal::timer::CountDown;

use heapless::consts;

use crate::{
    socket::{tcp::TcpSocket, Socket, SocketHandle, SocketSet},
    wifi::network::{WifiMode, WifiNetwork},
    ATClient
};

pub enum SerialMode {
    Cmd,
    Data,
    ExtendedData,
}

pub struct WifiConnection<T>
where
    T: CountDown,
{
    pub connected: bool,
    network: WifiNetwork,
    client: ATClient<T>,
    serial_mode: SerialMode,
    sockets: SocketSet<consts::U8>,
}

impl<T> WifiConnection<T>
where
    T: CountDown,
{
    pub fn new(client: ATClient<T>, network: WifiNetwork) -> Self {
        WifiConnection {
            connected: true,
            client,
            network,
            serial_mode: SerialMode::Cmd,
            sockets: SocketSet::default(),
        }
    }

    pub fn disconnect(mut self) -> ATClient<T> {
        self.connected = false;
        self.sockets.prune();
        self.client
    }

    pub fn try_reconnect(&mut self) -> Result<&WifiNetwork, ()> {
        if self.connected {
            Ok(&self.network)
        } else {
            Err(())
        }
    }

    pub fn is_station(&self) -> bool {
        self.network.mode == WifiMode::Station
    }

    pub fn is_access_point(&self) -> bool {
        !self.is_station()
    }

    pub fn tcp_socket(&mut self) -> SocketHandle {
        let tcp_socket = TcpSocket::new();
        let socket = Socket::Tcp(tcp_socket);
        let h = self.sockets.add(socket);
        {
            let _socket = self.sockets.get::<TcpSocket>(h);
            // socket.connect((address, port), 49500).unwrap();
        }
        h
    }
}
