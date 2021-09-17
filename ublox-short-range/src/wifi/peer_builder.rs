use crate::error::Error;
use core::fmt::Write;
/// Handles receiving data from sockets
/// implements TCP and UDP for WiFi client
use embedded_nal::{IpAddr, SocketAddr};
use heapless::String;

#[derive(Debug, Default)]
pub(crate) struct PeerUrlBuilder<'a> {
    hostname: Option<&'a str>,
    ip_addr: Option<IpAddr>,
    port: Option<u16>,
    ca: Option<&'a str>,
    cert: Option<&'a str>,
    pkey: Option<&'a str>,
    local_port: Option<u16>,
}

#[allow(dead_code)]
impl<'a> PeerUrlBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn write_domain(&self, s: &mut String<128>) -> Result<(), Error> {
        let port = self.port.ok_or(Error::Network)?;
        let addr = self
            .ip_addr
            .and_then(|ip| write!(s, "{}/", SocketAddr::new(ip, port)).ok());
        let host = self
            .hostname
            .and_then(|host| write!(s, "{}:{}/", host, port).ok());

        addr.xor(host).ok_or(Error::Network)
    }

    pub fn udp(&self) -> Result<String<128>, Error> {
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

    pub fn tcp(&mut self) -> Result<String<128>, Error> {
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

    pub fn address(&mut self, addr: &SocketAddr) -> &mut Self {
        self.ip_addr(addr.ip()).port(addr.port())
    }

    pub fn hostname(&mut self, hostname: &'a str) -> &mut Self {
        self.hostname.replace(hostname);
        self
    }

    /// maximum length 64
    pub fn ip_addr(&mut self, ip_addr: IpAddr) -> &mut Self {
        self.ip_addr.replace(ip_addr);
        self
    }

    /// port number
    pub fn port(&mut self, port: u16) -> &mut Self {
        self.port.replace(port);
        self
    }

    pub fn ca(&mut self, ca: &'a str) -> &mut Self {
        self.ca.replace(ca);
        self
    }

    pub fn cert(&mut self, cert: &'a str) -> &mut Self {
        self.cert.replace(cert);
        self
    }

    pub fn pkey(&mut self, pkey: &'a str) -> &mut Self {
        self.pkey.replace(pkey);
        self
    }

    pub fn local_port(&mut self, local_port: u16) -> &mut Self {
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
