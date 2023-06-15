//! DNS client compatible with the `embedded-nal-async` traits.
//!
//! This exists only for compatibility with crates that use `embedded-nal-async`.
//! Prefer using [`Stack::dns_query`](crate::Stack::dns_query) directly if you're
//! not using `embedded-nal-async`.

use no_std_net::IpAddr;

use super::UbloxStack;

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
pub struct DnsSocket<'a> {
    stack: &'a UbloxStack,
}

impl<'a> DnsSocket<'a> {
    /// Create a new DNS socket using the provided stack.
    ///
    /// NOTE: If using DHCP, make sure it has reconfigured the stack to ensure the DNS servers are updated.
    pub fn new(stack: &'a UbloxStack) -> Self {
        Self { stack }
    }

    /// Make a query for a given name and return the corresponding IP addresses.
    pub async fn query(
        &self,
        name: &str,
        addr_type: embedded_nal_async::AddrType,
    ) -> Result<IpAddr, Error> {
        let mut addrs = self.stack.dns_query(name, addr_type).await?;
        if addrs.len() < 1 {
            return Err(Error::Failed);
        }
        Ok(addrs.swap_remove(0))
    }
}

// #[cfg(all(feature = "unstable-traits", feature = "nightly"))]
impl<'a> embedded_nal_async::Dns for DnsSocket<'a> {
    type Error = Error;

    async fn get_host_by_name(
        &self,
        host: &str,
        addr_type: embedded_nal_async::AddrType,
    ) -> Result<IpAddr, Self::Error> {
        self.query(host, addr_type).await
    }

    async fn get_host_by_address(
        &self,
        _addr: embedded_nal_async::IpAddr,
    ) -> Result<heapless::String<256>, Self::Error> {
        todo!()
    }
}
