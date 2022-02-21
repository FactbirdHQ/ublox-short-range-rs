use crate::{
    command::data_mode::*,
    command::{
        data_mode::responses::ConnectPeerResponse,
        edm::{EdmAtCmdWrapper, EdmDataCommand},
    },
    wifi::peer_builder::PeerUrlBuilder,
    UbloxClient,
};
use atat::Clock;
use embedded_hal::digital::blocking::OutputPin;
/// Handles receiving data from sockets
/// implements TCP and UDP for WiFi client
use embedded_nal::{nb, SocketAddr, TcpClientStack};

use ublox_sockets::{Error, SocketHandle, TcpSocket, TcpState};

use super::EGRESS_CHUNK_SIZE;

impl<C, CLK, RST, const TIMER_HZ: u32, const N: usize, const L: usize> TcpClientStack
    for UbloxClient<C, CLK, RST, TIMER_HZ, N, L>
where
    C: atat::AtatClient,
    CLK: Clock<TIMER_HZ>,
    RST: OutputPin,
{
    type Error = Error;

    // Only return a SocketHandle to reference into the SocketSet owned by the UbloxClient,
    // as the Socket object itself provides no value without accessing it though the client.
    type TcpSocket = SocketHandle;

    /// Open a new TCP socket to the given address and port. The socket starts in the unconnected state.
    fn socket(&mut self) -> Result<Self::TcpSocket, Self::Error> {
        if let Some(ref mut sockets) = self.sockets {
            // Check if there are any unused sockets available
            if sockets.len() >= sockets.capacity() {
                // Check if there are any sockets closed by remote, and close it
                // if it has exceeded its timeout, in order to recycle it.
                if sockets.recycle(self.timer.now()) {
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

            sockets.add(TcpSocket::new(0)).map_err(|e| {
                defmt::error!("[TCP] Opening socket Error: {:?}", e);
                e
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
            return Err(Error::Illegal.into());
        }

        defmt::debug!("[TCP] Connect socket");
        if let Some(ref con) = self.wifi_connection {
            if !self.initialized || !con.is_connected() {
                return Err(nb::Error::Other(Error::Illegal));
            }
        } else {
            return Err(nb::Error::Other(Error::Illegal));
        }

        // If no socket is found we stop here
        // TODO: This could probably be done nicer?
        self.sockets
            .as_mut()
            .unwrap()
            .get::<TcpSocket<TIMER_HZ, L>>(*socket)
            .map_err(Self::Error::from)?;

        let url = PeerUrlBuilder::new()
            .address(&remote)
            .creds(self.security_credentials.clone())
            .tcp()
            .map_err(|_| Error::Unaddressable)?;

        let ConnectPeerResponse { peer_handle } = self
            .send_internal(&EdmAtCmdWrapper(ConnectPeer { url: &url }), false)
            .map_err(|_| Error::Unaddressable)?;

        let mut tcp = self
            .sockets
            .as_mut()
            .unwrap()
            .get::<TcpSocket<TIMER_HZ, L>>(*socket)
            .map_err(Self::Error::from)?;

        *socket = SocketHandle(peer_handle);
        tcp.set_state(TcpState::WaitingForConnect(remote));
        tcp.update_handle(*socket);

        defmt::trace!("[TCP] Connecting socket: {:?} to url: {=str}", socket, url);

        // TODO: Timeout?
        while {
            matches!(
                self.sockets
                    .as_mut()
                    .unwrap()
                    .get::<TcpSocket<TIMER_HZ, L>>(*socket)
                    .map_err(Self::Error::from)?
                    .state(),
                TcpState::WaitingForConnect(_)
            )
        } {
            self.spin().map_err(|_| Error::Illegal)?;
        }
        Ok(())
    }

    /// Check if this socket is still connected
    fn is_connected(&mut self, socket: &Self::TcpSocket) -> Result<bool, Self::Error> {
        if let Some(ref mut sockets) = self.sockets {
            if let Some(ref con) = self.wifi_connection {
                if !self.initialized || !con.is_connected() {
                    return Ok(false);
                }
            } else {
                return Ok(false);
            }

            let tcp = sockets.get::<TcpSocket<TIMER_HZ, L>>(*socket)?;
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
                .get::<TcpSocket<TIMER_HZ, L>>(*socket)
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
                .get::<TcpSocket<TIMER_HZ, L>>(*socket)
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
            if let Ok(ref tcp) = sockets.get::<TcpSocket<TIMER_HZ, L>>(socket) {
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
            Ok(())
        } else {
            Err(Error::Illegal)
        }
    }
}

// impl<C, CLK, RST, const N: usize, const L: usize> TcpFullStack for UbloxClient<C, CLK, RST, N, L>
// where
//     C: atat::AtatClient,
//     CLK: Clock,
//     RST: OutputPin,
//     Generic<CLK::T>: TryInto<Milliseconds>,
// {
//     fn bind(&mut self, socket: &mut Self::TcpSocket, local_port: u16) -> Result<(), Self::Error> {
//         todo!()
//     }

//     fn listen(&mut self, socket: &mut Self::TcpSocket) -> Result<(), Self::Error> {
//         todo!()
//     }

//     fn accept(
// 		&mut self,
// 		socket: &mut Self::TcpSocket,
// 	) -> nb::Result<(Self::TcpSocket, SocketAddr), Self::Error> {
//         todo!()
//     }
// }
