use atat::AtatClient;
use core::fmt::Write;
use embedded_nal::{AddrType, Dns};
use heapless::{consts, ArrayLength, String};
use no_std_net::IpAddr;
use crate::client::DNSState;

use crate::{
    // command::dns::{self, types::ResolutionType},
    error::Error,
    UbloxClient,
    command::ping::*,
};

impl<C, N, L> Dns for UbloxClient<C, N, L>
where
    C: atat::AtatClient,
    N: ArrayLength<Option<crate::sockets::SocketSetItem<L>>>,
    L: ArrayLength<u8>,
{
    type Error = Error;

    fn gethostbyaddr(&self, ip_addr: IpAddr) -> Result<String<consts::U256>, Self::Error> {
        // let mut ip_str = String::<consts::U256>::new();
        // write!(&mut ip_str, "{}", ip_addr).map_err(|_| Error::BadLength)?;

        // let resp = self.send_at(Ping {
        //     hostname: hostname,
        //     retry_num: 1,
        // })?;

        // Ok(String::from(resp.ip_domain_string.as_str()))
        Err(Error::Dns(types::PingError::Other))
    }

    fn gethostbyname(&self, hostname: &str, addr_type: AddrType) -> Result<IpAddr, Self::Error> {
        // if addr_type == AddrType::IPv6 {
        //     return Err(Error::Dns);
        // }
        self.dns_state.set(DNSState::Resolving);
        self.send_at(Ping {
            hostname: hostname,
            retry_num: 1,
        })?;
        while self.dns_state.get() == DNSState::Resolving { 
            self.spin();
        }

        match self.dns_state.get(){
            DNSState::Resolved(ip) => Ok(ip),
            DNSState::Error(e) => Err(Error::Dns(e)),
            _ => Err(Error::Dns(types::PingError::Other)),
        }
    }
}
