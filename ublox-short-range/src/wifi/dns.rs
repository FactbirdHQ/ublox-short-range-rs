use crate::client::DNSState;
use atat::clock::Clock;
use embedded_hal::digital::blocking::OutputPin;
use embedded_nal::{nb, AddrType, Dns, IpAddr};
use fugit::ExtU32;
use heapless::String;

use crate::{command::ping::*, UbloxClient};
use ublox_sockets::Error;

impl<C, CLK, RST, const TIMER_HZ: u32, const N: usize, const L: usize> Dns
    for UbloxClient<C, CLK, RST, TIMER_HZ, N, L>
where
    C: atat::AtatClient,
    CLK: Clock<TIMER_HZ>,
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
        defmt::debug!("Lookup hostname: {}", hostname);
        self.send_at(Ping {
            hostname,
            retry_num: 1,
        })
        .map_err(|_| nb::Error::Other(Error::Unaddressable))?;
        self.dns_state = DNSState::Resolving;

        let expiration = self.timer.now() + 8.secs();

        while self.dns_state == DNSState::Resolving {
            self.spin().map_err(|_| nb::Error::Other(Error::Illegal))?;

            if self.timer.now() >= expiration {
                return Err(nb::Error::Other(Error::Timeout));
            }
        }

        match self.dns_state {
            DNSState::Resolved(ip) => Ok(ip),
            _ => Err(nb::Error::Other(Error::Illegal)),
        }
    }
}
