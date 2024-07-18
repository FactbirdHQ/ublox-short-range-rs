use core::cell::RefCell;
use core::future::poll_fn;
use core::mem;
use core::task::Poll;

use embassy_time::Duration;
use embedded_nal_async::SocketAddr;
use ublox_sockets::{tcp, SocketHandle, TcpState};

use super::{SocketStack, UbloxStack};

/// Error returned by TcpSocket read/write functions.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    /// The connection was reset.
    ///
    /// This can happen on receiving a RST packet, or on timeout.
    ConnectionReset,
}

/// Error returned by [`TcpSocket::connect`].
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ConnectError {
    /// The socket is already connected or listening.
    InvalidState,
    /// The remote host rejected the connection with a RST packet.
    ConnectionReset,
    /// Connect timed out.
    TimedOut,
    /// No route to host.
    NoRoute,
}

/// Error returned by [`TcpSocket::accept`].
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum AcceptError {
    /// The socket is already connected or listening.
    InvalidState,
    /// Invalid listen port
    InvalidPort,
    /// The remote host rejected the connection with a RST packet.
    ConnectionReset,
}

/// A TCP socket.
pub struct TcpSocket<'a> {
    pub(crate) io: TcpIo<'a>,
}

/// The reader half of a TCP socket.
pub struct TcpReader<'a> {
    pub(crate) io: TcpIo<'a>,
}

/// The writer half of a TCP socket.
pub struct TcpWriter<'a> {
    pub(crate) io: TcpIo<'a>,
}

impl<'a> TcpReader<'a> {
    /// Read data from the socket.
    ///
    /// Returns how many bytes were read, or an error. If no data is available, it waits
    /// until there is at least one byte available.
    pub async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        self.io.read(buf).await
    }

    /// Call `f` with the largest contiguous slice of octets in the receive buffer,
    /// and dequeue the amount of elements returned by `f`.
    ///
    /// If no data is available, it waits until there is at least one byte available.
    pub async fn read_with<F, R>(&mut self, f: F) -> Result<R, Error>
    where
        F: FnOnce(&mut [u8]) -> (usize, R),
    {
        self.io.read_with(f).await
    }

    /// Return the maximum number of bytes inside the transmit buffer.
    pub fn recv_capacity(&self) -> usize {
        self.io.recv_capacity()
    }
}

impl<'a> TcpWriter<'a> {
    /// Write data to the socket.
    ///
    /// Returns how many bytes were written, or an error. If the socket is not ready to
    /// accept data, it waits until it is.
    pub async fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        self.io.write(buf).await
    }

    /// Flushes the written data to the socket.
    ///
    /// This waits until all data has been sent, and ACKed by the remote host. For a connection
    /// closed with [`abort()`](TcpSocket::abort) it will wait for the TCP RST packet to be sent.
    pub async fn flush(&mut self) -> Result<(), Error> {
        self.io.flush().await
    }

    /// Call `f` with the largest contiguous slice of octets in the transmit buffer,
    /// and enqueue the amount of elements returned by `f`.
    ///
    /// If the socket is not ready to accept data, it waits until it is.
    pub async fn write_with<F, R>(&mut self, f: F) -> Result<R, Error>
    where
        F: FnOnce(&mut [u8]) -> (usize, R),
    {
        self.io.write_with(f).await
    }

    /// Return the maximum number of bytes inside the transmit buffer.
    pub fn send_capacity(&self) -> usize {
        self.io.send_capacity()
    }
}

impl<'a> TcpSocket<'a> {
    /// Create a new TCP socket on the given stack, with the given buffers.
    pub fn new<const INGRESS_BUF_SIZE: usize, const URC_CAPACITY: usize>(
        stack: &'a UbloxStack<INGRESS_BUF_SIZE, URC_CAPACITY>,
        rx_buffer: &'a mut [u8],
        tx_buffer: &'a mut [u8],
    ) -> Self {
        let s = &mut *stack.socket.borrow_mut();
        let rx_buffer: &'static mut [u8] = unsafe { mem::transmute(rx_buffer) };
        let tx_buffer: &'static mut [u8] = unsafe { mem::transmute(tx_buffer) };
        let handle = s.sockets.add(tcp::Socket::new(
            tcp::SocketBuffer::new(rx_buffer),
            tcp::SocketBuffer::new(tx_buffer),
        ));

        Self {
            io: TcpIo {
                stack: &stack.socket,
                handle,
            },
        }
    }

    /// Return the maximum number of bytes inside the recv buffer.
    pub fn recv_capacity(&self) -> usize {
        self.io.recv_capacity()
    }

    /// Return the maximum number of bytes inside the transmit buffer.
    pub fn send_capacity(&self) -> usize {
        self.io.send_capacity()
    }

    /// Call `f` with the largest contiguous slice of octets in the transmit buffer,
    /// and enqueue the amount of elements returned by `f`.
    ///
    /// If the socket is not ready to accept data, it waits until it is.
    pub async fn write_with<F, R>(&mut self, f: F) -> Result<R, Error>
    where
        F: FnOnce(&mut [u8]) -> (usize, R),
    {
        self.io.write_with(f).await
    }

    /// Call `f` with the largest contiguous slice of octets in the receive buffer,
    /// and dequeue the amount of elements returned by `f`.
    ///
    /// If no data is available, it waits until there is at least one byte available.
    pub async fn read_with<F, R>(&mut self, f: F) -> Result<R, Error>
    where
        F: FnOnce(&mut [u8]) -> (usize, R),
    {
        self.io.read_with(f).await
    }

    /// Split the socket into reader and a writer halves.
    pub fn split(&mut self) -> (TcpReader<'_>, TcpWriter<'_>) {
        (TcpReader { io: self.io }, TcpWriter { io: self.io })
    }

    /// Connect to a remote host.
    pub async fn connect<T>(&mut self, remote_endpoint: T) -> Result<(), ConnectError>
    where
        T: Into<SocketAddr>,
    {
        match { self.io.with_mut(|s| s.connect(remote_endpoint, None)) } {
            Ok(()) => {}
            Err(_) => return Err(ConnectError::InvalidState),
            // Err(tcp::ConnectError::Unaddressable) => return Err(ConnectError::NoRoute),
        }

        poll_fn(|cx| {
            self.io.with_mut(|s| match s.state() {
                tcp::State::TimeWait => Poll::Ready(Err(ConnectError::ConnectionReset)),
                tcp::State::Listen => unreachable!(),
                tcp::State::Closed | tcp::State::SynSent | tcp::State::SynReceived => {
                    s.register_send_waker(cx.waker());
                    Poll::Pending
                }
                _ => Poll::Ready(Ok(())),
            })
        })
        .await
    }

    // /// Accept a connection from a remote host.
    // ///
    // /// This function puts the socket in listening mode, and waits until a connection is received.
    // pub async fn accept<T>(&mut self, local_endpoint: T) -> Result<(), AcceptError>
    // where
    //     T: Into<IpListenEndpoint>,
    // {
    //     todo!()
    //     // match self.io.with_mut(|s, _| s.listen(local_endpoint)) {
    //     //     Ok(()) => {}
    //     //     Err(tcp::ListenError::InvalidState) => return Err(AcceptError::InvalidState),
    //     //     Err(tcp::ListenError::Unaddressable) => return Err(AcceptError::InvalidPort),
    //     // }

    //     // poll_fn(|cx| {
    //     //     self.io.with_mut(|s, _| match s.state() {
    //     //         tcp::State::Listen | tcp::State::SynSent | tcp::State::SynReceived => {
    //     //             s.register_send_waker(cx.waker());
    //     //             Poll::Pending
    //     //         }
    //     //         _ => Poll::Ready(Ok(())),
    //     //     })
    //     // })
    //     // .await
    // }

    /// Read data from the socket.
    ///
    /// Returns how many bytes were read, or an error. If no data is available, it waits
    /// until there is at least one byte available.
    pub async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        self.io.read(buf).await
    }

    /// Write data to the socket.
    ///
    /// Returns how many bytes were written, or an error. If the socket is not ready to
    /// accept data, it waits until it is.
    pub async fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        self.io.write(buf).await
    }

    /// Flushes the written data to the socket.
    ///
    /// This waits until all data has been sent, and ACKed by the remote host. For a connection
    /// closed with [`abort()`](TcpSocket::abort) it will wait for the TCP RST packet to be sent.
    pub async fn flush(&mut self) -> Result<(), Error> {
        self.io.flush().await
    }

    /// Set the timeout for the socket.
    ///
    /// If the timeout is set, the socket will be closed if no data is received for the
    /// specified duration.
    pub fn set_timeout(&mut self, _duration: Option<Duration>) {
        todo!()
        // self.io.with_mut(|s| s.set_timeout(duration))
    }

    /// Set the keep-alive interval for the socket.
    ///
    /// If the keep-alive interval is set, the socket will send keep-alive packets after
    /// the specified duration of inactivity.
    ///
    /// If not set, the socket will not send keep-alive packets.
    pub fn set_keep_alive(&mut self, _interval: Option<Duration>) {
        todo!()
        // self.io
        //     .with_mut(|s| s.set_keep_alive(interval.map(duration_to_smoltcp)))
    }

    // /// Set the hop limit field in the IP header of sent packets.
    // pub fn set_hop_limit(&mut self, hop_limit: Option<u8>) {
    //     self.io.with_mut(|s| s.set_hop_limit(hop_limit))
    // }

    /// Get the local endpoint of the socket.
    ///
    /// Returns `None` if the socket is not bound (listening) or not connected.
    pub fn local_endpoint(&self) -> Option<SocketAddr> {
        todo!()
        // self.io.with(|s| s.local_endpoint())
    }

    /// Get the remote endpoint of the socket.
    ///
    /// Returns `None` if the socket is not connected.
    pub fn remote_endpoint(&self) -> Option<SocketAddr> {
        self.io.with(|s| s.remote_endpoint())
    }

    /// Get the state of the socket.
    pub fn state(&self) -> TcpState {
        self.io.with(|s| s.state())
    }

    /// Close the write half of the socket.
    ///
    /// This closes only the write half of the socket. The read half side remains open, the
    /// socket can still receive data.
    ///
    /// Data that has been written to the socket and not yet sent (or not yet ACKed) will still
    /// still sent. The last segment of the pending to send data is sent with the FIN flag set.
    pub fn close(&mut self) {
        self.io.with_mut(|s| s.close())
    }

    /// Forcibly close the socket.
    ///
    /// This instantly closes both the read and write halves of the socket. Any pending data
    /// that has not been sent will be lost.
    ///
    /// Note that the TCP RST packet is not sent immediately - if the `TcpSocket` is dropped too soon
    /// the remote host may not know the connection has been closed.
    /// `abort()` callers should wait for a [`flush()`](TcpSocket::flush) call to complete before
    /// dropping or reusing the socket.
    pub fn abort(&mut self) {
        self.io.with_mut(|s| s.abort())
    }

    /// Get whether the socket is ready to send data, i.e. whether there is space in the send buffer.
    pub fn may_send(&self) -> bool {
        self.io.with(|s| s.may_send())
    }

    /// return whether the receive half of the full-duplex connection is open.
    /// This function returns true if it’s possible to receive data from the remote endpoint.
    /// It will return true while there is data in the receive buffer, and if there isn’t,
    /// as long as the remote endpoint has not closed the connection.
    pub fn may_recv(&self) -> bool {
        self.io.with(|s| s.may_recv())
    }

    /// Get whether the socket is ready to receive data, i.e. whether there is some pending data in the receive buffer.
    pub fn can_recv(&self) -> bool {
        self.io.with(|s| s.can_recv())
    }
}

impl<'a> Drop for TcpSocket<'a> {
    fn drop(&mut self) {
        if matches!(
            self.state(),
            TcpState::Listen | TcpState::Established | TcpState::FinWait1
        ) {
            if let Some(peer_handle) = self.io.with(|s| s.peer_handle) {
                self.io
                    .stack
                    .borrow_mut()
                    .dropped_sockets
                    .push(peer_handle)
                    .ok();
            }
        }
        let mut stack = self.io.stack.borrow_mut();
        stack.sockets.remove(self.io.handle);
        stack.waker.wake();
    }
}

// =======================

#[derive(Copy, Clone)]
pub(crate) struct TcpIo<'a> {
    pub(crate) stack: &'a RefCell<SocketStack>,
    pub(crate) handle: SocketHandle,
}

impl<'d> TcpIo<'d> {
    fn with<R>(&self, f: impl FnOnce(&tcp::Socket) -> R) -> R {
        let s = &*self.stack.borrow();
        let socket = s.sockets.get::<tcp::Socket>(self.handle);
        f(socket)
    }

    fn with_mut<R>(&mut self, f: impl FnOnce(&mut tcp::Socket) -> R) -> R {
        let s = &mut *self.stack.borrow_mut();
        let socket = s.sockets.get_mut::<tcp::Socket>(self.handle);
        let res = f(socket);
        s.waker.wake();
        res
    }

    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        poll_fn(move |cx| {
            // CAUTION: smoltcp semantics around EOF are different to what you'd expect
            // from posix-like IO, so we have to tweak things here.
            self.with_mut(|s| match s.recv_slice(buf) {
                // No data ready
                Ok(0) if buf.is_empty() => {
                    // embedded_io_async::Read's contract is to not block if buf is empty. While
                    // this function is not a direct implementor of the trait method, we still don't
                    // want our future to never resolve.
                    Poll::Ready(Ok(0))
                }
                // No data ready
                Ok(0) => {
                    s.register_recv_waker(cx.waker());
                    Poll::Pending
                }
                // Data ready!
                Ok(n) => Poll::Ready(Ok(n)),
                // EOF
                Err(_) => Poll::Ready(Ok(0)),
                // FIXME:
                // Err(tcp::RecvError::Finished) => Poll::Ready(Ok(0)),
                // Connection reset. TODO: this can also be timeouts etc, investigate.
                // Err(tcp::RecvError::InvalidState) => Poll::Ready(Err(Error::ConnectionReset)),
            })
        })
        .await
    }

    async fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        poll_fn(move |cx| {
            self.with_mut(|s| match s.send_slice(buf) {
                // Not ready to send (no space in the tx buffer)
                Ok(0) => {
                    s.register_send_waker(cx.waker());
                    Poll::Pending
                }
                // Some data sent
                Ok(n) => Poll::Ready(Ok(n)),
                // Connection reset. TODO: this can also be timeouts etc, investigate.
                Err(_) => Poll::Ready(Err(Error::ConnectionReset)),
                // FIXME:
                // Err(tcp::SendError::InvalidState) => Poll::Ready(Err(Error::ConnectionReset)),
            })
        })
        .await
    }

    async fn write_with<F, R>(&mut self, f: F) -> Result<R, Error>
    where
        F: FnOnce(&mut [u8]) -> (usize, R),
    {
        let mut f = Some(f);

        poll_fn(move |cx| {
            self.with_mut(|s| {
                if !s.can_send() {
                    if s.may_send() {
                        // socket buffer is full wait until it has atleast one byte free
                        s.register_send_waker(cx.waker());
                        Poll::Pending
                    } else {
                        // if we can't transmit because the transmit half of the duplex connection is closed then return an error
                        Poll::Ready(Err(Error::ConnectionReset))
                    }
                } else {
                    Poll::Ready(match s.send(f.take().unwrap()) {
                        // Connection reset. TODO: this can also be timeouts etc, investigate.
                        // Err(tcp::SendError::InvalidState) => Err(Error::ConnectionReset),
                        Err(_) => Err(Error::ConnectionReset),
                        Ok(r) => Ok(r),
                    })
                }
            })
        })
        .await
    }

    async fn read_with<F, R>(&mut self, f: F) -> Result<R, Error>
    where
        F: FnOnce(&mut [u8]) -> (usize, R),
    {
        let mut f = Some(f);
        poll_fn(move |cx| {
            self.with_mut(|s| {
                if !s.can_recv() {
                    if s.may_recv() {
                        // socket buffer is empty wait until it has atleast one byte has arrived
                        s.register_recv_waker(cx.waker());
                        Poll::Pending
                    } else {
                        // if we can't receive because the receive half of the duplex connection is closed then return an error
                        Poll::Ready(Err(Error::ConnectionReset))
                    }
                } else {
                    Poll::Ready(match s.recv(f.take().unwrap()) {
                        // Connection reset. TODO: this can also be timeouts etc, investigate.
                        // Err(tcp::RecvError::Finished) | Err(tcp::RecvError::InvalidState) => {
                        //     Err(Error::ConnectionReset)
                        // }
                        Err(_) => Err(Error::ConnectionReset),
                        Ok(r) => Ok(r),
                    })
                }
            })
        })
        .await
    }

    async fn flush(&mut self) -> Result<(), Error> {
        poll_fn(move |cx| {
            self.with_mut(|s| {
                // If there are outstanding send operations, register for wake up and wait
                // smoltcp issues wake-ups when octets are dequeued from the send buffer
                if s.send_queue() > 0 {
                    s.register_send_waker(cx.waker());
                    Poll::Pending
                // No outstanding sends, socket is flushed
                } else {
                    Poll::Ready(Ok(()))
                }
            })
        })
        .await
    }

    fn recv_capacity(&self) -> usize {
        self.with(|s| s.recv_capacity())
    }

    fn send_capacity(&self) -> usize {
        self.with(|s| s.send_capacity())
    }
}

mod embedded_io_impls {
    use super::*;

    impl embedded_io_async::Error for ConnectError {
        fn kind(&self) -> embedded_io_async::ErrorKind {
            match self {
                ConnectError::ConnectionReset => embedded_io_async::ErrorKind::ConnectionReset,
                ConnectError::TimedOut => embedded_io_async::ErrorKind::TimedOut,
                ConnectError::NoRoute => embedded_io_async::ErrorKind::NotConnected,
                ConnectError::InvalidState => embedded_io_async::ErrorKind::Other,
            }
        }
    }

    impl embedded_io_async::Error for Error {
        fn kind(&self) -> embedded_io_async::ErrorKind {
            match self {
                Error::ConnectionReset => embedded_io_async::ErrorKind::ConnectionReset,
            }
        }
    }

    impl<'d> embedded_io_async::ErrorType for TcpSocket<'d> {
        type Error = Error;
    }

    impl<'d> embedded_io_async::Read for TcpSocket<'d> {
        async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
            self.io.read(buf).await
        }
    }

    impl<'d> embedded_io_async::ReadReady for TcpSocket<'d> {
        fn read_ready(&mut self) -> Result<bool, Self::Error> {
            Ok(self.io.with(|s| s.may_recv()))
        }
    }

    impl<'d> embedded_io_async::Write for TcpSocket<'d> {
        async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
            self.io.write(buf).await
        }

        async fn flush(&mut self) -> Result<(), Self::Error> {
            self.io.flush().await
        }
    }

    impl<'d> embedded_io_async::WriteReady for TcpSocket<'d> {
        fn write_ready(&mut self) -> Result<bool, Self::Error> {
            Ok(self.io.with(|s| s.may_send()))
        }
    }

    impl<'d> embedded_io_async::ErrorType for TcpReader<'d> {
        type Error = Error;
    }

    impl<'d> embedded_io_async::Read for TcpReader<'d> {
        async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
            self.io.read(buf).await
        }
    }

    impl<'d> embedded_io_async::ReadReady for TcpReader<'d> {
        fn read_ready(&mut self) -> Result<bool, Self::Error> {
            Ok(self.io.with(|s| s.may_recv()))
        }
    }

    impl<'d> embedded_io_async::ErrorType for TcpWriter<'d> {
        type Error = Error;
    }

    impl<'d> embedded_io_async::Write for TcpWriter<'d> {
        async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
            self.io.write(buf).await
        }

        async fn flush(&mut self) -> Result<(), Self::Error> {
            self.io.flush().await
        }
    }

    impl<'d> embedded_io_async::WriteReady for TcpWriter<'d> {
        fn write_ready(&mut self) -> Result<bool, Self::Error> {
            Ok(self.io.with(|s| s.may_send()))
        }
    }
}

/// TCP client compatible with `embedded-nal-async` traits.
pub mod client {
    use core::cell::{Cell, UnsafeCell};
    use core::mem::MaybeUninit;
    use core::ptr::NonNull;

    use crate::asynch::ublox_stack::dns::DnsSocket;

    use super::*;

    /// TCP client connection pool compatible with `embedded-nal-async` traits.
    ///
    /// The pool is capable of managing up to N concurrent connections with tx and rx buffers according to TX_SZ and RX_SZ.
    pub struct TcpClient<
        'd,
        const INGRESS_BUF_SIZE: usize,
        const URC_CAPACITY: usize,
        const N: usize,
        const TX_SZ: usize = 1024,
        const RX_SZ: usize = 1024,
    > {
        pub(crate) stack: &'d UbloxStack<INGRESS_BUF_SIZE, URC_CAPACITY>,
        pub(crate) state: &'d TcpClientState<N, TX_SZ, RX_SZ>,
    }

    impl<
            'd,
            const INGRESS_BUF_SIZE: usize,
            const URC_CAPACITY: usize,
            const N: usize,
            const TX_SZ: usize,
            const RX_SZ: usize,
        > embedded_nal_async::Dns
        for TcpClient<'d, INGRESS_BUF_SIZE, URC_CAPACITY, N, TX_SZ, RX_SZ>
    {
        type Error = crate::asynch::ublox_stack::dns::Error;

        async fn get_host_by_name(
            &self,
            host: &str,
            addr_type: embedded_nal_async::AddrType,
        ) -> Result<no_std_net::IpAddr, Self::Error> {
            DnsSocket::new(self.stack).query(host, addr_type).await
        }

        async fn get_host_by_address(
            &self,
            _addr: no_std_net::IpAddr,
            _result: &mut [u8],
        ) -> Result<usize, Self::Error> {
            unimplemented!()
        }
    }

    impl<
            'd,
            const INGRESS_BUF_SIZE: usize,
            const URC_CAPACITY: usize,
            const N: usize,
            const TX_SZ: usize,
            const RX_SZ: usize,
        > TcpClient<'d, INGRESS_BUF_SIZE, URC_CAPACITY, N, TX_SZ, RX_SZ>
    {
        /// Create a new `TcpClient`.
        pub fn new(
            stack: &'d UbloxStack<INGRESS_BUF_SIZE, URC_CAPACITY>,
            state: &'d TcpClientState<N, TX_SZ, RX_SZ>,
        ) -> Self {
            Self { stack, state }
        }
    }

    impl<
            'd,
            const INGRESS_BUF_SIZE: usize,
            const URC_CAPACITY: usize,
            const N: usize,
            const TX_SZ: usize,
            const RX_SZ: usize,
        > embedded_nal_async::TcpConnect
        for TcpClient<'d, INGRESS_BUF_SIZE, URC_CAPACITY, N, TX_SZ, RX_SZ>
    {
        type Error = Error;
        type Connection<'m> = TcpConnection<'m, N, TX_SZ, RX_SZ> where Self: 'm;

        async fn connect<'a>(
            &'a self,
            remote: SocketAddr,
        ) -> Result<Self::Connection<'a>, Self::Error> {
            let remote_endpoint = (remote.ip(), remote.port());
            let mut socket = TcpConnection::new(self.stack, self.state)?;
            socket
                .socket
                .connect(remote_endpoint)
                .await
                .map_err(|_| Error::ConnectionReset)?;
            Ok(socket)
        }
    }

    /// Opened TCP connection in a [`TcpClient`].
    pub struct TcpConnection<'d, const N: usize, const TX_SZ: usize, const RX_SZ: usize> {
        socket: TcpSocket<'d>,
        state: &'d TcpClientState<N, TX_SZ, RX_SZ>,
        bufs: NonNull<([u8; TX_SZ], [u8; RX_SZ])>,
    }

    impl<'d, const N: usize, const TX_SZ: usize, const RX_SZ: usize>
        TcpConnection<'d, N, TX_SZ, RX_SZ>
    {
        fn new<const INGRESS_BUF_SIZE: usize, const URC_CAPACITY: usize>(
            stack: &'d UbloxStack<INGRESS_BUF_SIZE, URC_CAPACITY>,
            state: &'d TcpClientState<N, TX_SZ, RX_SZ>,
        ) -> Result<Self, Error> {
            let mut bufs = state.pool.alloc().ok_or(Error::ConnectionReset)?;
            Ok(Self {
                socket: unsafe {
                    TcpSocket::new(stack, &mut bufs.as_mut().1, &mut bufs.as_mut().0)
                },
                state,
                bufs,
            })
        }
    }

    impl<'d, const N: usize, const TX_SZ: usize, const RX_SZ: usize> Drop
        for TcpConnection<'d, N, TX_SZ, RX_SZ>
    {
        fn drop(&mut self) {
            unsafe {
                self.socket.close();
                self.state.pool.free(self.bufs);
            }
        }
    }

    impl<'d, const N: usize, const TX_SZ: usize, const RX_SZ: usize> embedded_io_async::ErrorType
        for TcpConnection<'d, N, TX_SZ, RX_SZ>
    {
        type Error = Error;
    }

    impl<'d, const N: usize, const TX_SZ: usize, const RX_SZ: usize> embedded_io_async::Read
        for TcpConnection<'d, N, TX_SZ, RX_SZ>
    {
        async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
            self.socket.read(buf).await
        }
    }

    impl<'d, const N: usize, const TX_SZ: usize, const RX_SZ: usize> embedded_io_async::Write
        for TcpConnection<'d, N, TX_SZ, RX_SZ>
    {
        async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
            self.socket.write(buf).await
        }

        async fn flush(&mut self) -> Result<(), Self::Error> {
            self.socket.flush().await
        }
    }

    /// State for TcpClient
    pub struct TcpClientState<const N: usize, const TX_SZ: usize, const RX_SZ: usize> {
        pub(crate) pool: Pool<([u8; TX_SZ], [u8; RX_SZ]), N>,
    }

    impl<const N: usize, const TX_SZ: usize, const RX_SZ: usize> TcpClientState<N, TX_SZ, RX_SZ> {
        /// Create a new `TcpClientState`.
        pub const fn new() -> Self {
            Self { pool: Pool::new() }
        }
    }

    pub(crate) struct Pool<T, const N: usize> {
        used: [Cell<bool>; N],
        data: [UnsafeCell<MaybeUninit<T>>; N],
    }

    impl<T, const N: usize> Pool<T, N> {
        const VALUE: Cell<bool> = Cell::new(false);
        const UNINIT: UnsafeCell<MaybeUninit<T>> = UnsafeCell::new(MaybeUninit::uninit());

        pub(crate) const fn new() -> Self {
            Self {
                used: [Self::VALUE; N],
                data: [Self::UNINIT; N],
            }
        }
    }

    impl<T, const N: usize> Pool<T, N> {
        pub(crate) fn alloc(&self) -> Option<NonNull<T>> {
            for n in 0..N {
                // this can't race because Pool is not Sync.
                if !self.used[n].get() {
                    self.used[n].set(true);
                    let p = self.data[n].get() as *mut T;
                    return Some(unsafe { NonNull::new_unchecked(p) });
                }
            }
            None
        }

        /// safety: p must be a pointer obtained from self.alloc that hasn't been freed yet.
        pub(crate) unsafe fn free(&self, p: NonNull<T>) {
            let origin = self.data.as_ptr() as *mut T;
            let n = p.as_ptr().offset_from(origin);
            assert!(n >= 0);
            assert!((n as usize) < N);
            self.used[n as usize].set(false);
        }
    }
}
