use crate::client::{DNSState, DNSTableEntry};
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

        self.dns_table.upsert(DNSTableEntry::new(
            DNSState::Resolving,
            String::from(hostname),
        ));

        let expiration = self.timer.now() + 8.secs();

        while let Some(DNSState::Resolving) = self.dns_table.get_state(String::from(hostname)) {
            self.spin().map_err(|_| nb::Error::Other(Error::Illegal))?;

            if self.timer.now() >= expiration {
                break;
            }
        }

        match self.dns_table.get_state(String::from(hostname)) {
            Some(DNSState::Resolved(ip)) => Ok(ip),
            Some(DNSState::Resolving) => {
                self.dns_table.upsert(DNSTableEntry::new(
                    DNSState::Error(types::PingError::Timeout),
                    String::from(hostname),
                ));
                Err(nb::Error::Other(Error::Timeout))
            }
            _ => Err(nb::Error::Other(Error::Illegal)),
        }
    }
}
