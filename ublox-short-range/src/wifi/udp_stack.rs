use crate::{
    command::data_mode::*,
    command::{
        data_mode::types::{IPVersion, ServerConfig, UDPBehaviour},
        edm::{EdmAtCmdWrapper, EdmDataCommand},
    },
    wifi::peer_builder::PeerUrlBuilder,
    UbloxClient,
};
use core::convert::TryInto;
use embedded_hal::digital::OutputPin;
use embedded_nal::{nb, SocketAddr};
use embedded_time::{
    duration::{Generic, Milliseconds},
    Clock,
};

use embedded_nal::{UdpClientStack, UdpFullStack};
use ublox_sockets::{Error, SocketHandle, UdpSocket, UdpState};

use super::EGRESS_CHUNK_SIZE;

impl<C, CLK, RST, const N: usize, const L: usize> UdpClientStack for UbloxClient<C, CLK, RST, N, L>
where
    C: atat::AtatClient,
    CLK: Clock,
    RST: OutputPin,
    Generic<CLK::T>: TryInto<Milliseconds>,
{
    type Error = Error;

    // Only return a SocketHandle to reference into the SocketSet owned by the UbloxClient,
    // as the Socket object itself provides no value without accessing it though the client.
    type UdpSocket = SocketHandle;

    fn socket(&mut self) -> Result<Self::UdpSocket, Self::Error> {
        if let Some(ref mut sockets) = self.sockets {
            // Check if there are any unused sockets available
            if sockets.len() >= sockets.capacity() {
                if let Ok(ts) = self.timer.try_now() {
                    // Check if there are any sockets closed by remote, and close it
                    // if it has exceeded its timeout, in order to recycle it.
                    // TODO Is this correct?
                    if sockets.recycle(&ts) {
                        return Err(Error::SocketSetFull);
                    }
                } else {
                    return Err(Error::SocketSetFull);
                }
            }

            defmt::debug!("[UDP] Opening socket");
            if let Some(ref con) = self.wifi_connection {
                if !self.initialized || !con.is_connected() {
                    return Err(Error::Illegal);
                }
            } else {
                return Err(Error::Illegal);
            }

            sockets.add(UdpSocket::new(0)).map_err(|_| {
                defmt::error!("[UDP] Opening socket Error: Socket set full");
                Error::SocketSetFull
            })
        } else {
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
            return Err(Error::Illegal);
        }

        if let Some(ref con) = self.wifi_connection {
            if !self.initialized || !con.is_connected() {
                return Err(Error::Illegal);
            }
        } else {
            return Err(Error::Illegal);
        }

        let url = PeerUrlBuilder::new()
            .address(&remote)
            .udp()
            .map_err(|_| Error::Unaddressable)?;
        defmt::trace!("[UDP] Connecting URL! {=str}", url);
        let new_handle = self
            .send_internal(&EdmAtCmdWrapper(ConnectPeer { url: &url }), false)
            .map_err(|_| Error::Unaddressable)?
            .peer_handle;

        let mut udp = self
            .sockets
            .as_mut()
            .unwrap()
            .get::<UdpSocket<CLK, L>>(*socket)?;

        *socket = new_handle;
        udp.update_handle(*socket);

        // FIXME: Should this be blocking here?
        while self
            .sockets
            .as_mut()
            .unwrap()
            .get::<UdpSocket<CLK, L>>(*socket)?
            .state()
            == UdpState::Closed
        {
            self.spin().map_err(|_| Error::Illegal)?;
        }
        Ok(())
    }

    /// Send a datagram to the remote host.
    fn send(&mut self, socket: &mut Self::UdpSocket, buffer: &[u8]) -> nb::Result<(), Self::Error> {
        if let Some(ref mut sockets) = self.sockets {
            if let Some(ref con) = self.wifi_connection {
                if !self.initialized || !con.is_connected() {
                    return Err(Error::Illegal.into());
                }
            } else {
                return Err(Error::Illegal.into());
            }

            let udp = sockets
                .get::<UdpSocket<CLK, L>>(*socket)
                .map_err(Self::Error::from)?;

            if !udp.is_open() {
                return Err(Error::SocketClosed.into());
            }

            self.spin().map_err(|_| nb::Error::Other(Error::Illegal))?;

            let channel = *self
                .edm_mapping
                .channel_id(socket)
                .ok_or(nb::Error::WouldBlock)?;

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
        // Handle reciving for udp server ports
        let udp_listener = &mut self.udp_listener;
        if udp_listener.is_bound(*socket) {
            if !udp_listener.available(*socket).unwrap_or(false) {
                return Err(nb::Error::WouldBlock);
            }

            let (handle, remote) = self
                .udp_listener
                .accept(*socket)
                .map_err(|_| Error::NotBound)?;

            if let Some(ref mut sockets) = self.sockets {
                let mut udp = sockets
                    .get::<UdpSocket<CLK, L>>(handle)
                    .map_err(|_| Self::Error::InvalidSocket)?;

                let bytes = udp.recv_slice(buffer).map_err(Self::Error::from)?;
                self.udp_listener
                    .outgoing_connection(handle, remote)
                    .map_err(|_| Error::Illegal)?;
                Ok((bytes, remote))
            } else {
                Err(Error::Illegal.into())
            }

            // Handle reciving for udp normal ports
        } else if let Some(ref mut sockets) = self.sockets {
            let mut udp = sockets
                .get::<UdpSocket<CLK, L>>(*socket)
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
        if let Some(ref mut sockets) = self.sockets {
            defmt::debug!("[UDP] Closing socket: {:?}", socket);
            // If no sockets exists, nothing to close.
            if let Ok(ref mut udp) = sockets.get::<UdpSocket<CLK, L>>(socket) {
                let peer_handle = udp.handle();

                udp.close();
                self.send_at(ClosePeerConnection { peer_handle })
                    .map_err(|_| Error::Unaddressable)?;
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
/// - One can only send to Socket addresses that have send data first.
/// - One can only recive an incomming datastream once.
/// - One can only use send_to once after reciving data once.
/// - One has to use send_to after reciving data, to release the socket bound by remote host,
/// even if just sending no bytes.
///
impl<C, CLK, RST, const N: usize, const L: usize> UdpFullStack for UbloxClient<C, CLK, RST, N, L>
where
    C: atat::AtatClient,
    CLK: Clock,
    RST: OutputPin,
    Generic<CLK::T>: TryInto<Milliseconds>,
{
    fn bind(&mut self, socket: &mut Self::UdpSocket, local_port: u16) -> Result<(), Self::Error> {
        if self.sockets.is_none() {
            return Err(Error::Illegal);
        }

        defmt::debug!("[UDP] bind socket: {:?}", socket);
        if let Some(ref con) = self.wifi_connection {
            if !self.initialized || !con.is_connected() {
                return Err(Error::Illegal);
            }
        } else {
            return Err(Error::Illegal);
        }

        self.send_internal(
            &EdmAtCmdWrapper(ServerConfiguration {
                id: 2,
                server_config: ServerConfig::UDP(
                    local_port,
                    UDPBehaviour::AutoConnect,
                    IPVersion::IPv4,
                ),
            }),
            false,
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
        //Protect against non server sockets
        if socket.0 != 0 {
            return Err(Error::Illegal.into());
        }
        // Check incomming sockets for the socket address
        if let Some(socket) = self.udp_listener.get_outgoing(remote) {
            if let Some(ref mut sockets) = self.sockets {
                if let Some(ref con) = self.wifi_connection {
                    if !self.initialized || !con.is_connected() {
                        return Err(Error::Illegal.into());
                    }
                } else {
                    return Err(Error::Illegal.into());
                }

                if buffer.len() == 0 {
                    self.close(socket).unwrap();
                    return Ok(());
                }

                let udp = sockets
                    .get::<UdpSocket<CLK, L>>(socket)
                    .map_err(Self::Error::from)?;

                if !udp.is_open() {
                    return Err(Error::SocketClosed.into());
                }

                self.spin().map_err(|_| nb::Error::Other(Error::Illegal))?;

                let channel = *self
                    .edm_mapping
                    .channel_id(&socket)
                    .ok_or(nb::Error::WouldBlock)?;

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
                self.close(socket).unwrap();
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
