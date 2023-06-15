#[cfg(feature = "socket-tcp")]
pub mod tcp;
// #[cfg(feature = "socket-udp")]
// pub mod udp;

pub mod dns;

use core::cell::RefCell;
use core::future::poll_fn;
use core::task::{Context, Poll};

use crate::command::edm::types::{IPv4ConnectEvent, IPv6ConnectEvent, Protocol};
use crate::peer_builder::PeerUrlBuilder;

use super::{channel, MTU};

use super::channel::driver::{Driver, LinkState, RxToken, TxToken};
use embassy_sync::waitqueue::WakerRegistration;
use embassy_time::Timer;
use embedded_nal_async::SocketAddr;
use futures::{pin_mut, Future};
use heapless::Vec;
use no_std_net::IpAddr;
use serde::{Deserialize, Serialize};
use ublox_sockets::{
    AnySocket, ChannelId, PeerHandle, Socket, SocketHandle, SocketSet, SocketStorage, TcpState,
};

pub struct StackResources<const SOCK: usize> {
    sockets: [SocketStorage<'static>; SOCK],
}

impl<const SOCK: usize> StackResources<SOCK> {
    pub fn new() -> Self {
        Self {
            sockets: [SocketStorage::EMPTY; SOCK],
        }
    }
}

pub struct UbloxStack {
    pub(crate) socket: RefCell<SocketStack>,
    inner: RefCell<Inner>,
}

struct Inner {
    device: channel::Device<'static, MTU>,
    link_up: bool,
    dns_result: Option<Result<IpAddr, ()>>,
    dns_waker: WakerRegistration,
}

pub(crate) struct SocketStack {
    pub(crate) sockets: SocketSet<'static>,
    pub(crate) waker: WakerRegistration,
    dropped_sockets: heapless::Vec<PeerHandle, 3>,
}

impl UbloxStack {
    pub fn new<const SOCK: usize>(
        device: channel::Device<'static, MTU>,
        resources: &'static mut StackResources<SOCK>,
    ) -> Self {
        let sockets = SocketSet::new(&mut resources.sockets[..]);

        let socket = SocketStack {
            sockets,
            waker: WakerRegistration::new(),
            dropped_sockets: heapless::Vec::new(),
        };

        let inner = Inner {
            device,
            link_up: false,
            dns_result: None,
            dns_waker: WakerRegistration::new(),
        };

        Self {
            socket: RefCell::new(socket),
            inner: RefCell::new(inner),
        }
    }

    #[allow(dead_code)]
    fn with<R>(&self, f: impl FnOnce(&SocketStack, &Inner) -> R) -> R {
        f(&*self.socket.borrow(), &*self.inner.borrow())
    }

    fn with_mut<R>(&self, f: impl FnOnce(&mut SocketStack, &mut Inner) -> R) -> R {
        f(
            &mut *self.socket.borrow_mut(),
            &mut *self.inner.borrow_mut(),
        )
    }

    pub async fn run(&self) -> ! {
        poll_fn(|cx| {
            self.with_mut(|s, i| i.poll(cx, s));
            Poll::<()>::Pending
        })
        .await;
        unreachable!()
    }

    /// Make a query for a given name and return the corresponding IP addresses.
    // #[cfg(feature = "dns")]
    pub async fn dns_query(
        &self,
        name: &str,
        addr_type: embedded_nal_async::AddrType,
    ) -> Result<Vec<IpAddr, 1>, dns::Error> {
        // For A and AAAA queries we try detect whether `name` is just an IP address
        match addr_type {
            embedded_nal_async::AddrType::IPv4 => {
                if let Ok(ip) = name.parse().map(IpAddr::V4) {
                    return Ok([ip].into_iter().collect());
                }
            }
            embedded_nal_async::AddrType::IPv6 => {
                if let Ok(ip) = name.parse().map(IpAddr::V6) {
                    return Ok([ip].into_iter().collect());
                }
            }
            _ => {}
        }

        poll_fn(|cx| {
            self.with_mut(|_s, i| {
                i.dns_result = None;
                Poll::Ready(
                    i.send_packet(cx, SocketTx::Dns(name))
                        .map_err(|_| dns::Error::Failed),
                )
            })
        })
        .await?;

        poll_fn(|cx| {
            self.with_mut(|_s, i| match i.dns_result {
                Some(Ok(ip)) => Poll::Ready(Ok([ip].into_iter().collect())),
                Some(Err(_)) => Poll::Ready(Err(dns::Error::Failed)),
                None => {
                    i.dns_waker.register(cx.waker());
                    Poll::Pending
                }
            })
        })
        .await
    }
}

impl Inner {
    fn poll(&mut self, cx: &mut Context<'_>, s: &mut SocketStack) {
        s.waker.register(cx.waker());

        self.socket_rx(cx, s);
        self.socket_tx(cx, s);

        // Handle delayed close-by-drop here
        while let Some(dropped_peer_handle) = s.dropped_sockets.pop() {
            defmt::warn!("Handling dropped socket {}", dropped_peer_handle);
            self.send_packet(cx, SocketTx::Disconnect(dropped_peer_handle))
                .ok();
        }
        // Update link up
        let old_link_up = self.link_up;
        self.link_up = self.device.link_state(cx) == LinkState::Up;

        // Print when changed
        if old_link_up != self.link_up {
            defmt::info!("link_up = {:?}", self.link_up);
        }
    }

    fn socket_rx(&mut self, cx: &mut Context<'_>, s: &mut SocketStack) {
        while let Some((rx_token, _)) = self.device.receive(cx) {
            if let Err(e) = rx_token.consume(|a| {
                match postcard::from_bytes::<SocketRx>(a)? {
                    SocketRx::Data(packet) => {
                        for (_handle, socket) in s.sockets.iter_mut() {
                            match socket {
                                #[cfg(feature = "socket-udp")]
                                Socket::Udp(udp)
                                    if udp.edm_channel == Some(packet.edm_channel)
                                        && udp.may_recv() =>
                                {
                                    udp.rx_enqueue_slice(&packet.payload);
                                    break;
                                }
                                #[cfg(feature = "socket-tcp")]
                                Socket::Tcp(tcp)
                                    if tcp.edm_channel == Some(packet.edm_channel)
                                        && tcp.may_recv() =>
                                {
                                    tcp.rx_enqueue_slice(&packet.payload);
                                    break;
                                }
                                _ => {}
                            }
                        }
                    }
                    SocketRx::Ipv4Connect(ev) => {
                        let endpoint = SocketAddr::new(ev.remote_ip.into(), ev.remote_port);
                        Self::connect_event(ev.channel_id, ev.protocol, endpoint, s);
                    }
                    SocketRx::Ipv6Connect(ev) => {
                        let endpoint = SocketAddr::new(ev.remote_ip.into(), ev.remote_port);
                        Self::connect_event(ev.channel_id, ev.protocol, endpoint, s);
                    }
                    SocketRx::Disconnect(Disconnect::EdmChannel(channel_id)) => {
                        for (_handle, socket) in s.sockets.iter_mut() {
                            match socket {
                                #[cfg(feature = "socket-udp")]
                                Socket::Udp(udp) if udp.edm_channel == Some(channel_id) => {
                                    udp.edm_channel = None;
                                    break;
                                }
                                #[cfg(feature = "socket-tcp")]
                                Socket::Tcp(tcp) if tcp.edm_channel == Some(channel_id) => {
                                    tcp.edm_channel = None;
                                    break;
                                }
                                _ => {}
                            }
                        }
                    }
                    SocketRx::Disconnect(Disconnect::Peer(peer_handle)) => {
                        for (_handle, socket) in s.sockets.iter_mut() {
                            match socket {
                                #[cfg(feature = "socket-udp")]
                                Socket::Udp(udp) if udp.peer_handle == Some(peer_handle) => {
                                    tcp.peer_handle = None;
                                    udp.set_state(UdpState::TimeWait);
                                    break;
                                }
                                #[cfg(feature = "socket-tcp")]
                                Socket::Tcp(tcp) if tcp.peer_handle == Some(peer_handle) => {
                                    tcp.peer_handle = None;
                                    tcp.set_state(TcpState::TimeWait);
                                    break;
                                }
                                _ => {}
                            }
                        }
                    }
                    SocketRx::PeerHandle(socket_handle, peer_handle) => {
                        for (handle, socket) in s.sockets.iter_mut() {
                            if handle == socket_handle {
                                match socket {
                                    #[cfg(feature = "socket-udp")]
                                    Socket::Udp(udp) => {
                                        udp.peer_handle = Some(peer_handle);
                                    }
                                    #[cfg(feature = "socket-tcp")]
                                    Socket::Tcp(tcp) => {
                                        tcp.peer_handle = Some(peer_handle);
                                    }
                                    _ => {}
                                }
                                break;
                            }
                        }
                    }
                    SocketRx::Ping(res) => {
                        self.dns_result = Some(res);
                        self.dns_waker.wake();
                    }
                }
                Ok::<_, postcard::Error>(())
            }) {
                defmt::error!("Socket RX failed {:?}", e)
            };
        }
    }

    fn socket_tx(&mut self, cx: &mut Context<'_>, s: &mut SocketStack) {
        for (handle, socket) in s.sockets.iter_mut() {
            // if !socket.egress_permitted(self.inner.now, |ip_addr| self.inner.has_neighbor(&ip_addr))
            // {
            //     continue;
            // }

            let result = match socket {
                #[cfg(feature = "socket-udp")]
                Socket::Udp(udp) => todo!(),
                #[cfg(feature = "socket-tcp")]
                Socket::Tcp(tcp) => {
                    let res = match tcp.poll() {
                        Ok(TcpState::Closed) => {
                            if let Some(addr) = tcp.remote_endpoint() {
                                let url = PeerUrlBuilder::new()
                                    .address(&addr)
                                    .set_local_port(tcp.local_port)
                                    .tcp::<128>()
                                    .unwrap();

                                let pkt = SocketTx::Connect(Connect {
                                    socket_handle: handle,
                                    url: &url,
                                });

                                self.send_packet(cx, pkt).and_then(|r| {
                                    tcp.set_state(TcpState::SynSent);
                                    Ok(r)
                                })
                            } else {
                                Ok(())
                            }
                        }
                        // We transmit data in all states where we may have data in the buffer,
                        // or the transmit half of the connection is still open.
                        Ok(TcpState::Established)
                        | Ok(TcpState::CloseWait)
                        | Ok(TcpState::LastAck) => {
                            if let Some(edm_channel) = tcp.edm_channel {
                                tcp.tx_dequeue(|payload| {
                                    if payload.len() > 0 {
                                        let pkt = SocketTx::Data(DataPacket {
                                            edm_channel,
                                            payload,
                                        });

                                        (payload.len(), self.send_packet(cx, pkt))
                                    } else {
                                        (0, Ok(()))
                                    }
                                })
                            } else {
                                Ok(())
                            }
                        }
                        Ok(TcpState::FinWait1) => {
                            let pkt = SocketTx::Disconnect(tcp.peer_handle.unwrap());
                            self.send_packet(cx, pkt)
                        }
                        Ok(TcpState::Listen) => todo!(),
                        Ok(TcpState::SynReceived) => todo!(),
                        Err(_) => Err(()),
                        _ => Ok(()),
                    };

                    if let Some(poll_at) = tcp.poll_at() {
                        let t = Timer::at(poll_at);
                        pin_mut!(t);
                        if t.poll(cx).is_ready() {
                            cx.waker().wake_by_ref();
                        }
                    }

                    res
                }
                _ => Ok(()),
            };

            match result {
                Err(_) => {
                    break;
                } // Device buffer full.
                Ok(()) => {}
            }
        }
    }

    pub(crate) fn send_packet(&mut self, cx: &mut Context, pkt: SocketTx) -> Result<(), ()> {
        let Some(tx_token) = self.device.transmit(cx) else {
            return Err(());
        };

        let len = postcard::experimental::serialized_size(&pkt).map_err(drop)?;
        tx_token.consume(len, |tx_buf| {
            postcard::to_slice(&pkt, tx_buf).map(drop).map_err(drop)
        })?;

        Ok(())
    }

    fn connect_event(
        channel_id: ChannelId,
        protocol: Protocol,
        endpoint: SocketAddr,
        s: &mut SocketStack,
    ) {
        for (_handle, socket) in s.sockets.iter_mut() {
            match protocol {
                #[cfg(feature = "socket-tcp")]
                Protocol::TCP => match ublox_sockets::tcp::Socket::downcast_mut(socket) {
                    Some(tcp) if tcp.remote_endpoint == Some(endpoint) => {
                        tcp.edm_channel = Some(channel_id);
                        tcp.set_state(TcpState::Established);
                        break;
                    }
                    _ => {}
                },
                #[cfg(feature = "socket-udp")]
                Protocol::UDP => match ublox_sockets::udp::Socket::downcast_mut(socket) {
                    Some(udp) if udp.remote_endpoint == Some(endpoint) => {
                        udp.edm_channel = Some(channel_id);
                        udp.set_state(ublox_sockets::UdpState::Established);
                        break;
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
pub enum SocketTx<'a> {
    #[serde(borrow)]
    Data(DataPacket<'a>),
    #[serde(borrow)]
    Connect(Connect<'a>),
    Disconnect(PeerHandle),
    Dns(&'a str),
}

impl<'a> defmt::Format for SocketTx<'a> {
    fn format(&self, fmt: defmt::Formatter) {
        match self {
            SocketTx::Data(_) => defmt::write!(fmt, "SocketTx::Data"),
            SocketTx::Connect(_) => defmt::write!(fmt, "SocketTx::Connect"),
            SocketTx::Disconnect(_) => defmt::write!(fmt, "SocketTx::Disconnect"),
            SocketTx::Dns(_) => defmt::write!(fmt, "SocketTx::Dns"),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Connect<'a> {
    pub url: &'a str,
    pub socket_handle: SocketHandle,
}

#[derive(Serialize, Deserialize)]
pub enum SocketRx<'a> {
    #[serde(borrow)]
    Data(DataPacket<'a>),
    PeerHandle(SocketHandle, PeerHandle),
    Ipv4Connect(IPv4ConnectEvent),
    Ipv6Connect(IPv6ConnectEvent),
    Disconnect(Disconnect),
    Ping(Result<IpAddr, ()>),
}

#[derive(Serialize, Deserialize)]
pub enum Disconnect {
    EdmChannel(ChannelId),
    Peer(PeerHandle),
}

impl<'a> defmt::Format for SocketRx<'a> {
    fn format(&self, fmt: defmt::Formatter) {
        match self {
            SocketRx::Data(_) => defmt::write!(fmt, "SocketRx::Data"),
            SocketRx::PeerHandle(_, _) => defmt::write!(fmt, "SocketRx::PeerHandle"),
            SocketRx::Ipv4Connect(_) => defmt::write!(fmt, "SocketRx::Ipv4Connect"),
            SocketRx::Ipv6Connect(_) => defmt::write!(fmt, "SocketRx::Ipv6Connect"),
            SocketRx::Disconnect(_) => defmt::write!(fmt, "SocketRx::Disconnect"),
            SocketRx::Ping(_) => defmt::write!(fmt, "SocketRx::Ping"),
        }
    }
}

impl From<IPv4ConnectEvent> for SocketRx<'_> {
    fn from(value: IPv4ConnectEvent) -> Self {
        Self::Ipv4Connect(value)
    }
}

impl From<IPv6ConnectEvent> for SocketRx<'_> {
    fn from(value: IPv6ConnectEvent) -> Self {
        Self::Ipv6Connect(value)
    }
}

impl From<ChannelId> for SocketRx<'_> {
    fn from(value: ChannelId) -> Self {
        Self::Disconnect(Disconnect::EdmChannel(value))
    }
}

impl<'a> From<DataPacket<'a>> for SocketRx<'a> {
    fn from(value: DataPacket<'a>) -> Self {
        Self::Data(value)
    }
}

#[derive(Serialize, Deserialize)]
pub struct DataPacket<'a> {
    pub edm_channel: ChannelId,
    pub payload: &'a [u8],
}
