use core::{cell::RefCell, future::poll_fn, task::Poll};

use embassy_sync::waitqueue::WakerRegistration;
use embedded_nal_async::AddrType;
use no_std_net::IpAddr;

use crate::command::ping::types::PingError;

use super::{SocketStack, UbloxStack};

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

/// From u-connectXpress AT commands manual:
/// <domain> depends on the <scheme>. For internet domain names, the maximum
/// length is 64 characters.
/// Domain name length is 128 for NINA-W13 and NINA-W15 software version 4.0
/// .0 or later.
#[cfg(not(feature = "nina_w1xx"))]
pub const MAX_DOMAIN_NAME_LENGTH: usize = 64;

#[cfg(feature = "nina_w1xx")]
pub const MAX_DOMAIN_NAME_LENGTH: usize = 128;

pub struct DnsTableEntry {
    pub domain_name: heapless::String<MAX_DOMAIN_NAME_LENGTH>,
    pub state: DnsState,
    pub waker: WakerRegistration,
}

#[derive(PartialEq, Clone)]
pub enum DnsState {
    New,
    Pending,
    Resolved(IpAddr),
    Error(PingError),
}

impl DnsTableEntry {
    pub const fn new(domain_name: heapless::String<MAX_DOMAIN_NAME_LENGTH>) -> Self {
        Self {
            domain_name,
            state: DnsState::New,
            waker: WakerRegistration::new(),
        }
    }
}

pub struct DnsTable {
    pub table: heapless::Deque<DnsTableEntry, 4>,
}

impl DnsTable {
    pub const fn new() -> Self {
        Self {
            table: heapless::Deque::new(),
        }
    }
    pub fn upsert(&mut self, new_entry: DnsTableEntry) {
        if let Some(entry) = self
            .table
            .iter_mut()
            .find(|e| e.domain_name == new_entry.domain_name)
        {
            entry.state = new_entry.state;
            return;
        }

        if self.table.is_full() {
            self.table.pop_front();
        }
        unsafe {
            self.table.push_back_unchecked(new_entry);
        }
    }

    pub fn get(&self, domain_name: &str) -> Option<&DnsTableEntry> {
        self.table
            .iter()
            .find(|e| e.domain_name.as_str() == domain_name)
    }

    pub fn get_mut(&mut self, domain_name: &str) -> Option<&mut DnsTableEntry> {
        self.table
            .iter_mut()
            .find(|e| e.domain_name.as_str() == domain_name)
    }

    pub fn reverse_lookup(&self, ip: IpAddr) -> Option<&str> {
        self.table
            .iter()
            .find(|e| e.state == DnsState::Resolved(ip))
            .map(|e| e.domain_name.as_str())
    }
}

/// DNS client compatible with the `embedded-nal-async` traits.
///
/// This exists only for compatibility with crates that use `embedded-nal-async`.
/// Prefer using [`Stack::dns_query`](crate::Stack::dns_query) directly if you're
/// not using `embedded-nal-async`.
pub struct DnsSocket<'a> {
    stack: &'a RefCell<SocketStack>,
}

impl<'a> DnsSocket<'a> {
    /// Create a new DNS socket using the provided stack.
    pub fn new<const INGRESS_BUF_SIZE: usize, const URC_CAPACITY: usize>(
        stack: &'a UbloxStack<INGRESS_BUF_SIZE, URC_CAPACITY>,
    ) -> Self {
        Self {
            stack: &stack.socket,
        }
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

        let name_string = heapless::String::try_from(name).map_err(|_| Error::NameTooLong)?;

        {
            let mut s = self.stack.borrow_mut();
            s.dns_table.upsert(DnsTableEntry::new(name_string.clone()));
            s.waker.wake();
        }

        poll_fn(|cx| {
            let mut s = self.stack.borrow_mut();
            let query = s.dns_table.get_mut(&name_string).unwrap();
            match query.state {
                DnsState::Resolved(ip) => Poll::Ready(Ok(ip)),
                DnsState::Error(_e) => Poll::Ready(Err(Error::Failed)),
                _ => {
                    query.waker.register(cx.waker());
                    Poll::Pending
                }
            }
        })
        .await
    }
}

impl<'a> embedded_nal_async::Dns for DnsSocket<'a> {
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
        _addr: IpAddr,
        _result: &mut [u8],
    ) -> Result<usize, Self::Error> {
        unimplemented!()
    }
}
