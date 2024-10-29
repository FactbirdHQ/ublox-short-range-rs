use crate::error::Error;
use core::fmt::Write;
use core::net::{IpAddr, SocketAddr};
use heapless::String;

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SecurityCredentials {
    pub ca_cert_name: heapless::String<16>,
    pub c_cert_name: heapless::String<16>,
    pub c_key_name: heapless::String<16>,
}

#[derive(Default)]
pub(crate) struct PeerUrlBuilder<'a> {
    hostname: Option<&'a str>,
    ip_addr: Option<IpAddr>,
    port: Option<u16>,
    creds: Option<&'a SecurityCredentials>,
    local_port: Option<u16>,
}

#[allow(dead_code)]
impl<'a> PeerUrlBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    fn write_domain<const N: usize>(&self, s: &mut String<N>) -> Result<(), Error> {
        let port = self.port.ok_or(Error::Network)?;
        let addr = self
            .ip_addr
            .and_then(|ip| write!(s, "{}/", SocketAddr::new(ip, port)).ok());
        let host = self
            .hostname
            .and_then(|host| write!(s, "{}:{}/", host, port).ok());

        addr.xor(host).ok_or(Error::Network)
    }

    pub fn udp<const N: usize>(&self) -> Result<String<N>, Error> {
        let mut s = String::new();
        write!(&mut s, "udp://").map_err(|_| Error::Overflow)?;
        self.write_domain(&mut s)?;

        // Start writing query parameters
        write!(&mut s, "?").map_err(|_| Error::Overflow)?;

        if let Some(v) = self.local_port {
            write!(&mut s, "local_port={}&", v).map_err(|_| Error::Overflow)?;
        }

        // Remove trailing '&' or '?' if no query.
        s.pop();

        Ok(s)
    }

    pub fn tcp<const N: usize>(&mut self) -> Result<String<N>, Error> {
        let mut s = String::new();
        write!(&mut s, "tcp://").map_err(|_| Error::Overflow)?;
        self.write_domain(&mut s)?;

        // Start writing query parameters
        write!(&mut s, "?").map_err(|_| Error::Overflow)?;

        if let Some(v) = self.local_port {
            write!(&mut s, "local_port={}&", v).map_err(|_| Error::Overflow)?;
        }

        if let Some(creds) = self.creds.as_ref() {
            write!(&mut s, "ca={}&", creds.ca_cert_name).map_err(|_| Error::Overflow)?;
            write!(&mut s, "cert={}&", creds.c_cert_name).map_err(|_| Error::Overflow)?;
            write!(&mut s, "privKey={}&", creds.c_key_name).map_err(|_| Error::Overflow)?;
        };

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

    pub fn set_hostname(&mut self, hostname: Option<&'a str>) -> &mut Self {
        self.hostname = hostname;
        self
    }

    /// maximum length 64
    pub fn ip_addr(&mut self, ip_addr: IpAddr) -> &mut Self {
        self.ip_addr.replace(ip_addr);
        self
    }

    pub fn set_ip_addr(&mut self, ip_addr: Option<IpAddr>) -> &mut Self {
        self.ip_addr = ip_addr;
        self
    }

    /// port number
    pub fn port(&mut self, port: u16) -> &mut Self {
        self.port.replace(port);
        self
    }

    pub fn set_port(&mut self, port: Option<u16>) -> &mut Self {
        self.port = port;
        self
    }

    pub fn creds(&mut self, creds: &'a SecurityCredentials) -> &mut Self {
        self.creds.replace(creds);
        self
    }

    pub fn local_port(&mut self, local_port: u16) -> &mut Self {
        self.local_port.replace(local_port);
        self
    }

    pub fn set_local_port(&mut self, local_port: Option<u16>) -> &mut Self {
        self.local_port = local_port;
        self
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn udp_ipv4_url() {
        let address = "192.168.0.1:8080".parse().unwrap();
        let url = PeerUrlBuilder::new()
            .address(&address)
            .udp::<128>()
            .unwrap();
        assert_eq!(url, "udp://192.168.0.1:8080/");
    }

    #[test]
    fn udp_ipv6_url() {
        let address = "[FE80:0000:0000:0000:0202:B3FF:FE1E:8329]:8080"
            .parse()
            .unwrap();
        let url = PeerUrlBuilder::new()
            .address(&address)
            .udp::<128>()
            .unwrap();
        assert_eq!(url, "udp://[fe80::202:b3ff:fe1e:8329]:8080/");
    }

    #[test]
    fn udp_hostname_url() {
        let url = PeerUrlBuilder::new()
            .hostname("example.org")
            .port(2000)
            .local_port(2001)
            .udp::<128>()
            .unwrap();
        assert_eq!(url, "udp://example.org:2000/?local_port=2001");
    }

    #[test]
    fn tcp_certs() {
        let url = PeerUrlBuilder::new()
            .hostname("example.org")
            .port(2000)
            .creds(&SecurityCredentials {
                c_cert_name: heapless::String::try_from("client.crt").unwrap(),
                ca_cert_name: heapless::String::try_from("ca.crt").unwrap(),
                c_key_name: heapless::String::try_from("client.key").unwrap(),
            })
            .tcp::<128>()
            .unwrap();

        assert_eq!(
            url,
            "tcp://example.org:2000/?ca=ca.crt&cert=client.crt&privKey=client.key"
        );
    }
}
