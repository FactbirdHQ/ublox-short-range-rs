use crate::{
    command::data_mode::*,
    command::edm::{EdmAtCmdWrapper, EdmDataCommand},
    error::Error,
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

use embedded_nal::UdpClientStack;
use ublox_sockets::{SocketHandle, UdpSocket, UdpState};

pub(crate) const EGRESS_CHUNK_SIZE: usize = 512;

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
                    if sockets.recycle(&ts) {
                        return Err(Error::Network);
                    }
                } else {
                    return Err(Error::Network);
                }
            }

            defmt::debug!("[UDP] Opening socket");
            if let Some(ref con) = self.wifi_connection {
                if !self.initialized || !con.is_connected() {
                    return Err(Error::Network);
                }
            } else {
                return Err(Error::Network);
            }

            sockets.add(UdpSocket::new(0)).map_err(|_| {
                defmt::error!("[UDP] Opening socket Error: Socket set full");
                Error::Network
            })
        } else {
            Err(Error::SocketMemory)
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
            return Err(Error::SocketMemory);
        }

        defmt::debug!("[UDP] Connecting socket");
        if let Some(ref con) = self.wifi_connection {
            if !self.initialized || !con.is_connected() {
                return Err(Error::Network);
            }
        } else {
            return Err(Error::Network);
        }

        let url = PeerUrlBuilder::new().address(&remote).udp()?;
        defmt::trace!("[UDP] Connecting URL! {=str}", url);
        let resp = self.send_internal(&EdmAtCmdWrapper(ConnectPeer { url: &url }), false)?;

        let mut udp = self
            .sockets
            .as_mut()
            .unwrap()
            .get::<UdpSocket<CLK, L>>(*socket)?;
        udp.bind(remote)?;
        *socket = SocketHandle(resp.peer_handle);

        while self
            .sockets
            .as_mut()
            .unwrap()
            .get::<UdpSocket<CLK, L>>(*socket)?
            .state()
            == UdpState::Closed
        {
            self.spin()?;
        }
        Ok(())
    }

    /// Send a datagram to the remote host.
    fn send(&mut self, socket: &mut Self::UdpSocket, buffer: &[u8]) -> nb::Result<(), Self::Error> {
        if let Some(ref mut sockets) = self.sockets {
            if let Some(ref con) = self.wifi_connection {
                if !self.initialized || !con.is_connected() {
                    return Err(nb::Error::Other(Error::Network));
                }
            } else {
                return Err(nb::Error::Other(Error::Network));
            }

            let udp = sockets
                .get::<UdpSocket<CLK, L>>(*socket)
                .map_err(Self::Error::from)?;

            if !udp.is_open() {
                return Err(nb::Error::Other(Error::SocketClosed));
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
            Ok(())
        } else {
            Err(Error::SocketMemory.into())
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
        if let Some(ref mut sockets) = self.sockets {
            let mut udp = sockets
                .get::<UdpSocket<CLK, L>>(*socket)
                .map_err(Self::Error::from)?;

            let bytes = udp.recv_slice(buffer).map_err(Self::Error::from)?;

            let endpoint = udp.endpoint().ok_or(Error::SocketNotConnected)?;
            Ok((bytes, endpoint))
        } else {
            Err(Error::SocketMemory.into())
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
                self.send_at(ClosePeerConnection { peer_handle })?;
            }

            Ok(())
        } else {
            Err(Error::SocketMemory)
        }
    }
}
