use crate::client::DNSState;
use core::convert::TryInto;
use embedded_hal::digital::OutputPin;
use embedded_nal::{nb, AddrType, Dns, IpAddr};
use embedded_time::duration::{Extensions, Generic, Milliseconds};
use embedded_time::Clock;
use heapless::String;

use crate::{command::ping::*, error::Error, UbloxClient};

impl<C, CLK, RST, const N: usize, const L: usize> Dns for UbloxClient<C, CLK, RST, N, L>
where
    C: atat::AtatClient,
    CLK: Clock,
    RST: OutputPin,
    Generic<CLK::T>: TryInto<Milliseconds>,
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
        self.dns_state = DNSState::Resolving;
        self.send_at(Ping {
            hostname,
            retry_num: 1,
        })?;

        let expiration = self
            .timer
            .try_now()
            .map_err(|_| nb::Error::Other(Error::Timer))?
            + 5_u32.seconds();

        while self.dns_state == DNSState::Resolving {
            self.spin()?;

            if self
                .timer
                .try_now()
                .map_err(|_| nb::Error::Other(Error::Timer))?
                >= expiration
            {
                return Err(Error::Dns(types::PingError::Timeout).into());
            }
        }

        match self.dns_state {
            DNSState::Resolved(ip) => Ok(ip),
            DNSState::Error(e) => Err(Error::Dns(e).into()),
            _ => Err(Error::Dns(types::PingError::Other).into()),
        }
    }
}
