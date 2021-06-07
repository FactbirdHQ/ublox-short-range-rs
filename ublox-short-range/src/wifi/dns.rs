use crate::client::DNSState;
use embedded_nal::{AddrType, Dns, IpAddr};
use heapless::{consts, ArrayLength, String};

use crate::{
    command::ping::*,
    // command::dns::{self, types::ResolutionType},
    error::Error,
    UbloxClient,
};

impl<C, N, L> Dns for UbloxClient<C, N, L>
where
    C: atat::AtatClient,
    N: ArrayLength<Option<crate::sockets::SocketSetItem<L>>>,
    L: ArrayLength<u8>,
{
    type Error = Error;

    fn gethostbyaddr(&self, _ip_addr: IpAddr) -> Result<String<consts::U256>, Self::Error> {
        Err(Error::Unimplemented)
    }

    fn gethostbyname(&self, hostname: &str, _addr_type: AddrType) -> Result<IpAddr, Self::Error> {
        self.dns_state.set(DNSState::Resolving);
        self.send_at(Ping {
            hostname: hostname,
            retry_num: 1,
        })?;
        while self.dns_state.get() == DNSState::Resolving {
            self.spin()?;
        }

        match self.dns_state.get() {
            DNSState::Resolved(ip) => Ok(ip),
            DNSState::Error(e) => Err(Error::Dns(e)),
            _ => Err(Error::Dns(types::PingError::Other)),
        }
    }
}
