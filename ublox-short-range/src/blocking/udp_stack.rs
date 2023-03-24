use super::client::new_socket_num;
use super::UbloxClient;
use crate::{
    command::data_mode::*,
    command::{
        data_mode::types::{IPVersion, ServerType, UDPBehaviour},
        edm::{EdmAtCmdWrapper, EdmDataCommand},
    },
    wifi::peer_builder::PeerUrlBuilder,
};
use embedded_hal::digital::OutputPin;
use embedded_nal::{nb, SocketAddr, UdpFullStack};

use embedded_nal::UdpClientStack;
use ublox_sockets::{Error, SocketHandle, UdpSocket, UdpState};

use crate::wifi::EGRESS_CHUNK_SIZE;

impl<C, RST, const N: usize, const L: usize> UdpClientStack for UbloxClient<C, RST, N, L>
where
    C: atat::blocking::AtatClient,
    RST: OutputPin,
{
    type Error = Error;

    // Only return a SocketHandle to reference into the SocketSet owned by the UbloxClient,
    // as the Socket object itself provides no value without accessing it though the client.
    type UdpSocket = SocketHandle;

    fn socket(&mut self) -> Result<Self::UdpSocket, Self::Error> {
        self.connected_to_network().map_err(|_| Error::Illegal)?;
        if let Some(ref mut sockets) = self.sockets {
            // Check if there are any unused sockets available
            if sockets.len() >= sockets.capacity() {
                // Check if there are any sockets closed by remote, and close it
                // if it has exceeded its timeout, in order to recycle it.
                if sockets.recycle() {
                    return Err(Error::SocketSetFull);
                }
            }

            let socket_id = new_socket_num(sockets).unwrap();
            defmt::debug!("[UDP] Opening socket");
            sockets.add(UdpSocket::new(socket_id)).map_err(|_| {
                defmt::error!("[UDP] Opening socket Error: Socket set full");
                Error::SocketSetFull
            })
        } else {
            defmt::error!("[UDP] Opening socket Error: Missing socket set");
            Err(Error::Illegal)
        }
    }

    /// Connect a UDP socket with a peer using a dynamically selected port.
    /// Selects a port number automatically and initializes for read/writing.
    fn connect(
        &mut self,
        socket: &mut Self::UdpSocket,
        remote: SocketAddr,
    ) -> Result<(), Self::Error> {
        if self.sockets.is_none() {
            defmt::error!("[UDP] Connecting socket Error: Missing socket set");
            return Err(Error::Illegal);
        }
        let url = PeerUrlBuilder::new()
            .address(&remote)
            .udp()
            .map_err(|_| Error::Unaddressable)?;
        defmt::debug!("[UDP] Connecting Socket: {:?} to URL: {=str}", socket, url);

        self.connected_to_network().map_err(|_| Error::Illegal)?;

        // First look to see if socket is valid
        let mut udp = self
            .sockets
            .as_mut()
            .unwrap()
            .get::<UdpSocket<L>>(*socket)?;
        udp.bind(remote)?;

        // Then connect modem
        match self
            .send_internal(&EdmAtCmdWrapper(ConnectPeer { url: &url }), true)
            .map_err(|_| Error::Unaddressable)
        {
            Ok(resp) => self
                .socket_map
                .insert_peer(resp.peer_handle.into(), *socket)
                .map_err(|_| Error::InvalidSocket)?,

            Err(e) => {
                let mut udp = self
                    .sockets
                    .as_mut()
                    .unwrap()
                    .get::<UdpSocket<L>>(*socket)?;
                udp.close();
                return Err(e);
            }
        }
        while self
            .sockets
            .as_mut()
            .unwrap()
            .get::<UdpSocket<L>>(*socket)?
            .state()
            == UdpState::Closed
        {
            self.spin().map_err(|_| Error::Illegal)?;
        }
        Ok(())
    }

    /// Send a datagram to the remote host.
    fn send(&mut self, socket: &mut Self::UdpSocket, buffer: &[u8]) -> nb::Result<(), Self::Error> {
        self.spin().map_err(|_| Error::Illegal)?;
        if let Some(ref mut sockets) = self.sockets {
            // No send for server sockets!
            if self.udp_listener.is_bound(*socket) {
                return Err(nb::Error::Other(Error::Illegal));
            }

            let udp = sockets
                .get::<UdpSocket<L>>(*socket)
                .map_err(Self::Error::from)?;

            if !udp.is_open() {
                return Err(Error::SocketClosed.into());
            }

            let channel = *self
                .socket_map
                .socket_to_channel_id(socket)
                .ok_or(nb::Error::Other(Error::SocketClosed))?;

            for chunk in buffer.chunks(EGRESS_CHUNK_SIZE) {
                self.send_internal(
                    &EdmDataCommand {
                        channel,
                        data: chunk,
                    },
                    true,
                )
                .map_err(|_| nb::Error::Other(Error::Unaddressable))?;
            }
            Ok(())
        } else {
            Err(Error::Illegal.into())
        }
    }

    /// Read a datagram the remote host has sent to us. Returns `Ok(n)`, which
    /// means a datagram of size `n` has been received and it has been placed
    /// in `&buffer[0..n]`, or an error.
    fn receive(
        &mut self,
        socket: &mut Self::UdpSocket,
        buffer: &mut [u8],
    ) -> nb::Result<(usize, SocketAddr), Self::Error> {
        self.spin().ok();
        let udp_listener = &mut self.udp_listener;
        // Handle server sockets
        if udp_listener.is_bound(*socket) {
            // Nothing available, would block
            if !udp_listener.available(*socket).unwrap_or(false) {
                return Err(nb::Error::WouldBlock);
            }

            let (connection_handle, remote) = self
                .udp_listener
                .peek_remote(*socket)
                .map_err(|_| Error::NotBound)?;

            if let Some(ref mut sockets) = self.sockets {
                let mut udp = sockets
                    .get::<UdpSocket<L>>(*connection_handle)
                    .map_err(|_| Self::Error::InvalidSocket)?;

                let bytes = udp.recv_slice(buffer).map_err(Self::Error::from)?;
                Ok((bytes, remote.clone()))
            } else {
                Err(Error::Illegal.into())
            }

            // Handle reciving for udp normal sockets
        } else if let Some(ref mut sockets) = self.sockets {
            let mut udp = sockets
                .get::<UdpSocket<L>>(*socket)
                .map_err(Self::Error::from)?;

            let bytes = udp.recv_slice(buffer).map_err(Self::Error::from)?;

            let endpoint = udp.endpoint().ok_or(Error::SocketClosed)?;
            Ok((bytes, endpoint))
        } else {
            Err(Error::Illegal.into())
        }
    }

    /// Close an existing UDP socket.
    fn close(&mut self, socket: Self::UdpSocket) -> Result<(), Self::Error> {
        self.spin().ok();
        // Close server socket
        if self.udp_listener.is_bound(socket) {
            defmt::debug!("[UDP] Closing Server socket: {:?}", socket);

            // ID 2 used by UDP server
            self.send_internal(
                &EdmAtCmdWrapper(ServerConfiguration {
                    id: 2,
                    server_config: ServerType::Disabled,
                }),
                true,
            )
            .map_err(|_| Error::Unaddressable)?;

            // Borrow socket set to close server socket
            if let Some(ref mut sockets) = self.sockets {
                // If socket in socket set close
                if sockets.remove(socket).is_err() {
                    defmt::error!(
                        "[UDP] Closing server socket error: No socket matching: {:?}",
                        socket
                    );
                    return Err(Error::InvalidSocket);
                }
            } else {
                return Err(Error::Illegal);
            }

            // Close incomming connections
            while self.udp_listener.available(socket).unwrap_or(false) {
                if let Ok((connection_handle, _)) = self.udp_listener.get_remote(socket) {
                    defmt::debug!(
                        "[UDP] Closing incomming socket for Server: {:?}",
                        connection_handle
                    );
                    self.close(connection_handle)?;
                } else {
                    defmt::error!("[UDP] Incomming socket for server error - Listener says available, while nothing present");
                }
            }

            // Unbind server socket in listener
            self.udp_listener.unbind(socket).map_err(|_| {
                defmt::error!(
                    "[UDP] Closing socket error: No server socket matching: {:?}",
                    socket
                );
                Error::Illegal
            })
        // Handle normal sockets
        } else if let Some(ref mut sockets) = self.sockets {
            defmt::debug!("[UDP] Closing socket: {:?}", socket);
            // If no sockets exists, nothing to close.
            if let Ok(ref mut udp) = sockets.get::<UdpSocket<L>>(socket) {
                defmt::trace!("[UDP] Closing socket state: {:?}", udp.state());
                match udp.state() {
                    UdpState::Closed => {
                        sockets.remove(socket).ok();
                    }
                    UdpState::Established => {
                        // FIXME:udp.close();
                        if let Some(peer_handle) = self.socket_map.socket_to_peer(&udp.handle()) {
                            let peer_handle = *peer_handle;
                            self.send_at(ClosePeerConnection { peer_handle })
                                .map_err(|_| Error::Unaddressable)?;
                        }
                    }
                }
            } else {
                defmt::error!(
                    "[UDP] Closing socket error: No socket matching: {:?}",
                    socket
                );
                return Err(Error::InvalidSocket);
            }
            Ok(())
        } else {
            Err(Error::Illegal)
        }
    }
}

/// UDP Full Stack
///
/// This fullstack is build for request-response type servers due to HW/SW limitations
/// Limitations:
/// - The driver can only send to Socket addresses that have send data first.
/// - The driver can only call send_to once after reciving data once.
/// - The driver has to call send_to after reciving data, to release the socket bound by remote host,
/// even if just sending no bytes. Else these sockets will be held open until closure of server socket.
///
impl<C, RST, const N: usize, const L: usize> UdpFullStack for UbloxClient<C, RST, N, L>
where
    C: atat::blocking::AtatClient,
    RST: OutputPin,
{
    fn bind(&mut self, socket: &mut Self::UdpSocket, local_port: u16) -> Result<(), Self::Error> {
        if self.connected_to_network().is_err() || self.udp_listener.is_port_bound(local_port) {
            return Err(Error::Illegal);
        }

        defmt::debug!(
            "[UDP] binding socket: {:?} to port: {:?}",
            socket,
            local_port
        );

        // ID 2 used by UDP server
        self.send_internal(
            &EdmAtCmdWrapper(ServerConfiguration {
                id: 2,
                server_config: ServerType::UDP(
                    local_port,
                    UDPBehaviour::AutoConnect,
                    IPVersion::IPv4,
                ),
            }),
            true,
        )
        .map_err(|_| Error::Unaddressable)?;

        self.udp_listener
            .bind(*socket, local_port)
            .map_err(|_| Error::Illegal)?;

        Ok(())
    }

    fn send_to(
        &mut self,
        socket: &mut Self::UdpSocket,
        remote: SocketAddr,
        buffer: &[u8],
    ) -> nb::Result<(), Self::Error> {
        self.spin().map_err(|_| Error::Illegal)?;
        // Protect against non server sockets
        if !self.udp_listener.is_bound(*socket) {
            return Err(Error::Illegal.into());
        }
        // Check incomming sockets for the socket address
        if let Some(connection_socket) = self.udp_listener.get_outgoing(socket, remote) {
            if let Some(ref mut sockets) = self.sockets {
                if buffer.len() == 0 {
                    self.close(connection_socket)?;
                    return Ok(());
                }

                let udp = sockets
                    .get::<UdpSocket<L>>(connection_socket)
                    .map_err(Self::Error::from)?;

                if !udp.is_open() {
                    return Err(Error::SocketClosed.into());
                }

                let channel = *self
                    .socket_map
                    .socket_to_channel_id(&connection_socket)
                    .ok_or(nb::Error::WouldBlock)?;

                for chunk in buffer.chunks(EGRESS_CHUNK_SIZE) {
                    self.send_internal(
                        &EdmDataCommand {
                            channel,
                            data: chunk,
                        },
                        false,
                    )
                    .map_err(|_| nb::Error::Other(Error::Unaddressable))?;
                }
                self.close(connection_socket).unwrap();
                Ok(())
            } else {
                Err(Error::Illegal.into())
            }
        } else {
            Err(Error::Illegal.into())
        }

        ////// Do with URC
        // Crate a new SocketBuffer allocation for the incoming connection
        // let mut tcp = self
        //     .sockets
        //     .as_mut()
        //     .ok_or(Error::Illegal)?
        //     .get::<TcpSocket<CLK, L>>(data_socket)
        //     .map_err(Self::Error::from)?;

        // tcp.update_handle(handle);
        // tcp.set_state(TcpState::Connected(remote.clone()));
    }
}
