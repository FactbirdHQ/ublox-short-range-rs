use super::{Error, Result};
use crate::socket::{ChannelId, RingBuffer, Socket, SocketHandle, SocketMeta};
use core::convert::TryInto;
use embedded_nal::{Ipv4Addr, SocketAddr, SocketAddrV4};
use embedded_time::duration::{Generic, Milliseconds, Seconds};
use embedded_time::{Clock, Instant};

/// A TCP socket ring buffer.
pub type SocketBuffer<const N: usize> = RingBuffer<u8, N>;
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum State<CLK: Clock> {
    Closed,
    Listen,
    SynSent,
    Established,
    CloseWait,
    TimeWait,
    /// Block all writes (Socket is closed by remote)
    ShutdownForWrite(Instant<CLK>),
}

impl<CLK: Clock> Default for State<CLK> {
    fn default() -> Self {
        State::Closed
    }
}

/// A Transmission Control Protocol socket.
///
/// A TCP socket may passively listen for connections or actively connect to another endpoint.
/// Note that, for listening sockets, there is no "backlog"; to be able to simultaneously
/// accept several connections, as many sockets must be allocated, or any new connection
/// attempts will be reset.
pub struct TcpSocket<CLK: Clock, const L: usize> {
    pub(crate) meta: SocketMeta,
    pub(crate) endpoint: SocketAddr,
    state: State<CLK>,
    rx_buffer: SocketBuffer<L>,
    read_timeout: Option<Seconds>,
}

impl<CLK: Clock, const L: usize> TcpSocket<CLK, L> {
    #[allow(unused_comparisons)] // small usize platforms always pass rx_capacity check
    /// Create a socket using the given buffers.
    pub fn new(socket_id: usize) -> Self {
        let mut meta = SocketMeta::default();
        meta.handle.0 = socket_id;
        TcpSocket {
            meta,
            endpoint: SocketAddrV4::new(Ipv4Addr::unspecified(), 0).into(),
            state: State::default(),
            rx_buffer: SocketBuffer::new(),
            // ca_cert_name: None,
            // c_cert_name: None, //TODO: Make &str with lifetime
            // c_key_name: None,
            read_timeout: None,
        }
    }

    /// Return the socket handle.
    #[inline]
    pub fn handle(&self) -> SocketHandle {
        self.meta.handle
    }

    /// Return the socket channel id.
    #[inline]
    pub fn channel_id(&self) -> ChannelId {
        self.meta.channel_id
    }

    /// Return the socket endpoint.
    #[inline]
    pub fn endpoint(&self) -> SocketAddr {
        self.endpoint
    }

    /// Return the connection state, in terms of the TCP state machine.
    #[inline]
    pub fn state(&self) -> &State<CLK> {
        &self.state
    }

    pub fn recycle(&self, ts: &Instant<CLK>) -> bool
    where
        Generic<CLK::T>: TryInto<Milliseconds>,
    {
        if let Some(read_timeout) = self.read_timeout {
            match self.state {
                State::ShutdownForWrite(ref closed_time) => ts
                    .checked_duration_since(closed_time)
                    .and_then(|dur| dur.try_into().ok())
                    .map(|dur: Milliseconds| dur >= read_timeout)
                    .unwrap_or(false),
                _ => false,
            }
        } else {
            false
        }
    }

    /// Close the connection.
    pub fn close(&mut self) {
        self.set_state(State::Closed);
    }

    /// Return whether the socket is passively listening for incoming connections.
    ///
    /// In terms of the TCP state machine, the socket must be in the `LISTEN` state.
    #[inline]
    pub fn is_listening(&self) -> bool {
        match self.state {
            State::Listen => true,
            _ => false,
        }
    }

    /// Return whether the socket is open.
    ///
    /// This function returns true if the socket will process incoming or dispatch outgoing
    /// packets. Note that this does not mean that it is possible to send or receive data through
    /// the socket; for that, use [can_send](#method.can_send) or [can_recv](#method.can_recv).
    ///
    /// In terms of the TCP state machine, the socket must not be in the `CLOSED`
    /// or `TIME-WAIT` states.
    #[inline]
    pub fn is_open(&self) -> bool {
        match self.state {
            State::Closed => false,
            State::TimeWait => false,
            _ => true,
        }
    }

    /// Return whether a connection is active.
    ///
    /// This function returns true if the socket is actively exchanging packets with
    /// a remote endpoint. Note that this does not mean that it is possible to send or receive
    /// data through the socket; for that, use [can_send](#method.can_send) or
    /// [can_recv](#method.can_recv).
    ///
    /// If a connection is established, [abort](#method.close) will send a reset to
    /// the remote endpoint.
    ///
    /// In terms of the TCP state machine, the socket must be in the `CLOSED`, `TIME-WAIT`,
    /// or `LISTEN` state.
    #[inline]
    pub fn is_active(&self) -> bool {
        match self.state {
            State::Closed => false,
            State::TimeWait => false,
            State::Listen => false,
            _ => true,
        }
    }

    /// Return whether the transmit half of the full-duplex connection is open.
    ///
    /// This function returns true if it's possible to send data and have it arrive
    /// to the remote endpoint. However, it does not make any guarantees about the state
    /// of the transmit buffer, and even if it returns true, [send](#method.send) may
    /// not be able to enqueue any octets.
    ///
    /// In terms of the TCP state machine, the socket must be in the `ESTABLISHED` or
    /// `CLOSE-WAIT` state.
    #[inline]
    pub fn may_send(&self) -> bool {
        match self.state {
            State::Established => true,
            // In CLOSE-WAIT, the remote endpoint has closed our receive half of the connection
            // but we still can transmit indefinitely.
            State::CloseWait => true,
            _ => false,
        }
    }

    /// Return whether the receive half of the full-duplex connection is open.
    ///
    /// This function returns true if it's possible to receive data from the remote endpoint.
    /// It will return true while there is data in the receive buffer, and if there isn't,
    /// as long as the remote endpoint has not closed the connection.
    ///
    /// In terms of the TCP state machine, the socket must be in the `ESTABLISHED`,
    /// `FIN-WAIT-1`, or `FIN-WAIT-2` state, or have data in the receive buffer instead.
    #[inline]
    pub fn may_recv(&self) -> bool {
        match self.state {
            State::Established => true,
            // If we have something in the receive buffer, we can receive that.
            _ if !self.rx_buffer.is_empty() => true,
            _ => false,
        }
    }

    /// Check whether the receive half of the full-duplex connection buffer is open
    /// (see [may_recv](#method.may_recv), and the receive buffer is not full.
    #[inline]
    pub fn can_recv(&self) -> bool {
        if !self.may_recv() {
            return false;
        }

        !self.rx_buffer.is_full()
    }

    fn recv_impl<'b, F, R>(&'b mut self, f: F) -> Result<R>
    where
        F: FnOnce(&'b mut SocketBuffer<L>) -> (usize, R),
    {
        // We may have received some data inside the initial SYN, but until the connection
        // is fully open we must not dequeue any data, as it may be overwritten by e.g.
        // another (stale) SYN. (We do not support TCP Fast Open.)
        if !self.may_recv() {
            return Err(Error::Illegal);
        }

        let (_size, result) = f(&mut self.rx_buffer);
        Ok(result)
    }

    /// Call `f` with the largest contiguous slice of octets in the receive buffer,
    /// and dequeue the amount of elements returned by `f`.
    ///
    /// This function returns `Err(Error::Illegal) if the receive half of
    /// the connection is not open; see [may_recv](#method.may_recv).
    pub fn recv<'b, F, R>(&'b mut self, f: F) -> Result<R>
    where
        F: FnOnce(&'b mut [u8]) -> (usize, R),
    {
        self.recv_impl(|rx_buffer| rx_buffer.dequeue_many_with(f))
    }

    /// Call `f` with a slice of octets in the receive buffer, and dequeue the
    /// amount of elements returned by `f`.
    ///
    /// If the buffer read wraps around, the second argument of `f` will be
    /// `Some()` with the remainder of the buffer, such that the combined slice
    /// of the two arguments, makes up the full buffer.
    ///
    /// This function returns `Err(Error::Illegal) if the receive half of the
    /// connection is not open; see [may_recv](#method.may_recv).
    pub fn recv_wrapping<'b, F>(&'b mut self, f: F) -> Result<usize>
    where
        F: FnOnce(&'b [u8], Option<&'b [u8]>) -> usize,
    {
        self.recv_impl(|rx_buffer| {
            rx_buffer.dequeue_many_with_wrapping(|a, b| {
                let len = f(a, b);
                (len, len)
            })
        })
    }

    /// Dequeue a sequence of received octets, and fill a slice from it.
    ///
    /// This function returns the amount of bytes actually dequeued, which is limited
    /// by the amount of free space in the transmit buffer; down to zero.
    ///
    /// See also [recv](#method.recv).
    pub fn recv_slice(&mut self, data: &mut [u8]) -> Result<usize> {
        self.recv_impl(|rx_buffer| {
            let size = rx_buffer.dequeue_slice(data);
            (size, size)
        })
    }

    /// Peek at a sequence of received octets without removing them from
    /// the receive buffer, and return a pointer to it.
    ///
    /// This function otherwise behaves identically to [recv](#method.recv).
    pub fn peek(&mut self, size: usize) -> Result<&[u8]> {
        // See recv() above.
        if !self.may_recv() {
            return Err(Error::Illegal);
        }

        Ok(self.rx_buffer.get_allocated(0, size))
    }

    /// Peek at a sequence of received octets without removing them from
    /// the receive buffer, and fill a slice from it.
    ///
    /// This function otherwise behaves identically to [recv_slice](#method.recv_slice).
    pub fn peek_slice(&mut self, data: &mut [u8]) -> Result<usize> {
        let buffer = self.peek(data.len())?;
        let data = &mut data[..buffer.len()];
        data.copy_from_slice(buffer);
        Ok(buffer.len())
    }

    pub fn rx_enqueue_slice(&mut self, data: &[u8]) -> usize {
        self.rx_buffer.enqueue_slice(data)
    }

    /// Return the amount of octets queued in the receive buffer.
    ///
    /// Note that the Berkeley sockets interface does not have an equivalent of this API.
    pub fn recv_queue(&self) -> usize {
        self.rx_buffer.len()
    }

    pub fn set_state(&mut self, state: State<CLK>) {
        self.state = state
    }
}

impl<CLK: Clock, const L: usize> Into<Socket<CLK, L>> for TcpSocket<CLK, L> {
    fn into(self) -> Socket<CLK, L> {
        Socket::Tcp(self)
    }
}
