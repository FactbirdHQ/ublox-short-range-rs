use super::{AnySocket, Error, Result, Socket, SocketRef, SocketType};
use core::convert::TryInto;
use embedded_nal::SocketAddr;
use embedded_time::duration::{Generic, Milliseconds};
use embedded_time::{Clock, Instant};
use heapless::Vec;
use serde::{Deserialize, Serialize};

/// A handle, identifying a socket in a set.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Default,
    Serialize,
    Deserialize,
    defmt::Format,
)]
pub struct Handle(pub usize);

/// A channel id, identifying a socket in a set.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Default,
    Serialize,
    Deserialize,
    defmt::Format,
)]
pub struct ChannelId(pub u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketIndicator<'a> {
    Handle(usize),
    ChannelId(u8),
    Endpoint(&'a SocketAddr),
}

impl<'a> From<Handle> for SocketIndicator<'a> {
    fn from(handle: Handle) -> Self {
        Self::Handle(handle.0)
    }
}

impl<'a> defmt::Format for SocketIndicator<'a> {
    fn format(&self, fmt: defmt::Formatter) {
        match self {
            SocketIndicator::Handle(n) => defmt::write!(fmt, "Handle({})", n,),
            SocketIndicator::ChannelId(n) => defmt::write!(fmt, "ChannelId({})", n,),
            SocketIndicator::Endpoint(e) => match e {
                SocketAddr::V4(v4) => {
                    defmt::write!(fmt, "Endpoint(v4:{}:{})", <u32>::from(*v4.ip()), v4.port())
                }
                SocketAddr::V6(v6) => {
                    defmt::write!(fmt, "Endpoint(v6:{}:{})", <u128>::from(*v6.ip()), v6.port())
                }
            },
        };
    }
}

/// An extensible set of sockets.
#[derive(Default)]
pub struct Set<CLK: Clock, const N: usize, const L: usize> {
    pub sockets: Vec<Option<Socket<CLK, L>>, N>,
}

impl<CLK: Clock, const N: usize, const L: usize> Set<CLK, N, L> {
    /// Create a socket set using the provided storage.
    pub fn new() -> Self {
        let mut sockets = Vec::new();
        while sockets.len() < N {
            sockets.push(None).ok();
        }
        Set { sockets }
    }

    /// Get the maximum number of sockets the set can hold
    pub fn capacity(&self) -> usize {
        N
    }

    /// Get the current number of initialized sockets, the set is holding
    pub fn len(&self) -> usize {
        self.sockets.iter().filter(|a| a.is_some()).count()
    }

    /// Check if the set is currently holding no active sockets
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get the type of a specific socket in the set.
    ///
    /// Returned as a [`SocketType`]
    pub fn socket_type(&self, indicator: SocketIndicator) -> Option<SocketType> {
        if let Ok(index) = self.index_of(indicator) {
            if let Some(socket) = self.sockets.get(index) {
                return socket.as_ref().map(|s| s.get_type());
            }
        }

        None
    }

    /// Add a socket to the set with the reference count 1, and return its handle.
    pub fn add<U>(&mut self, socket: U) -> Result<Handle>
    where
        U: Into<Socket<CLK, L>>,
    {
        let socket = socket.into();
        for slot in self.sockets.iter_mut() {
            if slot.is_none() {
                let handle = socket.handle();
                *slot = Some(socket);
                return Ok(handle);
            }
        }
        Err(Error::SocketSetFull)
    }

    /// Get a socket from the set by its indicator, as mutable.
    pub fn get<T: AnySocket<CLK, L>>(
        &mut self,
        indicator: SocketIndicator,
    ) -> Result<SocketRef<T>> {
        let index = self.index_of(indicator)?;

        match self.sockets.get_mut(index).ok_or(Error::InvalidSocket)? {
            Some(socket) => Ok(T::downcast(SocketRef::new(socket))?),
            None => Err(Error::InvalidSocket),
        }
    }

    /// Get the index of a given socket in the set.
    fn index_of(&self, indicator: SocketIndicator) -> Result<usize> {
        self.sockets
            .iter()
            .position(|i| {
                i.as_ref().map_or(false, |s| match indicator {
                    SocketIndicator::ChannelId(id) => s.channel_id().0 == id,
                    SocketIndicator::Handle(handle) => s.handle().0 == handle,
                    SocketIndicator::Endpoint(add) => *s.endpoint() == *add,
                })
            })
            .ok_or(Error::InvalidSocket)
    }

    /// Remove a socket from the set
    pub fn remove(&mut self, indicator: SocketIndicator) -> Result<()> {
        let index = self.index_of(indicator)?;
        let socket = self.sockets.get_mut(index).ok_or(Error::InvalidSocket)?;

        defmt::trace!(
            "Removing socket! {} {}",
            indicator,
            socket.as_ref().map(|i| i.get_type())
        );

        socket.take().ok_or(Error::InvalidSocket)?;
        Ok(())
    }

    // /// Prune the sockets in this set.
    // ///
    // /// Pruning affects sockets with reference count 0. Open sockets are closed.
    // /// Closed sockets are removed and dropped.
    // pub fn prune(&mut self) {
    //     for (_, item) in self.sockets.iter_mut() {
    //         let mut may_remove = false;
    //         if let Item {
    //             refs: 0,
    //             ref mut socket,
    //         } = item
    //         {
    //             match *socket {
    //                 #[cfg(feature = "socket-raw")]
    //                 Socket::Raw(_) => may_remove = true,
    //                 #[cfg(all(
    //                     feature = "socket-icmp",
    //                     any(feature = "proto-ipv4", feature = "proto-ipv6")
    //                 ))]
    //                 Socket::Icmp(_) => may_remove = true,
    //                 #[cfg(feature = "socket-udp")]
    //                 Socket::Udp(_) => may_remove = true,
    //                 #[cfg(feature = "socket-tcp")]
    //                 Socket::Tcp(ref mut socket) => {
    //                     if socket.state() == TcpState::Closed {
    //                         may_remove = true
    //                     } else {
    //                         socket.close()
    //                     }
    //                 }
    //             }
    //         }
    //         if may_remove {
    //             *item = None
    //         }
    //     }
    // }

    pub fn recycle(&mut self, ts: &Instant<CLK>) -> bool
    where
        Generic<CLK::T>: TryInto<Milliseconds>,
    {
        let h = self.iter().find(|(_, s)| s.recycle(ts)).map(|(h, _)| h);
        if h.is_none() {
            return false;
        }
        self.remove(h.unwrap().into()).is_ok()
    }

    /// Iterate every socket in this set.
    pub fn iter(&self) -> impl Iterator<Item = (Handle, &Socket<CLK, L>)> {
        self.sockets.iter().filter_map(|slot| {
            if let Some(socket) = slot {
                Some((Handle(socket.handle().0), socket))
            } else {
                None
            }
        })
    }

    /// Iterate every socket in this set, as SocketRef.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Handle, SocketRef<Socket<CLK, L>>)> {
        self.sockets.iter_mut().filter_map(|slot| {
            if let Some(socket) = slot {
                Some((Handle(socket.handle().0), SocketRef::new(socket)))
            } else {
                None
            }
        })
    }
}

#[cfg(test)]
mod tests {}
