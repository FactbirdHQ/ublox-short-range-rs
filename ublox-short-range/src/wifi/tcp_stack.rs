use crate::{
    command::data_mode::*,
    command::{
        data_mode::responses::ConnectPeerResponse,
        edm::{EdmAtCmdWrapper, EdmDataCommand},
    },
    error::Error,
    wifi::peer_builder::PeerUrlBuilder,
    UbloxClient,
};
use core::convert::TryInto;
use embedded_hal::digital::OutputPin;
/// Handles receiving data from sockets
/// implements TCP and UDP for WiFi client
use embedded_nal::{nb, SocketAddr, TcpClientStack};
use embedded_time::{
    duration::{Generic, Milliseconds},
    Clock,
};

use ublox_sockets::{SocketHandle, TcpSocket, TcpState};

pub(crate) const EGRESS_CHUNK_SIZE: usize = 512;

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

    /// Open a new TCP socket to the given address and port. The socket starts in the unconnected state.
    fn socket(&mut self) -> Result<Self::TcpSocket, Self::Error> {
        if let Some(ref mut sockets) = self.sockets {
            // Check if there are any unused sockets available
            if sockets.len() >= sockets.capacity() {
                if let Ok(ts) = self.timer.try_now() {
                    // Check if there are any sockets closed by remote, and close it
                    // if it has exceeded its timeout, in order to recycle it.
                    if sockets.recycle(&ts) {
                        return Err(Error::Network);
                    }
                } else {
                    return Err(Error::Network);
                }
            }

            defmt::debug!("[TCP] Opening socket");
            if let Some(ref con) = self.wifi_connection {
                if !self.initialized || !con.is_connected() {
                    return Err(Error::Network);
                }
            } else {
                return Err(Error::Network);
            }

            sockets.add(TcpSocket::new(0)).map_err(|_| {
                defmt::error!("[TCP] Opening socket Error: Socket set full");
                Error::Network
            })
        } else {
            Err(Error::SocketMemory)
        }
    }

    /// Connect to the given remote host and port.
    fn connect(
        &mut self,
        socket: &mut Self::TcpSocket,
        remote: SocketAddr,
    ) -> nb::Result<(), Self::Error> {
        if self.sockets.is_none() {
            return Err(Error::SocketMemory.into());
        }

        defmt::debug!("[TCP] Connect socket: {:?}", socket);
        if let Some(ref con) = self.wifi_connection {
            if !self.initialized || !con.is_connected() {
                return Err(nb::Error::Other(Error::Network));
            }
        } else {
            return Err(nb::Error::Other(Error::Network));
        }

        // If no socket is found we stop here
        // TODO: This could probably be done nicer?
        self.sockets
            .as_mut()
            .unwrap()
            .get::<TcpSocket<CLK, L>>(*socket)
            .map_err(Self::Error::from)?;

        let mut url_builder = PeerUrlBuilder::new();
        self.security_credentials
            .as_ref()
            .and_then(|cred| cred.ca_cert_name.as_ref())
            .map(|ca| url_builder.ca(ca));
        self.security_credentials
            .as_ref()
            .and_then(|cred| cred.c_cert_name.as_ref())
            .map(|cert| url_builder.cert(cert));
        self.security_credentials
            .as_ref()
            .and_then(|cred| cred.c_key_name.as_ref())
            .map(|pkey| url_builder.pkey(pkey));
        let url = url_builder.address(&remote).tcp()?;

        defmt::trace!("[TCP] Connecting to url! {=str}", url);

        let ConnectPeerResponse { peer_handle } =
            self.send_internal(&EdmAtCmdWrapper(ConnectPeer { url: &url }), false)?;

        let mut tcp = self
            .sockets
            .as_mut()
            .unwrap()
            .get::<TcpSocket<CLK, L>>(*socket)
            .map_err(Self::Error::from)?;

        tcp.set_state(TcpState::WaitingForConnect(remote));
        *socket = SocketHandle(peer_handle);

        // TODO: Timeout?
        while {
            matches!(
                self.sockets
                    .as_mut()
                    .unwrap()
                    .get::<TcpSocket<CLK, L>>(*socket)
                    .map_err(Self::Error::from)?
                    .state(),
                TcpState::WaitingForConnect(_)
            )
        } {
            self.spin()?;
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

            let tcp = sockets.get::<TcpSocket<CLK, L>>(*socket)?;
            Ok(tcp.is_connected())
        } else {
            Err(Error::SocketMemory)
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
                    return Err(nb::Error::Other(Error::Network));
                }
            } else {
                return Err(nb::Error::Other(Error::Network));
            }

            let tcp = sockets
                .get::<TcpSocket<CLK, L>>(*socket)
                .map_err(|e| nb::Error::Other(e.into()))?;

            if !tcp.is_connected() {
                return Err(nb::Error::Other(Error::SocketNotConnected));
            }

            let channel = *self
                .edm_mapping
                .channel_id(socket)
                .ok_or(nb::Error::Other(Error::SocketNotConnected))?;

            for chunk in buffer.chunks(EGRESS_CHUNK_SIZE) {
                self.send_internal(
                    &EdmDataCommand {
                        channel,
                        data: chunk,
                    },
                    true,
                )?;
            }
            Ok(buffer.len())
        } else {
            Err(Error::SocketMemory.into())
        }
    }

    fn receive(
        &mut self,
        socket: &mut Self::TcpSocket,
        buffer: &mut [u8],
    ) -> nb::Result<usize, Self::Error> {
        self.spin()?;
        if let Some(ref mut sockets) = self.sockets {
            let mut tcp = sockets
                .get::<TcpSocket<CLK, L>>(*socket)
                .map_err(|e| nb::Error::Other(e.into()))?;

            tcp.recv_slice(buffer)
                .map_err(|e| nb::Error::Other(e.into()))
        } else {
            Err(Error::SocketMemory.into())
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
                        Err(Error::AT(atat::Error::InvalidResponse)) | Ok(_) => (),
                        Err(e) => return Err(e),
                    }
                } else {
                    // No connection exists the socket should be removed from the set here
                    sockets.remove(socket)?;
                }
            }
            Ok(())
        } else {
            Err(Error::SocketMemory)
        }
    }
}
