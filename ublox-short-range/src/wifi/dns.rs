use crate::client::{DNSState, DNSTableEntry};
use atat::blocking::AtatClient;
use embassy_time::{Duration, Instant};
use embedded_hal::digital::OutputPin;
use embedded_nal::{nb, AddrType, Dns, IpAddr};
use heapless::String;

use crate::{command::ping::*, UbloxClient};
use ublox_sockets::Error;

impl<'buf, 'sub, AtCl, AtUrcCh, RST, const N: usize, const L: usize> Dns
    for UbloxClient<'buf, 'sub, AtCl, AtUrcCh, RST, N, L>
where
    'buf: 'sub,
    AtCl: AtatClient,
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

        let expiration = Instant::now() + Duration::from_secs(8);

        while let Some(DNSState::Resolving) = self.dns_table.get_state(String::from(hostname)) {
            self.spin().map_err(|_| nb::Error::Other(Error::Illegal))?;

            if Instant::now() >= expiration {
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
