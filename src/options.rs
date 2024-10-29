use core::net::Ipv4Addr;
use heapless::Vec;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
/// Channel to broadcast wireless hotspot on.
pub enum Channel {
    /// Channel 1
    One = 1,
    /// Channel 2
    Two = 2,
    /// Channel 3
    Three = 3,
    /// Channel 4
    Four = 4,
    /// Channel 5
    Five = 5,
    /// Channel 6
    Six = 6,
}

#[allow(dead_code)]
#[derive(Debug)]
/// Band type of wireless hotspot.
pub enum Band {
    /// Band `A`
    A,
    /// Band `BG`
    Bg,
}

#[derive(Debug, Default)]
pub struct HotspotOptions {
    pub(crate) channel: Option<Channel>,
    pub(crate) band: Option<Band>,
    pub(crate) dhcp_server: bool,
}

impl HotspotOptions {
    pub fn new() -> Self {
        Self {
            channel: Some(Channel::One),
            band: Some(Band::Bg),
            dhcp_server: true,
        }
    }

    pub fn channel(mut self, channel: Channel) -> Self {
        self.channel = Some(channel);
        self
    }

    pub fn band(mut self, band: Band) -> Self {
        self.band = Some(band);
        self
    }

    pub fn dhcp_server(mut self, dhcp_server: bool) -> Self {
        self.dhcp_server = dhcp_server;
        self
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum WifiAuthentication<'a> {
    #[default]
    None,
    Wpa2Passphrase(&'a str),
    // Wpa2Psk(&'a [u8; 32]),
}

impl<'a> From<&'a str> for WifiAuthentication<'a> {
    fn from(s: &'a str) -> Self {
        Self::Wpa2Passphrase(s)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]

pub struct ConnectionOptions<'a> {
    pub ssid: &'a str,
    pub auth: WifiAuthentication<'a>,

    #[cfg_attr(feature = "defmt", defmt(Debug2Format))]
    pub ip: Option<Ipv4Addr>,
    #[cfg_attr(feature = "defmt", defmt(Debug2Format))]
    pub subnet: Option<Ipv4Addr>,
    #[cfg_attr(feature = "defmt", defmt(Debug2Format))]
    pub gateway: Option<Ipv4Addr>,
    #[cfg_attr(feature = "defmt", defmt(Debug2Format))]
    pub dns: Vec<Ipv4Addr, 2>,
}

impl<'a> ConnectionOptions<'a> {
    pub fn new(ssid: &'a str) -> Self {
        Self {
            ssid,
            ..Default::default()
        }
    }

    pub fn no_auth(mut self) -> Self {
        self.auth = WifiAuthentication::None;
        self
    }

    pub fn wpa2_passphrase(mut self, password: &'a str) -> Self {
        self.auth = WifiAuthentication::Wpa2Passphrase(password);
        self
    }

    pub fn ip_address(mut self, ip_addr: Ipv4Addr) -> Self {
        self.ip = Some(ip_addr);
        self
    }

    pub fn subnet_address(mut self, subnet_addr: Ipv4Addr) -> Self {
        self.subnet = Some(subnet_addr);

        self
    }

    pub fn gateway_address(mut self, gateway_addr: Ipv4Addr) -> Self {
        self.gateway = Some(gateway_addr);

        self
    }

    pub fn dns_server(mut self, dns_serv: Vec<Ipv4Addr, 2>) -> Self {
        self.dns = dns_serv;
        self
    }
}
