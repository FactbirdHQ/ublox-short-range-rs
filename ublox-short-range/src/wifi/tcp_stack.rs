use crate::{
    command::data_mode::*,
    command::{
        data_mode::types::{ImmediateFlush, ServerConfig},
        edm::{EdmAtCmdWrapper, EdmDataCommand},
    },
    wifi::peer_builder::PeerUrlBuilder,
    UbloxClient,
};
use core::convert::TryInto;
use embedded_hal::digital::OutputPin;
/// Handles receiving data from sockets
/// implements TCP and UDP for WiFi client
use embedded_nal::{nb, SocketAddr, TcpClientStack, TcpFullStack};
use embedded_time::{
    duration::{Generic, Milliseconds},
    Clock,
};

use ublox_sockets::{Error, SocketHandle, TcpSocket, TcpState};

use super::EGRESS_CHUNK_SIZE;

impl<C, CLK, RST, const N: usize, const L: usize> TcpClientStack for UbloxClient<C, CLK, RST, N, L>
where
    C: atat::AtatClient,
    CLK: Clock,
    RST: OutputPin,
    Generic<CLK::T>: TryInto<Milliseconds>,
{
    type Error = Error;

    // Only return a SocketHandle to reference into the SocketSet owned by the UbloxClient,
    // as the Socket object itself provides no value without accessing it though the client.
    type TcpSocket = SocketHandle;

    /// Open a socket for usage as a TCP client.
    ///
    /// The socket must be connected before it can be used.
    ///
    /// Returns `Ok(socket)` if the socket was successfully created.
    fn socket(&mut self) -> Result<Self::TcpSocket, Self::Error> {
        if let Some(ref mut sockets) = self.sockets {
            // Check if there are any unused sockets available
            if sockets.len() >= sockets.capacity() {
                if let Ok(ts) = self.timer.try_now() {
                    // Check if there are any sockets closed by remote, and close it
                    // if it has exceeded its timeout, in order to recycle it.
                    if sockets.recycle(&ts) {
                        return Err(Error::SocketSetFull);
                    }
                } else {
                    return Err(Error::SocketSetFull);
                }
            }

            defmt::debug!("[TCP] Opening socket");
            if let Some(ref con) = self.wifi_connection {
                if !self.initialized || !con.is_connected() {
                    return Err(Error::Illegal);
                }
            } else {
                return Err(Error::Illegal);
            }

            sockets.add(TcpSocket::new(0)).map_err(|_| {
                defmt::error!("[TCP] Opening socket Error: Socket set full");
                Error::SocketSetFull
            })
        } else {
            Err(Error::Illegal)
        }
    }

    /// Connect to the given remote host and port.
    fn connect(
        &mut self,
        socket: &mut Self::TcpSocket,
        remote: SocketAddr,
    ) -> nb::Result<(), Self::Error> {
        if self.sockets.is_none() {
            return Err(nb::Error::Other(Error::Illegal));
        }

        if let Some(ref con) = self.wifi_connection {
            if !self.initialized || !con.is_connected() {
                return Err(nb::Error::Other(Error::Illegal));
            }
        } else {
            return Err(nb::Error::Other(Error::Illegal));
        }

        // If no socket is found we stop here
        match self
            .sockets
            .as_mut()
            .unwrap()
            .get::<TcpSocket<CLK, L>>(*socket)
            .map_err(Self::Error::from)?
            .state()
        {
            TcpState::Created => {
                let url = PeerUrlBuilder::new()
                    .address(&remote)
                    .creds(self.security_credentials.clone())
                    .tcp()
                    .map_err(|_| Error::Unaddressable)?;

                defmt::trace!("[TCP] Connecting to url! {=str}", url);

                let new_handle = self
                    .send_at(ConnectPeer { url: &url })
                    .map_err(|_| Error::Unaddressable)?
                    .peer_handle;

                let mut tcp = self
                    .sockets
                    .as_mut()
                    .unwrap()
                    .get::<TcpSocket<CLK, L>>(*socket)
                    .map_err(Self::Error::from)?;
                *socket = new_handle;
                tcp.set_state(TcpState::WaitingForConnect(remote));
                tcp.update_handle(*socket);
                Err(nb::Error::WouldBlock)
            }
            TcpState::WaitingForConnect(_) => {
                self.spin().map_err(|_| Error::Illegal)?;
                Err(nb::Error::WouldBlock)
            }
            TcpState::Connected(_) => Ok(()),
            _ => Err(Error::Illegal.into()),
        }
    }

    /// Check if this socket is still connected
    fn is_connected(&mut self, socket: &Self::TcpSocket) -> Result<bool, Self::Error> {
        if let Some(ref mut sockets) = self.sockets {
            if let Some(ref con) = self.wifi_connection {
                if !self.initialized || !con.is_connected() {
                    defmt::debug!("!self.initialized || !con.is_connected()");
                    return Ok(false);
                }
            } else {
                defmt::debug!("not wifi_connection ?!");

                return Ok(false);
            }

            let tcp = sockets.get::<TcpSocket<CLK, L>>(*socket)?;
            Ok(tcp.is_connected())
        } else {
            Err(Error::Illegal)
        }
    }

    /// Write to the stream. Returns the number of bytes written is returned
    /// (which may be less than `buffer.len()`), or an error.
    fn send(
        &mut self,
        socket: &mut Self::TcpSocket,
        buffer: &[u8],
    ) -> nb::Result<usize, Self::Error> {
        if let Some(ref mut sockets) = self.sockets {
            if let Some(ref con) = self.wifi_connection {
                if !self.initialized || !con.is_connected() {
                    return Err(Error::Illegal.into());
                }
            } else {
                return Err(Error::Illegal.into());
            }

            let tcp = sockets
                .get::<TcpSocket<CLK, L>>(*socket)
                .map_err(|e| nb::Error::Other(e.into()))?;

            if !tcp.is_connected() {
                return Err(Error::SocketClosed.into());
            }

            let channel = *self
                .edm_mapping
                .channel_id(socket)
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
            Ok(buffer.len())
        } else {
            Err(Error::Illegal.into())
        }
    }

    fn receive(
        &mut self,
        socket: &mut Self::TcpSocket,
        buffer: &mut [u8],
    ) -> nb::Result<usize, Self::Error> {
        self.spin().map_err(|_| nb::Error::Other(Error::Illegal))?;
        if let Some(ref mut sockets) = self.sockets {
            let mut tcp = sockets
                .get::<TcpSocket<CLK, L>>(*socket)
                .map_err(Self::Error::from)?;

            Ok(tcp.recv_slice(buffer).map_err(Self::Error::from)?)
        } else {
            Err(Error::Illegal.into())
        }
    }

    /// Close an existing TCP socket.
    fn close(&mut self, socket: Self::TcpSocket) -> Result<(), Self::Error> {
        if let Some(ref mut sockets) = self.sockets {
            defmt::debug!("[TCP] Closing socket: {:?}", socket);
            // If the socket is not found it is already removed
            if let Ok(ref tcp) = sockets.get::<TcpSocket<CLK, L>>(socket) {
                // If socket is not closed that means a connection excists which has to be closed
                if !matches!(
                    tcp.state(),
                    TcpState::ShutdownForWrite(_) | TcpState::Created
                ) {
                    let peer_handle = tcp.handle();
                    match self.send_at(ClosePeerConnection { peer_handle }) {
                        Err(crate::error::Error::AT(atat::Error::InvalidResponse)) | Ok(_) => (),
                        Err(_) => return Err(Error::Unaddressable),
                    }
                } else {
                    // No connection exists the socket should be removed from the set here
                    sockets.remove(socket)?;
                }
            }
            // TODO: Close listening socket?
            Ok(())
        } else {
            Err(Error::Illegal)
        }
    }
}

impl<C, CLK, RST, const N: usize, const L: usize> TcpFullStack for UbloxClient<C, CLK, RST, N, L>
where
    C: atat::AtatClient,
    CLK: Clock,
    RST: OutputPin,
    Generic<CLK::T>: TryInto<Milliseconds>,
{
    fn bind(&mut self, socket: &mut Self::TcpSocket, local_port: u16) -> Result<(), Self::Error> {
        if self.sockets.is_none() {
            return Err(Error::Illegal);
        }

        defmt::debug!("[TCP] bind socket: {:?}", socket);
        if let Some(ref con) = self.wifi_connection {
            if !self.initialized || !con.is_connected() {
                return Err(Error::Illegal);
            }
        } else {
            return Err(Error::Illegal);
        }

        self.send_internal(
            &EdmAtCmdWrapper(ServerConfiguration {
                id: 1,
                server_config: ServerConfig::TCP(local_port, ImmediateFlush::Disable),
            }),
            false,
        )
        .map_err(|_| Error::Unaddressable)?;

        self.tcp_listener
            .bind(*socket, local_port)
            .map_err(|_| Error::Illegal)?;

        Ok(())
    }

    fn listen(&mut self, _socket: &mut Self::TcpSocket) -> Result<(), Self::Error> {
        // Nop operation as this happens together with `bind()`
        Ok(())
    }

    fn accept(
        &mut self,
        socket: &mut Self::TcpSocket,
    ) -> nb::Result<(Self::TcpSocket, SocketAddr), Self::Error> {
        if !self
            .tcp_listener
            .available(*socket)
            .map_err(|_| Error::NotBound)?
        {
            return Err(nb::Error::WouldBlock);
        }

        let data_socket = self.socket()?;
        let (handle, remote) = self
            .tcp_listener
            .accept(*socket)
            .map_err(|_| Error::NotBound)?;

        // Crate a new SocketBuffer allocation for the incoming connection
        let mut tcp = self
            .sockets
            .as_mut()
            .ok_or(Error::Illegal)?
            .get::<TcpSocket<CLK, L>>(data_socket)
            .map_err(Self::Error::from)?;

        tcp.update_handle(handle);
        tcp.set_state(TcpState::Connected(remote.clone()));

        Ok((handle, remote))
    }
}
