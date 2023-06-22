use core::task::Poll;

use atat::asynch::AtatClient;
use embedded_nal_async::AddrType;
use futures::Future;
use no_std_net::IpAddr;

use crate::command::ping::Ping;

use super::UbloxStack;

struct DnsFuture<'a, AT: AtatClient + 'static> {
    stack: &'a UbloxStack<AT>,
}

impl<'a, AT: AtatClient> Future for DnsFuture<'a, AT> {
    type Output = Result<IpAddr, Error>;

    fn poll(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> Poll<Self::Output> {
        // let i = &mut *self.stack.inner.borrow_mut();
        // match i.dns_result {
        //     Some(Ok(ip)) => Poll::Ready(Ok(ip)),
        //     Some(Err(_)) => Poll::Ready(Err(Error::Failed)),
        //     None => {
        //         i.dns_waker.register(cx.waker());
        Poll::Pending
        //     }
        // }
    }
}

/// Errors returned by DnsSocket.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    /// Invalid name
    InvalidName,
    /// Name too long
    NameTooLong,
    /// Name lookup failed
    Failed,
}

/// DNS client compatible with the `embedded-nal-async` traits.
///
/// This exists only for compatibility with crates that use `embedded-nal-async`.
/// Prefer using [`Stack::dns_query`](crate::Stack::dns_query) directly if you're
/// not using `embedded-nal-async`.
pub struct DnsSocket<'a, AT: AtatClient + 'static> {
    stack: &'a UbloxStack<AT>,
}

impl<'a, AT: AtatClient> DnsSocket<'a, AT> {
    /// Create a new DNS socket using the provided stack.
    ///
    /// NOTE: If using DHCP, make sure it has reconfigured the stack to ensure the DNS servers are updated.
    pub fn new(stack: &'a UbloxStack<AT>) -> Self {
        Self { stack }
    }

    /// Make a query for a given name and return the corresponding IP addresses.
    pub async fn query(&self, name: &str, addr_type: AddrType) -> Result<IpAddr, Error> {
        match addr_type {
            AddrType::IPv4 => {
                if let Ok(ip) = name.parse().map(IpAddr::V4) {
                    return Ok(ip);
                }
            }
            AddrType::IPv6 => {
                if let Ok(ip) = name.parse().map(IpAddr::V6) {
                    return Ok(ip);
                }
            }
            _ => {}
        }

        // let i = &mut *self.stack.inner.borrow_mut();
        // i.dns_result = None;
        // i.device
        //     .at
        //     .send_edm(Ping {
        //         hostname: name,
        //         retry_num: 1,
        //     })
        //     .await
        //     .map_err(|_| Error::Failed)?;

        DnsFuture { stack: self.stack }.await
    }
}

// #[cfg(all(feature = "unstable-traits", feature = "nightly"))]
impl<'a, AT: AtatClient> embedded_nal_async::Dns for DnsSocket<'a, AT> {
    type Error = Error;

    async fn get_host_by_name(
        &self,
        host: &str,
        addr_type: AddrType,
    ) -> Result<IpAddr, Self::Error> {
        self.query(host, addr_type).await
    }

    async fn get_host_by_address(
        &self,
        _addr: embedded_nal_async::IpAddr,
    ) -> Result<heapless::String<256>, Self::Error> {
        unimplemented!()
    }
}
