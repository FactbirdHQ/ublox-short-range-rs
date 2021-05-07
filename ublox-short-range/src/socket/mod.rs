mod meta;
mod ref_;
mod ring_buffer;
mod set;
pub mod tcp;
pub mod udp;

pub(crate) use self::meta::Meta as SocketMeta;
pub use self::ring_buffer::RingBuffer;
use embedded_nal::SocketAddr;
use embedded_time::Clock;

#[cfg(feature = "socket-tcp")]
pub use tcp::{State as TcpState, TcpSocket};
#[cfg(feature = "socket-udp")]
pub use udp::{State as UdpState, UdpSocket};

pub use self::ref_::Ref as SocketRef;
pub use self::set::{ChannelId, Handle as SocketHandle, Set as SocketSet};

/// The error type for the networking stack.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum Error {
    /// An operation cannot proceed because a buffer is empty or full.
    Exhausted,
    /// An operation is not permitted in the current state.
    Illegal,
    /// An endpoint or address of a remote host could not be translated to a lower level address.
    /// E.g. there was no an Ethernet address corresponding to an IPv4 address in the ARP cache,
    /// or a TCP connection attempt was made to an unspecified endpoint.
    Unaddressable,

    SocketSetFull,
    InvalidSocket,
    DuplicateSocket,
}

type Result<T> = core::result::Result<T, Error>;

/// A network socket.
///
/// This enumeration abstracts the various types of sockets based on the IP protocol.
/// To downcast a `Socket` value to a concrete socket, use the [AnySocket] trait,
/// e.g. to get `UdpSocket`, call `UdpSocket::downcast(socket)`.
///
/// It is usually more convenient to use [SocketSet::get] instead.
///
/// [AnySocket]: trait.AnySocket.html
/// [SocketSet::get]: struct.SocketSet.html#method.get
#[non_exhaustive]
pub enum Socket<CLK: Clock, const L: usize> {
    // #[cfg(feature = "socket-raw")]
    // Raw(RawSocket<'a, 'b>),
    // #[cfg(all(
    //     feature = "socket-icmp",
    //     any(feature = "proto-ipv4", feature = "proto-ipv6")
    // ))]
    // Icmp(IcmpSocket<'a, 'b>),
    #[cfg(feature = "socket-udp")]
    Udp(UdpSocket<CLK, L>),
    #[cfg(feature = "socket-tcp")]
    Tcp(TcpSocket<CLK, L>),
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum SocketType {
    Udp,
    Tcp,
}

impl<CLK: Clock, const L: usize> Socket<CLK, L> {
    /// Return the socket handle.
    #[inline]
    pub fn handle(&self) -> SocketHandle {
        self.meta().handle
    }

    /// Return the socket channel id.
    #[inline]
    pub fn channel_id(&self) -> ChannelId {
        self.meta().channel_id
    }

    /// Return the socket address.
    pub fn endpoint(&self) -> &SocketAddr {
        match self {
            // #[cfg(feature = "socket-raw")]
            // Socket::Raw(ref $( $mut_ )* $socket) => $code,
            // #[cfg(all(feature = "socket-icmp", any(feature = "proto-ipv4", feature = "proto-ipv6")))]
            // Socket::Icmp(ref $( $mut_ )* $socket) => $code,
            #[cfg(feature = "socket-udp")]
            Socket::Udp(ref socket) => &socket.endpoint,
            #[cfg(feature = "socket-tcp")]
            Socket::Tcp(ref socket) => &socket.endpoint,
        }
    }

    pub fn get_type(&self) -> SocketType {
        match self {
            // #[cfg(feature = "socket-raw")]
            // Socket::Raw(ref $( $mut_ )* $socket) => $code,
            // #[cfg(all(feature = "socket-icmp", any(feature = "proto-ipv4", feature = "proto-ipv6")))]
            // Socket::Icmp(ref $( $mut_ )* $socket) => $code,
            #[cfg(feature = "socket-udp")]
            Socket::Udp(_) => SocketType::Tcp,
            #[cfg(feature = "socket-tcp")]
            Socket::Tcp(_) => SocketType::Udp,
        }
    }

    pub(crate) fn meta(&self) -> &SocketMeta {
        match self {
            // #[cfg(feature = "socket-raw")]
            // Socket::Raw(ref $( $mut_ )* $socket) => $code,
            // #[cfg(all(feature = "socket-icmp", any(feature = "proto-ipv4", feature = "proto-ipv6")))]
            // Socket::Icmp(ref $( $mut_ )* $socket) => $code,
            #[cfg(feature = "socket-udp")]
            Socket::Udp(ref socket) => &socket.meta,
            #[cfg(feature = "socket-tcp")]
            Socket::Tcp(ref socket) => &socket.meta,
        }
    }
}

/// A conversion trait for network sockets.
pub trait AnySocket<CLK: Clock, const L: usize>: Sized {
    fn downcast(socket_ref: SocketRef<'_, Socket<CLK, L>>) -> Result<SocketRef<'_, Self>>;
}

#[cfg(feature = "socket-tcp")]
impl<CLK: Clock, const L: usize> AnySocket<CLK, L> for TcpSocket<CLK, L> {
    fn downcast(ref_: SocketRef<'_, Socket<CLK, L>>) -> Result<SocketRef<'_, Self>> {
        match SocketRef::into_inner(ref_) {
            Socket::Tcp(ref mut socket) => Ok(SocketRef::new(socket)),
            _ => Err(Error::Illegal),
        }
    }
}

#[cfg(feature = "socket-udp")]
impl<CLK: Clock, const L: usize> AnySocket<CLK, L> for UdpSocket<CLK, L> {
    fn downcast(ref_: SocketRef<'_, Socket<CLK, L>>) -> Result<SocketRef<'_, Self>> {
        match SocketRef::into_inner(ref_) {
            Socket::Udp(ref mut socket) => Ok(SocketRef::new(socket)),
            _ => Err(Error::Illegal),
        }
    }
}
