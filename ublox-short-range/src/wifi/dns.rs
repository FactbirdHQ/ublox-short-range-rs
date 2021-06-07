use crate::client::DNSState;
use embedded_nal::{AddrType, Dns, IpAddr};
use heapless::String;

use crate::{
    command::ping::*,
    // command::dns::{self, types::ResolutionType},
    error::Error,
    UbloxClient,
};

impl<C, const N: usize, const L: usize> Dns for UbloxClient<C, N, L>
where
    C: atat::AtatClient,
{
    type Error = Error;

    fn get_host_by_address(&mut self, _ip_addr: IpAddr) -> nb::Result<String<256>, Self::Error> {
        Err(Error::Unimplemented.into())
    }

    fn get_host_by_name(
        &mut self,
        hostname: &str,
        _addr_type: AddrType,
    ) -> nb::Result<IpAddr, Self::Error> {
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
            DNSState::Error(e) => Err(Error::Dns(e).into()),
            _ => Err(Error::Dns(types::PingError::Other).into()),
        }
    }
}
