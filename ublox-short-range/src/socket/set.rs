use super::{AnySocket, Error, Result, Socket, SocketRef, SocketType};

use embedded_nal::SocketAddr;
use embedded_time::Clock;
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
    pub fn socket_type(&self, handle: Handle) -> Option<SocketType> {
        match self.sockets.iter().find_map(|i| {
            if let Some(ref s) = i {
                if s.handle().0 == handle.0 {
                    Some(s)
                } else {
                    None
                }
            } else {
                None
            }
        }) {
            Some(socket) => Some(socket.get_type()),
            None => None,
        }
    }

    /// Get the type of a specific socket in the set.
    ///
    /// Returned as a [`SocketType`]
    pub fn socket_type_by_channel_id(&self, channel_id: ChannelId) -> Option<SocketType> {
        match self.sockets.iter().find_map(|i| {
            if let Some(ref s) = i {
                if s.channel_id().0 == channel_id.0 {
                    Some(s)
                } else {
                    None
                }
            } else {
                None
            }
        }) {
            Some(socket) => Some(socket.get_type()),
            None => None,
        }
    }

    /// Get the type of a specific socket in the set.
    ///
    /// Returned as a [`SocketType`]
    pub fn socket_type_by_endpoint(&self, endpoint: &SocketAddr) -> Option<SocketType> {
        match self.sockets.iter().find_map(|i| {
            if let Some(ref s) = i {
                if s.endpoint() == endpoint {
                    Some(s)
                } else {
                    None
                }
            } else {
                None
            }
        }) {
            Some(socket) => Some(socket.get_type()),
            None => None,
        }
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

    /// Get a socket from the set by its handle, as mutable.
    pub fn get<U: AnySocket<CLK, L>>(&mut self, handle: Handle) -> Result<SocketRef<U>> {
        match self.sockets.iter_mut().find_map(|i| {
            if let Some(ref mut s) = i {
                if s.handle().0 == handle.0 {
                    Some(s)
                } else {
                    None
                }
            } else {
                None
            }
        }) {
            Some(socket) => Ok(U::downcast(SocketRef::new(socket))?),
            None => Err(Error::InvalidSocket),
        }
    }

    /// Get a socket from the set by its channel id, as mutable.
    pub fn get_by_channel<U: AnySocket<CLK, L>>(
        &mut self,
        channel_id: ChannelId,
    ) -> Result<SocketRef<U>> {
        match self.sockets.iter_mut().find_map(|i| {
            if let Some(ref mut s) = i {
                if s.channel_id().0 == channel_id.0 {
                    Some(s)
                } else {
                    None
                }
            } else {
                None
            }
        }) {
            Some(socket) => Ok(U::downcast(SocketRef::new(socket))?),
            None => Err(Error::InvalidSocket),
        }
    }

    /// Get a socket from the set by its endpoint, as mutable.
    pub fn get_by_endpoint<U: AnySocket<CLK, L>>(
        &mut self,
        endpoint: &SocketAddr,
    ) -> Result<SocketRef<U>> {
        match self.sockets.iter_mut().find_map(|i| {
            if let Some(ref mut s) = i {
                if s.endpoint() == endpoint {
                    Some(s)
                } else {
                    None
                }
            } else {
                None
            }
        }) {
            Some(socket) => Ok(U::downcast(SocketRef::new(socket))?),
            None => Err(Error::InvalidSocket),
        }
    }

    /// Remove a socket from the set, without changing its state.
    pub fn remove(&mut self, handle: Handle) -> Result<Socket<CLK, L>> {
        let index = self
            .sockets
            .iter_mut()
            .position(|i| {
                if let Some(s) = i {
                    return s.handle().0 == handle.0;
                }
                false
            })
            .ok_or(Error::InvalidSocket)?;

        let socket: &mut Option<Socket<CLK, L>> = unsafe { self.sockets.get_unchecked_mut(index) };

        socket.take().ok_or(Error::InvalidSocket)
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
