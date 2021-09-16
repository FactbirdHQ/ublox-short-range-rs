use crate::{
    command::data_mode::*,
    command::{
        data_mode::responses::ConnectPeerResponse,
        edm::{EdmAtCmdWrapper, EdmDataCommand},
    },
    error::Error,
    socket::SocketHandle,
    UbloxClient,
};
use core::convert::TryInto;
use core::fmt::Write;
use embedded_hal::digital::OutputPin;
/// Handles receiving data from sockets
/// implements TCP and UDP for WiFi client
pub use embedded_nal::{nb, AddrType, IpAddr, SocketAddr, SocketAddrV4, SocketAddrV6};
use embedded_time::duration::{Generic, Milliseconds};
use embedded_time::Clock;
use heapless::String;

#[cfg(feature = "socket-udp")]
use crate::socket::{UdpSocket, UdpState};
#[cfg(feature = "socket-udp")]
use embedded_nal::UdpClientStack;

#[cfg(feature = "socket-tcp")]
use crate::socket::{TcpSocket, TcpState};
#[cfg(feature = "socket-tcp")]
use embedded_nal::TcpClientStack;

pub(crate) const EGRESS_CHUNK_SIZE: usize = 512;

#[cfg(feature = "socket-udp")]
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
        // Check if there are any unused sockets available
        if self.sockets.len() >= self.sockets.capacity() {
            if let Ok(ts) = self.timer.try_now() {
                // Check if there are any sockets closed by remote, and close it
                // if it has exceeded its timeout, in order to recycle it.
                if self.sockets.recycle(&ts) {
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

        if let Ok(h) = self.sockets.add(UdpSocket::new(0)) {
            Ok(h)
        } else {
            defmt::error!("[UDP] Opening socket Error: Socket set full");
            Err(Error::Network)
        }
    }

    /// Connect a UDP socket with a peer using a dynamically selected port.
    /// Selects a port number automatically and initializes for read/writing.
    fn connect(
        &mut self,
        socket: &mut Self::UdpSocket,
        remote: SocketAddr,
    ) -> Result<(), Self::Error> {
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
        let handle = SocketHandle(resp.peer_handle);
        let mut udp = self
            .sockets
            .get::<UdpSocket<CLK, L>>(socket.clone().into())?;
        udp.endpoint = remote;
        udp.meta.handle = handle;
        *socket = handle;

        while {
            let udp = self
                .sockets
                .get::<UdpSocket<CLK, L>>(socket.clone().into())?;
            udp.state() == UdpState::Closed
        } {
            self.spin()?;
        }
        Ok(())
    }

    /// Send a datagram to the remote host.
    fn send(&mut self, socket: &mut Self::UdpSocket, buffer: &[u8]) -> nb::Result<(), Self::Error> {
        if let Some(ref con) = self.wifi_connection {
            if !self.initialized || !con.is_connected() {
                return Err(nb::Error::Other(Error::Network));
            }
        } else {
            return Err(nb::Error::Other(Error::Network));
        }

        let udp = self
            .sockets
            .get::<UdpSocket<CLK, L>>(socket.clone().into())
            .map_err(|e| nb::Error::Other(e.into()))?;

        if !udp.is_open() {
            return Err(nb::Error::Other(Error::SocketClosed));
        }

        let channel = udp.channel_id().0;

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
    }

    /// Read a datagram the remote host has sent to us. Returns `Ok(n)`, which
    /// means a datagram of size `n` has been received and it has been placed
    /// in `&buffer[0..n]`, or an error.
    fn receive(
        &mut self,
        socket: &mut Self::UdpSocket,
        buffer: &mut [u8],
    ) -> nb::Result<(usize, SocketAddr), Self::Error> {
        let mut udp = self
            .sockets
            .get::<UdpSocket<CLK, L>>(socket.clone().into())
            .map_err(|e| nb::Error::Other(Error::Socket(e)))?;

        let us = udp
            .recv_slice(buffer)
            .map_err(|e| nb::Error::Other(e.into()))?;
        Ok((us, udp.endpoint()))
    }

    /// Close an existing UDP socket.
    fn close(&mut self, socket: Self::UdpSocket) -> Result<(), Self::Error> {
        defmt::debug!("[UDP] Closing socket: {:?}", socket.0);
        // If no sockets exists, nothing to close.
        if let Some(ref mut udp) = self
            .sockets
            .get::<UdpSocket<CLK, L>>(socket.clone().into())
            .ok()
        {
            let peer_handle = udp.handle().0;

            udp.close();
            self.send_at(ClosePeerConnection { peer_handle })?;
        }

        Ok(())
    }
}

#[cfg(feature = "socket-tcp")]
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
        defmt::debug!("[TCP] Opening socket");
        if let Some(ref con) = self.wifi_connection {
            if !self.initialized || !con.is_connected() {
                return Err(Error::Network);
            }
        } else {
            return Err(Error::Network);
        }
        if let Ok(h) = self.sockets.add(TcpSocket::new(0)) {
            Ok(h)
        } else {
            defmt::error!("[TCP] Opening socket Error: Socket set full");
            Err(Error::Network)
        }
    }

    /// Connect to the given remote host and port.
    fn connect(
        &mut self,
        socket: &mut Self::TcpSocket,
        remote: SocketAddr,
    ) -> nb::Result<(), Self::Error> {
        defmt::debug!("[TCP] Connect socket: {:?}", socket.0);
        if let Some(ref con) = self.wifi_connection {
            if !self.initialized || !con.is_connected() {
                return Err(nb::Error::Other(Error::Network));
            }
        } else {
            return Err(nb::Error::Other(Error::Network));
        }

        // If no socket is found we stop here
        self.sockets
            .get::<TcpSocket<CLK, L>>(socket.clone().into())
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
            .get::<TcpSocket<CLK, L>>(socket.clone().into())
            .map_err(Self::Error::from)?;

        tcp.set_state(TcpState::WaitingForConnect);
        tcp.endpoint = remote;
        tcp.meta.handle = SocketHandle(peer_handle);
        *socket = SocketHandle(peer_handle);

        // TODO: Timeout?
        while {
            let tcp = self
                .sockets
                .get::<TcpSocket<CLK, L>>(socket.clone().into())
                .map_err(Self::Error::from)?;
            matches!(tcp.state(), TcpState::WaitingForConnect)
        } {
            self.spin()?;
        }
        Ok(())
    }

    /// Check if this socket is still connected
    fn is_connected(&mut self, socket: &Self::TcpSocket) -> Result<bool, Self::Error> {
        if let Some(ref con) = self.wifi_connection {
            if !self.initialized || !con.is_connected() {
                return Ok(false);
            }
        } else {
            return Ok(false);
        }

        let socket_ref = self
            .sockets
            .get::<TcpSocket<CLK, L>>(socket.clone().into())?;
        Ok(matches!(socket_ref.state(), TcpState::Connected))
    }

    /// Write to the stream. Returns the number of bytes written is returned
    /// (which may be less than `buffer.len()`), or an error.
    fn send(
        &mut self,
        socket: &mut Self::TcpSocket,
        buffer: &[u8],
    ) -> nb::Result<usize, Self::Error> {
        if let Some(ref con) = self.wifi_connection {
            if !self.initialized || !con.is_connected() {
                return Err(nb::Error::Other(Error::Network));
            }
        } else {
            return Err(nb::Error::Other(Error::Network));
        }

        let tcp = self
            .sockets
            .get::<TcpSocket<CLK, L>>(socket.clone().into())
            .map_err(|e| nb::Error::Other(e.into()))?;

        if !matches!(tcp.state(), TcpState::Connected) {
            return Err(nb::Error::Other(Error::SocketNotConnected));
        }

        let channel = tcp.channel_id().0;

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
    }

    fn receive(
        &mut self,
        socket: &mut Self::TcpSocket,
        buffer: &mut [u8],
    ) -> nb::Result<usize, Self::Error> {
        self.spin()?;

        let mut tcp = self
            .sockets
            .get::<TcpSocket<CLK, L>>((*socket).into())
            .map_err(|e| nb::Error::Other(e.into()))?;

        tcp.recv_slice(buffer)
            .map_err(|e| nb::Error::Other(e.into()))
    }

    /// Close an existing TCP socket.
    fn close(&mut self, socket: Self::TcpSocket) -> Result<(), Self::Error> {
        defmt::debug!("[TCP] Closing socket: {:?}", socket.0);
        // If the socket is not found it is already removed
        if let Some(ref tcp) = self.sockets.get::<TcpSocket<CLK, L>>(socket.into()).ok() {
            //If socket is not closed that means a connection excists which has to be closed
            if !matches!(
                tcp.state(),
                TcpState::ShutdownForWrite(_) | TcpState::Created
            ) {
                let peer_handle = tcp.handle().0;
                match self.send_at(ClosePeerConnection { peer_handle }) {
                    Err(Error::AT(atat::Error::InvalidResponse)) | Ok(_) => (),
                    Err(e) => return Err(e.into()),
                }
            } else {
                //No connection exists the socket should be removed from the set here
                self.sockets.remove(socket.into())?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
struct PeerUrlBuilder<'a> {
    hostname: Option<&'a str>,
    ip_addr: Option<IpAddr>,
    port: Option<u16>,
    ca: Option<&'a str>,
    cert: Option<&'a str>,
    pkey: Option<&'a str>,
    local_port: Option<u16>,
}

impl<'a> PeerUrlBuilder<'a> {
    fn new() -> Self {
        Self::default()
    }

    fn write_domain(&self, s: &mut String<128>) -> Result<(), Error> {
        let port = self.port.ok_or(Error::Network)?;
        let addr = self
            .ip_addr
            .and_then(|ip| write!(s, "{}/", SocketAddr::new(ip, port)).ok());
        let host = self
            .hostname
            .and_then(|host| write!(s, "{}:{}/", host, port).ok());

        addr.xor(host).ok_or(Error::Network)
    }

    fn udp(&self) -> Result<String<128>, Error> {
        let mut s = String::new();
        write!(&mut s, "udp://").ok();
        self.write_domain(&mut s)?;

        // Start writing query parameters
        write!(&mut s, "?").ok();
        self.local_port
            .map(|v| write!(&mut s, "local_port={}&", v).ok());
        // Remove trailing '&' or '?' if no query.
        s.pop();

        Ok(s)
    }

    fn tcp(&mut self) -> Result<String<128>, Error> {
        let mut s = String::new();
        write!(&mut s, "tcp://").ok();
        self.write_domain(&mut s)?;

        // Start writing query parameters
        write!(&mut s, "?").ok();
        self.local_port
            .map(|v| write!(&mut s, "local_port={}&", v).ok());
        self.ca.map(|v| write!(&mut s, "ca={}&", v).ok());
        self.cert.map(|v| write!(&mut s, "cert={}&", v).ok());
        self.pkey.map(|v| write!(&mut s, "privKey={}&", v).ok());
        // Remove trailing '&' or '?' if no query.
        s.pop();

        Ok(s)
    }

    fn address(&mut self, addr: &SocketAddr) -> &mut Self {
        self.ip_addr(addr.ip()).port(addr.port())
    }

    #[allow(dead_code)]
    fn hostname(&mut self, hostname: &'a str) -> &mut Self {
        self.hostname.replace(hostname);
        self
    }

    /// maximum length 64
    fn ip_addr(&mut self, ip_addr: IpAddr) -> &mut Self {
        self.ip_addr.replace(ip_addr);
        self
    }

    /// port number
    fn port(&mut self, port: u16) -> &mut Self {
        self.port.replace(port);
        self
    }

    fn ca(&mut self, ca: &'a str) -> &mut Self {
        self.ca.replace(ca);
        self
    }

    fn cert(&mut self, cert: &'a str) -> &mut Self {
        self.cert.replace(cert);
        self
    }

    fn pkey(&mut self, pkey: &'a str) -> &mut Self {
        self.pkey.replace(pkey);
        self
    }

    #[allow(dead_code)]
    fn local_port(&mut self, local_port: u16) -> &mut Self {
        self.local_port.replace(local_port);
        self
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn udp_ipv4_url() {
        let address = "192.168.0.1:8080".parse().unwrap();
        let url = PeerUrlBuilder::new().address(&address).udp().unwrap();
        assert_eq!(url, "udp://192.168.0.1:8080/");
    }

    #[test]
    fn udp_ipv6_url() {
        let address = "[FE80:0000:0000:0000:0202:B3FF:FE1E:8329]:8080"
            .parse()
            .unwrap();
        let url = PeerUrlBuilder::new().address(&address).udp().unwrap();
        assert_eq!(url, "udp://[fe80::202:b3ff:fe1e:8329]:8080/");
    }

    #[test]
    fn udp_hostname_url() {
        let url = PeerUrlBuilder::new()
            .hostname("example.org")
            .port(2000)
            .local_port(2001)
            .udp()
            .unwrap();
        assert_eq!(url, "udp://example.org:2000/?local_port=2001");
    }

    #[test]
    fn tcp_certs() {
        let url = PeerUrlBuilder::new()
            .hostname("example.org")
            .port(2000)
            .ca("ca.crt")
            .cert("client.crt")
            .pkey("client.key")
            .tcp()
            .unwrap();
        assert_eq!(
            url,
            "tcp://example.org:2000/?ca=ca.crt&cert=client.crt&privKey=client.key"
        );
    }
}
