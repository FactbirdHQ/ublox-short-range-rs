use super::client::{DNSState, UbloxClient};
use super::timer;
use embassy_time::Duration;
use embedded_hal::digital::OutputPin;
use embedded_nal::{nb, AddrType, Dns, IpAddr};
use heapless::String;
use ublox_sockets::Error;

use crate::{blocking::timer::Timer, command::ping::*};

impl<C, RST, const N: usize, const L: usize> Dns for UbloxClient<C, RST, N, L>
where
    C: atat::blocking::AtatClient,
    RST: OutputPin,
{
    type Error = Error;

    fn get_host_by_address(&mut self, _ip_addr: IpAddr) -> nb::Result<String<256>, Self::Error> {
        unimplemented!()
    }

    fn get_host_by_name(
        &mut self,
        hostname: &str,
        _addr_type: AddrType,
    ) -> nb::Result<IpAddr, Self::Error> {
        debug!("Lookup hostname: {}", hostname);
        self.send_at(Ping {
            hostname,
            retry_num: 1,
        })
        .map_err(|_| nb::Error::Other(Error::Unaddressable))?;

        self.dns_state = DNSState::Resolving;

        match Timer::with_timeout(Duration::from_secs(8), || {
            if self.spin().is_err() {
                return Some(Err(Error::Illegal));
            }

            match self.dns_state {
                DNSState::Resolving => None,
                DNSState::Resolved(ip) => Some(Ok(ip)),
                _ => Some(Err(Error::Illegal)),
            }
        }) {
            Ok(ip) => Ok(ip),
            Err(timer::Error::Timeout) => Err(nb::Error::Other(Error::Timeout)),
            Err(timer::Error::Other(e)) => Err(nb::Error::Other(e)),
        }
    }
}
