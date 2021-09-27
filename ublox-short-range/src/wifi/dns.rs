use crate::client::DNSState;
use core::convert::TryInto;
use embedded_hal::digital::OutputPin;
use embedded_nal::{nb, AddrType, Dns, IpAddr};
use embedded_time::duration::{Extensions, Generic, Milliseconds};
use embedded_time::Clock;
use heapless::String;

use crate::{command::ping::*, UbloxClient};
use ublox_sockets::Error;

impl<C, CLK, RST, const N: usize, const L: usize> Dns for UbloxClient<C, CLK, RST, N, L>
where
    C: atat::AtatClient,
    CLK: Clock,
    RST: OutputPin,
    Generic<CLK::T>: TryInto<Milliseconds>,
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
        self.dns_state = DNSState::Resolving;
        self.send_at(Ping {
            hostname,
            retry_num: 1,
        })
        .map_err(|_| nb::Error::Other(Error::Unaddressable))?;

        let expiration = self
            .timer
            .try_now()
            .map_err(|_| nb::Error::Other(Error::Timer))?
            + 5_u32.seconds();

        while self.dns_state == DNSState::Resolving {
            self.spin().map_err(|_| nb::Error::Other(Error::Illegal))?;

            if self
                .timer
                .try_now()
                .map_err(|_| nb::Error::Other(Error::Timer))?
                >= expiration
            {
                return Err(nb::Error::Other(Error::Timeout));
            }
        }

        match self.dns_state {
            DNSState::Resolved(ip) => Ok(ip),
            _ => Err(nb::Error::Other(Error::Illegal)),
        }
    }
}
