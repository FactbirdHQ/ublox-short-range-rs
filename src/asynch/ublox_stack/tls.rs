use atat::asynch::AtatClient;
use embassy_time::Duration;
use no_std_net::SocketAddr;
use ublox_sockets::TcpState as State;

use crate::peer_builder::SecurityCredentials;

use super::{
    tcp::{ConnectError, Error, TcpIo, TcpReader, TcpSocket, TcpWriter},
    UbloxStack,
};

pub struct TlsSocket<'a> {
    inner: TcpSocket<'a>,
}

impl<'a> TlsSocket<'a> {
    /// Create a new TCP socket on the given stack, with the given buffers.
    pub fn new<AT: AtatClient, const URC_CAPACITY: usize>(
        stack: &'a UbloxStack<AT, URC_CAPACITY>,
        rx_buffer: &'a mut [u8],
        tx_buffer: &'a mut [u8],
        credentials: SecurityCredentials,
    ) -> Self {
        let tcp_socket = TcpSocket::new(stack, rx_buffer, tx_buffer);

        let TcpIo { stack, handle } = tcp_socket.io;

        let s = &mut *stack.borrow_mut();
        info!("Associating credentials {} with {}", credentials, handle);
        s.credential_map.insert(handle, credentials);

        Self { inner: tcp_socket }
    }

    /// Return the maximum number of bytes inside the recv buffer.
    pub fn recv_capacity(&self) -> usize {
        self.inner.recv_capacity()
    }

    /// Return the maximum number of bytes inside the transmit buffer.
    pub fn send_capacity(&self) -> usize {
        self.inner.send_capacity()
    }

    /// Call `f` with the largest contiguous slice of octets in the transmit buffer,
    /// and enqueue the amount of elements returned by `f`.
    ///
    /// If the socket is not ready to accept data, it waits until it is.
    pub async fn write_with<F, R>(&mut self, f: F) -> Result<R, Error>
    where
        F: FnOnce(&mut [u8]) -> (usize, R),
    {
        self.inner.write_with(f).await
    }

    /// Call `f` with the largest contiguous slice of octets in the receive buffer,
    /// and dequeue the amount of elements returned by `f`.
    ///
    /// If no data is available, it waits until there is at least one byte available.
    pub async fn read_with<F, R>(&mut self, f: F) -> Result<R, Error>
    where
        F: FnOnce(&mut [u8]) -> (usize, R),
    {
        self.inner.read_with(f).await
    }

    /// Split the socket into reader and a writer halves.
    pub fn split(&mut self) -> (TcpReader<'_>, TcpWriter<'_>) {
        (
            TcpReader { io: self.inner.io },
            TcpWriter { io: self.inner.io },
        )
    }

    /// Connect to a remote host.
    pub async fn connect<T>(&mut self, remote_endpoint: T) -> Result<(), ConnectError>
    where
        T: Into<SocketAddr>,
    {
        self.inner.connect(remote_endpoint).await
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
        self.inner.read(buf).await
    }

    /// Write data to the socket.
    ///
    /// Returns how many bytes were written, or an error. If the socket is not ready to
    /// accept data, it waits until it is.
    pub async fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        self.inner.write(buf).await
    }

    /// Flushes the written data to the socket.
    ///
    /// This waits until all data has been sent, and ACKed by the remote host. For a connection
    /// closed with [`abort()`](TlsSocket::abort) it will wait for the TCP RST packet to be sent.
    pub async fn flush(&mut self) -> Result<(), Error> {
        self.inner.flush().await
    }

    /// Set the timeout for the socket.
    ///
    /// If the timeout is set, the socket will be closed if no data is received for the
    /// specified duration.
    pub fn set_timeout(&mut self, _duration: Option<Duration>) {
        todo!()
        // self.inner.set_timeout(duration)
    }

    /// Set the keep-alive interval for the socket.
    ///
    /// If the keep-alive interval is set, the socket will send keep-alive packets after
    /// the specified duration of inactivity.
    ///
    /// If not set, the socket will not send keep-alive packets.
    pub fn set_keep_alive(&mut self, interval: Option<Duration>) {
        self.inner.set_keep_alive(interval)
    }

    // /// Set the hop limit field in the IP header of sent packets.
    // pub fn set_hop_limit(&mut self, hop_limit: Option<u8>) {
    //     self.inner.set_hop_limit()
    // }

    /// Get the local endpoint of the socket.
    ///
    /// Returns `None` if the socket is not bound (listening) or not connected.
    pub fn local_endpoint(&self) -> Option<SocketAddr> {
        todo!()
        // self.inner.local_endpoint()
    }

    /// Get the remote endpoint of the socket.
    ///
    /// Returns `None` if the socket is not connected.
    pub fn remote_endpoint(&self) -> Option<SocketAddr> {
        self.inner.remote_endpoint()
    }

    /// Get the state of the socket.
    pub fn state(&self) -> State {
        self.inner.state()
    }

    /// Close the write half of the socket.
    ///
    /// This closes only the write half of the socket. The read half side remains open, the
    /// socket can still receive data.
    ///
    /// Data that has been written to the socket and not yet sent (or not yet ACKed) will still
    /// still sent. The last segment of the pending to send data is sent with the FIN flag set.
    pub fn close(&mut self) {
        self.inner.close()
    }

    /// Forcibly close the socket.
    ///
    /// This instantly closes both the read and write halves of the socket. Any pending data
    /// that has not been sent will be lost.
    ///
    /// Note that the TCP RST packet is not sent immediately - if the `TlsSocket` is dropped too soon
    /// the remote host may not know the connection has been closed.
    /// `abort()` callers should wait for a [`flush()`](TlsSocket::flush) call to complete before
    /// dropping or reusing the socket.
    pub fn abort(&mut self) {
        self.inner.abort()
    }

    /// Get whether the socket is ready to send data, i.e. whether there is space in the send buffer.
    pub fn may_send(&self) -> bool {
        self.inner.may_send()
    }

    /// return whether the recieve half of the full-duplex connection is open.
    /// This function returns true if it’s possible to receive data from the remote endpoint.
    /// It will return true while there is data in the receive buffer, and if there isn’t,
    /// as long as the remote endpoint has not closed the connection.
    pub fn may_recv(&self) -> bool {
        self.inner.may_recv()
    }

    /// Get whether the socket is ready to receive data, i.e. whether there is some pending data in the receive buffer.
    pub fn can_recv(&self) -> bool {
        self.inner.can_recv()
    }
}

impl<'a> Drop for TlsSocket<'a> {
    fn drop(&mut self) {
        let mut stack = self.inner.io.stack.borrow_mut();
        stack.credential_map.remove(&self.inner.io.handle);
    }
}

mod embedded_io_impls {
    use super::*;

    impl<'d> embedded_io_async::ErrorType for TlsSocket<'d> {
        type Error = Error;
    }

    impl<'d> embedded_io_async::Read for TlsSocket<'d> {
        async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
            self.inner.read(buf).await
        }
    }

    impl<'d> embedded_io_async::ReadReady for TlsSocket<'d> {
        fn read_ready(&mut self) -> Result<bool, Self::Error> {
            self.inner.read_ready()
        }
    }

    impl<'d> embedded_io_async::Write for TlsSocket<'d> {
        async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
            self.inner.write(buf).await
        }

        async fn flush(&mut self) -> Result<(), Self::Error> {
            self.inner.flush().await
        }
    }

    impl<'d> embedded_io_async::WriteReady for TlsSocket<'d> {
        fn write_ready(&mut self) -> Result<bool, Self::Error> {
            self.inner.write_ready()
        }
    }
}

/// TLS client compatible with `embedded-nal-async` traits.
pub mod client {
    use core::ptr::NonNull;

    use crate::asynch::ublox_stack::dns::DnsSocket;
    use crate::asynch::ublox_stack::tcp::client::TcpClientState;

    use super::*;

    /// TLS client connection pool compatible with `embedded-nal-async` traits.
    ///
    /// The pool is capable of managing up to N concurrent connections with tx and rx buffers according to TX_SZ and RX_SZ.
    pub struct TlsClient<
        'd,
        AT: AtatClient + 'static,
        const N: usize,
        const URC_CAPACITY: usize,
        const TX_SZ: usize = 1024,
        const RX_SZ: usize = 1024,
    > {
        pub(crate) stack: &'d UbloxStack<AT, URC_CAPACITY>,
        pub(crate) state: &'d TcpClientState<N, TX_SZ, RX_SZ>,
        pub(crate) credentials: SecurityCredentials,
    }

    impl<
            'd,
            AT: AtatClient,
            const N: usize,
            const URC_CAPACITY: usize,
            const TX_SZ: usize,
            const RX_SZ: usize,
        > embedded_nal_async::Dns for TlsClient<'d, AT, N, URC_CAPACITY, TX_SZ, RX_SZ>
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
            AT: AtatClient,
            const N: usize,
            const URC_CAPACITY: usize,
            const TX_SZ: usize,
            const RX_SZ: usize,
        > TlsClient<'d, AT, N, URC_CAPACITY, TX_SZ, RX_SZ>
    {
        /// Create a new `TlsClient`.
        pub fn new(
            stack: &'d UbloxStack<AT, URC_CAPACITY>,
            state: &'d TcpClientState<N, TX_SZ, RX_SZ>,
            credentials: SecurityCredentials,
        ) -> Self {
            Self {
                stack,
                state,
                credentials,
            }
        }
    }

    impl<
            'd,
            AT: AtatClient,
            const N: usize,
            const URC_CAPACITY: usize,
            const TX_SZ: usize,
            const RX_SZ: usize,
        > embedded_nal_async::TcpConnect for TlsClient<'d, AT, N, URC_CAPACITY, TX_SZ, RX_SZ>
    {
        type Error = Error;
        type Connection<'m> = TlsConnection<'m, N, TX_SZ, RX_SZ> where Self: 'm;

        async fn connect<'a>(
            &'a self,
            remote: SocketAddr,
        ) -> Result<Self::Connection<'a>, Self::Error> {
            let remote_endpoint = (remote.ip(), remote.port());
            let mut socket = TlsConnection::new(self.stack, self.state, self.credentials.clone())?;
            socket
                .socket
                .connect(remote_endpoint)
                .await
                .map_err(|_| Error::ConnectionReset)?;
            Ok(socket)
        }
    }

    /// Opened TLS connection in a [`TlsClient`].
    pub struct TlsConnection<'d, const N: usize, const TX_SZ: usize, const RX_SZ: usize> {
        socket: TlsSocket<'d>,
        state: &'d TcpClientState<N, TX_SZ, RX_SZ>,
        bufs: NonNull<([u8; TX_SZ], [u8; RX_SZ])>,
    }

    impl<'d, const N: usize, const TX_SZ: usize, const RX_SZ: usize>
        TlsConnection<'d, N, TX_SZ, RX_SZ>
    {
        fn new<AT: AtatClient, const URC_CAPACITY: usize>(
            stack: &'d UbloxStack<AT, URC_CAPACITY>,
            state: &'d TcpClientState<N, TX_SZ, RX_SZ>,
            credentials: SecurityCredentials,
        ) -> Result<Self, Error> {
            let mut bufs = state.pool.alloc().ok_or(Error::ConnectionReset)?;
            Ok(Self {
                socket: unsafe {
                    TlsSocket::new(
                        stack,
                        &mut bufs.as_mut().1,
                        &mut bufs.as_mut().0,
                        credentials,
                    )
                },
                state,
                bufs,
            })
        }
    }

    impl<'d, const N: usize, const TX_SZ: usize, const RX_SZ: usize> Drop
        for TlsConnection<'d, N, TX_SZ, RX_SZ>
    {
        fn drop(&mut self) {
            unsafe {
                self.socket.close();
                self.state.pool.free(self.bufs);
            }
        }
    }

    impl<'d, const N: usize, const TX_SZ: usize, const RX_SZ: usize> embedded_io_async::ErrorType
        for TlsConnection<'d, N, TX_SZ, RX_SZ>
    {
        type Error = Error;
    }

    impl<'d, const N: usize, const TX_SZ: usize, const RX_SZ: usize> embedded_io_async::Read
        for TlsConnection<'d, N, TX_SZ, RX_SZ>
    {
        async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
            self.socket.read(buf).await
        }
    }

    impl<'d, const N: usize, const TX_SZ: usize, const RX_SZ: usize> embedded_io_async::Write
        for TlsConnection<'d, N, TX_SZ, RX_SZ>
    {
        async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
            self.socket.write(buf).await
        }

        async fn flush(&mut self) -> Result<(), Self::Error> {
            self.socket.flush().await
        }
    }
}
