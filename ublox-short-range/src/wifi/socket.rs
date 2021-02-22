// Handles reciving data from sockets
// implements TCP and UDP for WiFi client

// use embedded_hal::digital::v2::OutputPin;
pub use embedded_nal::{AddrType, IpAddr, SocketAddr, SocketAddrV4, SocketAddrV6, nb};
use heapless::{consts, ArrayLength, String};
pub use no_std_net::{Ipv4Addr, Ipv6Addr};
// use serde::{Serialize};
use atat::serde_at::{to_string, SerializeOptions};

use crate::command::data_mode::{types::*, *};
use crate::command::edm::{EdmAtCmdWrapper, EdmDataCommand};
use crate::UbloxClient;
use crate::{error::Error, socket, socket::ChannelId};
use typenum::marker_traits::Unsigned;

use crate::{
    hex,
    socket::{SocketHandle, SocketType},
};

#[cfg(feature = "socket-udp")]
use crate::socket::{UdpSocket, UdpState};
#[cfg(feature = "socket-udp")]
use embedded_nal::UdpClient;

#[cfg(feature = "socket-tcp")]
use crate::socket::{TcpSocket, TcpState};
#[cfg(feature = "socket-tcp")]
use embedded_nal::TcpClient;

pub type IngressChunkSize = consts::U256;
pub type EgressChunkSize = consts::U512;

impl<C, N, L> UbloxClient<C, N, L>
where
    C: atat::AtatClient,
    N: ArrayLength<Option<crate::sockets::SocketSetItem<L>>>,
    L: ArrayLength<u8>,
{
    /// Helper function to manage the internal poll counter, used to poll open
    /// sockets for incoming data, in case a `SocketDataAvailable` URC is missed
    /// once in a while, as the ublox module will never send the URC again, if
    /// the socket is not read.
    // pub(crate) fn poll_cnt(&self, reset: bool) -> u16 {
    //     // if reset {
    //     //     // Reset poll_cnt
    //     //     self.poll_cnt.set(0);
    //     //     0
    //     // } else {
    //     //     // Increment poll_cnt by one, and return the old value
    //     //     let old = self.poll_cnt.get();
    //     //     self.poll_cnt.set(old + 1);
    //     //     old
    //     // }
    //     0
    // }

    pub(crate) fn handle_socket_error<A: atat::AtatResp, F: Fn() -> Result<A, Error>>(
        &self,
        f: F,
        socket: Option<SocketHandle>,
        attempt: u8,
    ) -> Result<A, Error> {
        match f() {
            Ok(r) => Ok(r),
            Err(e @ Error::AT(atat::Error::Timeout)) => {
                if attempt < 3 {
                    defmt::error!("[RETRY] Retrying! {:?}", attempt);
                    self.handle_socket_error(f, socket, attempt + 1)
                } else {
                    Err(e)
                }
            }
            Err(e @ Error::AT(atat::Error::InvalidResponse)) => {
                // let SocketErrorResponse { error } = self
                //     .send_internal(&GetSocketError, false)
                //     .unwrap_or_else(|_e| SocketErrorResponse { error: 110 });

                // if error != 0 {
                if let Some(handle) = socket {
                    let mut sockets = self.sockets.try_borrow_mut()?;
                    match sockets.socket_type(handle) {
                        Some(SocketType::Tcp) => {
                            let mut tcp = sockets.get::<TcpSocket<_>>(handle)?;
                            tcp.close();
                        }
                        Some(SocketType::Udp) => {
                            let mut udp = sockets.get::<UdpSocket<_>>(handle)?;
                            udp.close();
                        }
                        None => {}
                    }
                    sockets.remove(handle)?;
                }
                Err(e)
            }
            Err(e) => Err(e),
        }
    }

    pub(crate) fn socket_ingress(
        &self,
        channel_id: ChannelId,
        data: &[u8],
    ) -> Result<usize, Error> {
        if data.len() == 0 {
            return Ok(0);
        }

        // Allow room for 2x length (Hex), and command overhead
        // let chunk_size = core::cmp::min(data.len(), IngressChunkSize::to_usize());
        let mut sockets = self
            .sockets
            .try_borrow_mut()?;

        // Reset poll_cnt
        // self.poll_cnt(true);

        match sockets.socket_type_by_channel_id(channel_id) {
            Some(SocketType::Tcp) => {
                // Handle tcp socket
                let mut tcp = sockets.get_by_channel::<TcpSocket<_>>(channel_id)?;
                if !tcp.can_recv() {
                    return Err(Error::Busy);
                }

                Ok(tcp.rx_enqueue_slice(data))
            }
            Some(SocketType::Udp) => {
                // Handle udp socket
                let mut udp = sockets.get_by_channel::<UdpSocket<_>>(channel_id)?;

                if !udp.can_recv() {
                    return Err(Error::Busy);
                }
                Ok(udp.rx_enqueue_slice(data))
            }
            _ => {
                defmt::error!("SocketNotFound {:?}", channel_id);
                Err(Error::SocketNotFound)
            }
        }
    }
}

#[cfg(feature = "socket-udp")]
impl<C, N, L> UdpClient for UbloxClient<C, N, L>
where
    C: atat::AtatClient,
    N: ArrayLength<Option<crate::sockets::SocketSetItem<L>>>,
    L: ArrayLength<u8>,
{
    type Error = Error;

    // Only return a SocketHandle to reference into the SocketSet owned by the GsmClient,
    // as the Socket object itself provides no value without accessing it though the client.
    type UdpSocket = SocketHandle;

    fn socket(&self) -> Result<Self::UdpSocket, Self::Error>{
        defmt::debug!("[UDP] Opening socket");
        if let Some(ref con) = *self.wifi_connection.try_borrow()? {
            if !self.initialized.get() || !con.is_connected() {
                return Err(Error::Network);
            }
        } else {
            return Err(Error::Network);
        }
        if let Ok(mut sockets) = self.sockets.try_borrow_mut() {
            if let Ok(h) = sockets.add(UdpSocket::new(0)) {
                Ok(h)
            } else {
                defmt::error!("[UDP] Opening socket Error: Socket set full");
                Err(Error::Network)
            }
        } else {
            defmt::error!("[UDP] Opening socket Error: Unable to borrow sockets");
            Err(Error::Network)
        }
    }

    /// Connect a UDP socket with a peer using a dynamically selected port.
    /// Selects a port number automatically and initializes for read/writing.
    fn connect(&self, socket: &mut Self::UdpSocket, remote: SocketAddr) -> Result<(), Self::Error> {
        defmt::debug!("[UDP] Connecting socket");
        if let Some(ref con) = *self
            .wifi_connection
            .try_borrow()?
        {
            if !self.initialized.get() || !con.is_connected() {
                return Err(Error::Network);
            }
        } else {
            return Err(Error::Network);
        }

        {
            let mut url = String::<consts::U128>::from("udp://");
            let dud = String::<consts::U1>::new();
            let mut workspace = String::<consts::U43>::new();
            let mut ip_str = String::<consts::U43>::from("[");
            let mut port = String::<consts::U8>::new();

            match remote.ip() {
                IpAddr::V4(ip) => {
                    ip_str = to_string(
                        &ip,
                        String::<consts::U1>::new(),
                        SerializeOptions {
                            value_sep: false,
                            cmd_prefix: &"",
                            termination: &"",
                        },
                    )
                    .map_err(|_e| Self::Error::BadLength)?;
                    url.push_str(&ip_str[1..ip_str.len() - 1])
                        .map_err(|_e| Self::Error::BadLength)?;
                }
                IpAddr::V6(ip) => {
                    workspace = to_string(
                        &ip,
                        String::<consts::U1>::new(),
                        SerializeOptions::default(),
                    )
                    .map_err(|_e| Self::Error::BadLength)?;

                    ip_str
                        .push_str(&workspace[1..workspace.len() - 1])
                        .map_err(|_e| Self::Error::BadLength)?;
                    ip_str.push(']').map_err(|_e| Self::Error::BadLength)?;
                    url.push_str(&ip_str).map_err(|_e| Self::Error::BadLength)?;
                }
            }
            url.push(':').map_err(|_e| Self::Error::BadLength)?;

            port = to_string(
                &remote.port(),
                String::<consts::U1>::new(),
                SerializeOptions::default(),
            )
            .map_err(|_e| Self::Error::BadLength)?;
            url.push_str(&port).map_err(|_e| Self::Error::BadLength)?;
            url.push('/').map_err(|_e| Self::Error::BadLength)?;

            let mut sockets = self.sockets.try_borrow_mut()?;
            defmt::trace!("[UDP] Connecting URL! {:str}", url);
            match self.handle_socket_error(
                || self.send_internal(&EdmAtCmdWrapper(ConnectPeer { url: &url }), false),
                None,
                0,
            ) {
                Ok(resp) => {
                    let handle = SocketHandle(resp.peer_handle);
                    let mut udp = sockets.get::<UdpSocket<_>>(*socket)?;
                    udp.endpoint = remote;
                    udp.meta.handle = handle;
                    *socket = handle;
                }
                Err(e) => return Err(e)
            }
        }
        while {
            let mut sockets = self.sockets.try_borrow_mut()?;
            let mut udp = sockets.get::<UdpSocket<_>>(*socket)?;
            udp.state() == UdpState::Closed
        } {
            self.spin()?;
        }
        Ok(())
    }

    /// Send a datagram to the remote host.
    fn send(&self, socket: &mut Self::UdpSocket, buffer: &[u8]) -> nb::Result<(), Self::Error> {
        if let Some(ref con) = *self
            .wifi_connection
            .try_borrow()
            .map_err(|e| nb::Error::Other(e.into()))?
        {
            if !self.initialized.get() || !con.is_connected() {
                return Err(nb::Error::Other(Error::Network));
            }
        } else {
            return Err(nb::Error::Other(Error::Network));
        }

        let mut sockets = self
            .sockets
            .try_borrow_mut()
            .map_err(|e| nb::Error::Other(e.into()))?;

        let mut udp = sockets
            .get::<UdpSocket<_>>(*socket)
            .map_err(|e| nb::Error::Other(e.into()))?;
        
        if !udp.is_open() {
            return Err(nb::Error::Other(Error::SocketClosed));
        }

        for chunk in buffer.chunks(EgressChunkSize::to_usize()) {
            self.handle_socket_error(
                || {
                    self.send_internal(
                        &EdmDataCommand {
                            channel: udp.channel_id().0,
                            data: chunk,
                        },
                        true,
                    )
                },
                Some(*socket),
                0,
            )?;
        }
        Ok(())
    }

    /// Read a datagram the remote host has sent to us. Returns `Ok(n)`, which
    /// means a datagram of size `n` has been received and it has been placed
    /// in `&buffer[0..n]`, or an error.
    fn receive(
        &self,
        socket: &mut Self::UdpSocket,
        buffer: &mut [u8],
    ) -> nb::Result<(usize, SocketAddr), Self::Error> {
        // self.spin()?;

        let mut sockets = self
            .sockets
            .try_borrow_mut()
            .map_err(|e| nb::Error::Other(e.into()))?;

        let mut udp = sockets
            .get::<UdpSocket<_>>(*socket)
            .map_err(|e| nb::Error::Other(Error::Socket(e)))?;

        let us = udp.recv_slice(buffer)
            .map_err(|e| nb::Error::Other(e.into()))?;
        Ok((us, udp.endpoint()))
    }

    /// Close an existing UDP socket.
    fn close(&self, socket: Self::UdpSocket) -> Result<(), Self::Error> {
        defmt::debug!("[UDP] Closelosing socket: {:?}", socket.0);
        let mut sockets = self.sockets.try_borrow_mut()?;
        //If no sockets excists, nothing to close.
        if let Some(ref mut udp) = sockets.get::<UdpSocket<_>>(socket).ok() {
            self.handle_socket_error(
                || {
                    self.send_at(ClosePeerConnection {
                        peer_handle: udp.handle().0,
                    })
                },
                Some(socket),
                0,
            )?;
            udp.close();
        }

        Ok(())
    }
}

#[cfg(feature = "socket-tcp")]
impl<C, N, L> TcpClient for UbloxClient<C, N, L>
where
    C: atat::AtatClient,
    N: ArrayLength<Option<crate::sockets::SocketSetItem<L>>>,
    L: ArrayLength<u8>,
{
    type Error = Error;

    // Only return a SocketHandle to reference into the SocketSet owned by the GsmClient,
    // as the Socket object itself provides no value without accessing it though the client.
    type TcpSocket = SocketHandle;

    /// Open a new TCP socket to the given address and port. The socket starts in the unconnected state.
    fn socket(&self) -> Result<Self::TcpSocket, Self::Error> {
        defmt::debug!("[TCP] Opening socket");
        if let Some(ref con) = *self.wifi_connection.try_borrow()? {
            if !self.initialized.get() || !con.is_connected() {
                return Err(Error::Network);
            }
        } else {
            return Err(Error::Network);
        }
        if let Ok(mut sockets) = self.sockets.try_borrow_mut() {
            if let Ok(h) = sockets.add(TcpSocket::new(0)) {
                Ok(h)
            } else {
                defmt::error!("[TCP] Opening socket Error: Socket set full");
                Err(Error::Network)
            }
        } else {
            defmt::error!("[TCP] Opening socket Error: Not able to borrow sockets");
            Err(Error::Network)
        }
    }

    /// Connect to the given remote host and port.
    fn connect(
        &self,
        socket: &mut Self::TcpSocket,
        remote: SocketAddr,
    ) -> nb::Result<(), Self::Error> {
        defmt::debug!("[TCP] Connect socket: {:?}", socket.0);
        if let Some(ref con) = *self
            .wifi_connection
            .try_borrow()
            .map_err(|e| nb::Error::Other(e.into()))?
        {
            if !self.initialized.get() || !con.is_connected() {
                return Err(nb::Error::Other(Error::Network));
            }
        } else {
            return Err(nb::Error::Other(Error::Network));
        }

        {
            let mut sockets = self.sockets
                .try_borrow_mut()
                .map_err(Self::Error::from)?;
                // .map_err(|e| nb::Error::Other(e.into()))?;
            //If no socket is found we stop here
            let mut tcp = sockets.get::<TcpSocket<_>>(*socket)
                .map_err(Self::Error::from)?;
                // .map_err(|e| nb::Error::Other(e.into()))?;

            //TODO: Optimize! and when possible rewrite to ufmt!
            let mut url = String::<consts::U128>::from("tcp://");
            let dud = String::<consts::U1>::new();
            let mut workspace = String::<consts::U43>::new();
            let mut ip_str = String::<consts::U43>::from("[");
            let mut port = String::<consts::U8>::new();

            match remote.ip() {
                IpAddr::V4(ip) => {
                    ip_str = to_string(
                        &ip,
                        String::<consts::U1>::new(),
                        SerializeOptions {
                            value_sep: false,
                            cmd_prefix: &"",
                            termination: &"",
                        },
                    )
                    .map_err(|_e| Self::Error::BadLength)?;
                    url.push_str(&ip_str[1..ip_str.len() - 1])
                        .map_err(|_e| Self::Error::BadLength)?;
                }
                IpAddr::V6(ip) => {
                    workspace = to_string(
                        &ip,
                        String::<consts::U1>::new(),
                        SerializeOptions::default(),
                    )
                    .map_err(|_e| Self::Error::BadLength)?;

                    ip_str
                        .push_str(&workspace[1..workspace.len() - 1])
                        .map_err(|_e| Self::Error::BadLength)?;
                    ip_str.push(']').map_err(|_e| Self::Error::BadLength)?;
                    url.push_str(&ip_str).map_err(|_e| Self::Error::BadLength)?;
                }
            }
            url.push(':').map_err(|_e| Self::Error::BadLength)?;


            port = to_string(
                &remote.port(),
                String::<consts::U1>::new(),
                SerializeOptions::default(),
            )
            .map_err(|_e| Self::Error::BadLength)?;
            url.push_str(&port).map_err(|_e| Self::Error::BadLength)?;
            url.push('/').map_err(|_e| Self::Error::BadLength)?;

            // if tcp.c_cert_name != None || tcp.c_key_name != None || tcp.ca_cert_name != None {
            if let Some(ref credentials) = self.security_credentials {
                url.push('?').map_err(|_e| Self::Error::BadLength)?;

                // if let Some(ref ca) = tcp.ca_cert_name{
                if let Some(ref ca) = credentials.ca_cert_name {
                    url.push_str(&"ca=").map_err(|_e| Self::Error::BadLength)?;
                    url.push_str(ca).map_err(|_e| Self::Error::BadLength)?;
                    url.push('&').map_err(|_e| Self::Error::BadLength)?;
                }

                // if let Some(ref client) = tcp.c_cert_name{
                if let Some(ref client) = credentials.c_cert_name {
                    url.push_str(&"cert=")
                        .map_err(|_e| Self::Error::BadLength)?;
                    url.push_str(client).map_err(|_e| Self::Error::BadLength)?;
                    url.push('&').map_err(|_e| Self::Error::BadLength)?;
                }

                // if let Some(ref key) = tcp.c_key_name {
                if let Some(ref key) = credentials.c_key_name {
                    url.push_str(&"privKey=")
                        .map_err(|_e| Self::Error::BadLength)?;
                    url.push_str(key).map_err(|_e| Self::Error::BadLength)?;
                    url.push('&').map_err(|_e| Self::Error::BadLength)?;
                }
                url.pop();
            }

            defmt::trace!("[TCP] Connecting to url! {:str}", url);

            let resp = self.handle_socket_error(
                || self.send_internal(&EdmAtCmdWrapper(ConnectPeer { url: &url }), false),
                Some(*socket),
                0,
            )?;
            let handle = SocketHandle(resp.peer_handle);
            tcp.set_state(TcpState::SynSent);
            tcp.endpoint = remote;
            tcp.meta.handle = handle;
            *socket = handle;
        }
        while {
            let mut sockets = self.sockets.try_borrow_mut().map_err(Self::Error::from)?;
            let mut tcp = sockets.get::<TcpSocket<_>>(*socket).map_err(Self::Error::from)?;
            tcp.state() == TcpState::SynSent
        } {
            self.spin()?;
        }
        Ok(())
    }

    /// Check if this socket is still connected
    fn is_connected(&self, socket: &Self::TcpSocket) -> Result<bool, Self::Error> {
        if let Some(ref con) = *self.wifi_connection.try_borrow()? {
            if !self.initialized.get() || !con.is_connected() {
                return Ok(false);
            }
        } else {
            return Ok(false);
        }

        let mut sockets = self.sockets.try_borrow_mut()?;
        let socket_ref = sockets.get::<TcpSocket<_>>(*socket)?;
        Ok(socket_ref.state() == TcpState::Established)
    }

    /// Write to the stream. Returns the number of bytes written is returned
    /// (which may be less than `buffer.len()`), or an error.
    fn send(&self, socket: &mut Self::TcpSocket, buffer: &[u8]) -> nb::Result<usize, Self::Error> {
        if let Some(ref con) = *self
            .wifi_connection
            .try_borrow()
            .map_err(|e| nb::Error::Other(e.into()))?
        {
            if !self.initialized.get() || !con.is_connected() {
                return Err(nb::Error::Other(Error::Network));
            }
        } else {
            return Err(nb::Error::Other(Error::Network));
        }

        let mut sockets = self
            .sockets
            .try_borrow_mut()
            .map_err(|e| nb::Error::Other(e.into()))?;

        let mut tcp = sockets
            .get::<TcpSocket<_>>(*socket)
            .map_err(|e| nb::Error::Other(e.into()))?;

        if !tcp.may_send() {
            return Err(nb::Error::Other(Error::SocketClosed));
        }

        for chunk in buffer.chunks(EgressChunkSize::to_usize()) {
            self.handle_socket_error(
                || {
                    self.send_internal(
                        &EdmDataCommand {
                            channel: tcp.channel_id().0,
                            data: chunk,
                        },
                        true,
                    )
                },
                Some(*socket),
                0,
            )?;
        }
        Ok(buffer.len())
    }

    fn receive(&self, socket: &mut Self::TcpSocket, buffer: &mut [u8]) -> nb::Result<usize, Self::Error> {
        self.spin()?;

        let mut sockets = self
            .sockets
            .try_borrow_mut()
            .map_err(|e| nb::Error::Other(e.into()))?;

        let mut tcp = sockets
            .get::<TcpSocket<_>>(*socket)
            .map_err(|e| nb::Error::Other(e.into()))?;

        tcp.recv_slice(buffer)
            .map_err(|e| nb::Error::Other(e.into()))
    }

    // fn read_with<F>(&self, socket: &mut Self::TcpSocket, f: F) -> nb::Result<usize, Self::Error>
    // where
    //     F: FnOnce(&[u8], Option<&[u8]>) -> usize,
    // {
    //     self.spin()?;

    //     let mut sockets = self
    //         .sockets
    //         .try_borrow_mut()
    //         .map_err(|e| nb::Error::Other(e.into()))?;

    //     let mut tcp = sockets
    //         .get::<TcpSocket<_>>(*socket)
    //         .map_err(|e| nb::Error::Other(e.into()))?;

    //     tcp.recv_wrapping(|a, b| f(a, b))
    //         .map_err(|e| nb::Error::Other(e.into()))
    // }

    /// Close an existing TCP socket.
    fn close(&self, socket: Self::TcpSocket) -> Result<(), Self::Error> {
        defmt::debug!("[TCP] Closing socket: {:?}", socket.0);
        let mut sockets = self.sockets.try_borrow_mut()?;

        // If the socket is not found it is already removed
        if let Some(ref mut tcp) = sockets.get::<TcpSocket<_>>(socket).ok() {
            //If socket is not closed that means a connection excists which has to be closed
            if tcp.state() != TcpState::Closed {
                match self.handle_socket_error(
                    || {
                        self.send_at(ClosePeerConnection {
                            peer_handle: tcp.handle().0,
                        })
                    },
                    Some(socket),
                    0,
                ) {
                    Err(Error::AT(atat::Error::InvalidResponse)) | Ok(_) => (),
                    Err(e) => return Err(e.into()),
                }
                tcp.close();
            } else {
                //No connection exists the socket should be removed from the set here
                tcp.close();
                sockets.remove(socket)?;
            }
        }
        Ok(())
    }
}
