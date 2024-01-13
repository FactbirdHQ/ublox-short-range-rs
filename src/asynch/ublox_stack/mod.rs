#[cfg(feature = "socket-tcp")]
pub mod tcp;
#[cfg(feature = "socket-tcp")]
pub mod tls;
#[cfg(feature = "socket-udp")]
pub mod udp;

pub mod dns;

use core::cell::RefCell;
use core::future::poll_fn;
use core::ops::{DerefMut, Rem};
use core::task::Poll;

use crate::asynch::state::Device;
use crate::command::data_mode::responses::ConnectPeerResponse;
use crate::command::data_mode::urc::PeerDisconnected;
use crate::command::data_mode::{ClosePeerConnection, ConnectPeer};
use crate::command::edm::types::{DataEvent, Protocol, DATA_PACKAGE_SIZE};
use crate::command::edm::urc::EdmEvent;
use crate::command::edm::EdmDataCommand;
use crate::command::ping::types::PingError;
use crate::command::ping::urc::{PingErrorResponse, PingResponse};
use crate::command::ping::Ping;
use crate::command::security::types::SecurityDataType;
use crate::command::security::{PrepareSecurityDataImport, SendSecurityDataImport};
use crate::command::Urc;
use crate::peer_builder::{PeerUrlBuilder, SecurityCredentials};

use self::dns::{DnsSocket, DnsState, DnsTable, MAX_DOMAIN_NAME_LENGTH};

use super::state::{self, LinkState};
use super::AtHandle;

use atat::asynch::AtatClient;
use embassy_futures::select::{select4, Either4};
use embassy_sync::waitqueue::WakerRegistration;
use embassy_time::{Duration, Ticker};
use embedded_nal_async::SocketAddr;
use futures::pin_mut;
use no_std_net::IpAddr;
use portable_atomic::{AtomicBool, AtomicU8, Ordering};
use ublox_sockets::{
    AnySocket, ChannelId, PeerHandle, Socket, SocketHandle, SocketSet, SocketStorage,
};

#[cfg(feature = "socket-tcp")]
use ublox_sockets::TcpState;

#[cfg(feature = "socket-udp")]
use ublox_sockets::UdpState;

pub struct StackResources<const SOCK: usize> {
    sockets: [SocketStorage<'static>; SOCK],
}

impl<const SOCK: usize> Default for StackResources<SOCK> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const SOCK: usize> StackResources<SOCK> {
    pub fn new() -> Self {
        Self {
            sockets: [SocketStorage::EMPTY; SOCK],
        }
    }
}

pub struct UbloxStack<AT: AtatClient + 'static, const URC_CAPACITY: usize> {
    socket: RefCell<SocketStack>,
    device: RefCell<state::Device<'static, AT, URC_CAPACITY>>,
    last_tx_socket: AtomicU8,
    should_tx: AtomicBool,
    link_up: AtomicBool,
}

struct SocketStack {
    sockets: SocketSet<'static>,
    waker: WakerRegistration,
    dns_table: DnsTable,
    dropped_sockets: heapless::Vec<PeerHandle, 3>,
    credential_map: heapless::FnvIndexMap<SocketHandle, SecurityCredentials, 3>,
}

impl<AT: AtatClient + 'static, const URC_CAPACITY: usize> UbloxStack<AT, URC_CAPACITY> {
    pub fn new<const SOCK: usize>(
        device: state::Device<'static, AT, URC_CAPACITY>,
        resources: &'static mut StackResources<SOCK>,
    ) -> Self {
        let sockets = SocketSet::new(&mut resources.sockets[..]);

        let socket = SocketStack {
            sockets,
            dns_table: DnsTable::new(),
            waker: WakerRegistration::new(),
            dropped_sockets: heapless::Vec::new(),
            credential_map: heapless::IndexMap::new(),
        };

        Self {
            socket: RefCell::new(socket),
            device: RefCell::new(device),
            last_tx_socket: AtomicU8::new(0),
            link_up: AtomicBool::new(false),
            should_tx: AtomicBool::new(false),
        }
    }

    pub async fn run(&self) -> ! {
        loop {
            // FIXME: It feels like this can be written smarter/simpler?
            let should_tx = poll_fn(|cx| match self.should_tx.load(Ordering::Relaxed) {
                true => {
                    self.should_tx.store(false, Ordering::Relaxed);
                    Poll::Ready(())
                }
                false => {
                    self.should_tx.store(true, Ordering::Relaxed);
                    self.socket.borrow_mut().waker.register(cx.waker());
                    Poll::<()>::Pending
                }
            });

            let ticker = Ticker::every(Duration::from_millis(100));
            pin_mut!(ticker);

            let mut device = self.device.borrow_mut();
            let Device {
                ref mut urc_subscription,
                ref mut shared,
                ref mut at,
            } = device.deref_mut();

            match select4(
                urc_subscription.next_message_pure(),
                should_tx,
                ticker.next(),
                poll_fn(
                    |cx| match (self.link_up.load(Ordering::Relaxed), shared.link_state(cx)) {
                        (true, LinkState::Down) => Poll::Ready(LinkState::Down),
                        (false, LinkState::Up) => Poll::Ready(LinkState::Up),
                        _ => Poll::Pending,
                    },
                ),
            )
            .await
            {
                Either4::First(event) => {
                    Self::socket_rx(event, &self.socket);
                }
                Either4::Second(_) | Either4::Third(_) => {
                    if let Some(ev) = self.tx_event() {
                        Self::socket_tx(ev, &self.socket, at).await;
                    }
                }
                Either4::Fourth(new_state) => {
                    // Update link up
                    let old_link_up = self.link_up.load(Ordering::Relaxed);
                    let new_link_up = new_state == LinkState::Up;
                    self.link_up.store(new_link_up, Ordering::Relaxed);

                    // Print when changed
                    if old_link_up != new_link_up {
                        info!("link_up = {:?}", new_link_up);
                    }
                }
            }
        }
    }

    // FIXME: This could probably be improved
    pub async fn import_credentials(
        &self,
        root_ca: (&str, &[u8]),
        cert: (&str, &[u8]),
        priv_key: (&str, &[u8]),
    ) -> Result<(), atat::Error> {
        let mut device = self.device.borrow_mut();

        assert!(root_ca.0.len() < 16);
        assert!(cert.0.len() < 16);
        assert!(priv_key.0.len() < 16);

        device
            .at
            .send_edm(PrepareSecurityDataImport {
                data_type: SecurityDataType::TrustedRootCA,
                data_size: root_ca.1.len(),
                internal_name: root_ca.0,
                password: None,
            })
            .await?;

        device
            .at
            .send_edm(SendSecurityDataImport {
                data: atat::serde_bytes::Bytes::new(root_ca.1),
            })
            .await?;

        device
            .at
            .send_edm(PrepareSecurityDataImport {
                data_type: SecurityDataType::ClientCertificate,
                data_size: cert.1.len(),
                internal_name: cert.0,
                password: None,
            })
            .await?;

        device
            .at
            .send_edm(SendSecurityDataImport {
                data: atat::serde_bytes::Bytes::new(cert.1),
            })
            .await?;

        device
            .at
            .send_edm(PrepareSecurityDataImport {
                data_type: SecurityDataType::ClientPrivateKey,
                data_size: priv_key.1.len(),
                internal_name: priv_key.0,
                password: None,
            })
            .await?;

        device
            .at
            .send_edm(SendSecurityDataImport {
                data: atat::serde_bytes::Bytes::new(priv_key.1),
            })
            .await?;

        // FIXME:
        // self.socket.borrow_mut().credential_map.insert(key, value);

        Ok(())
    }

    /// Make a query for a given name and return the corresponding IP addresses.
    // #[cfg(feature = "dns")]
    pub async fn dns_query(
        &self,
        name: &str,
        addr_type: embedded_nal_async::AddrType,
    ) -> Result<IpAddr, dns::Error> {
        DnsSocket::new(self).query(name, addr_type).await
    }

    fn socket_rx(event: EdmEvent, socket: &RefCell<SocketStack>) {
        match event {
            EdmEvent::IPv4ConnectEvent(ev) => {
                let endpoint = SocketAddr::new(ev.remote_ip.into(), ev.remote_port);
                Self::connect_event(ev.channel_id, ev.protocol, endpoint, socket);
            }
            EdmEvent::IPv6ConnectEvent(ev) => {
                let endpoint = SocketAddr::new(ev.remote_ip.into(), ev.remote_port);
                Self::connect_event(ev.channel_id, ev.protocol, endpoint, socket);
            }
            EdmEvent::DisconnectEvent(channel_id) => {
                let mut s = socket.borrow_mut();
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
            EdmEvent::DataEvent(DataEvent { channel_id, data }) => {
                let mut s = socket.borrow_mut();
                for (_handle, socket) in s.sockets.iter_mut() {
                    match socket {
                        #[cfg(feature = "socket-udp")]
                        Socket::Udp(udp)
                            if udp.edm_channel == Some(channel_id) =>
                            // FIXME:
                            // if udp.edm_channel == Some(channel_id) && udp.may_recv() =>
                        {
                            let n = udp.rx_enqueue_slice(&data);
                            if n < data.len() {
                                error!(
                                    "[{}] UDP RX data overflow! Discarding {} bytes",
                                    udp.peer_handle,
                                    data.len() - n
                                );
                            }
                            break;
                        }
                        #[cfg(feature = "socket-tcp")]
                        Socket::Tcp(tcp)
                            if tcp.edm_channel == Some(channel_id) && tcp.may_recv() =>
                        {
                            let n = tcp.rx_enqueue_slice(&data);
                            if n < data.len() {
                                error!(
                                    "[{}] TCP RX data overflow! Discarding {} bytes",
                                    tcp.peer_handle,
                                    data.len() - n
                                );
                            }
                            break;
                        }
                        _ => {}
                    }
                }
            }
            EdmEvent::ATEvent(Urc::PeerDisconnected(PeerDisconnected { handle })) => {
                let mut s = socket.borrow_mut();
                for (_handle, socket) in s.sockets.iter_mut() {
                    match socket {
                        #[cfg(feature = "socket-udp")]
                        Socket::Udp(udp) if udp.peer_handle == Some(handle) => {
                            udp.peer_handle = None;
                            // FIXME:
                            // udp.set_state(UdpState::TimeWait);
                            break;
                        }
                        #[cfg(feature = "socket-tcp")]
                        Socket::Tcp(tcp) if tcp.peer_handle == Some(handle) => {
                            tcp.peer_handle = None;
                            tcp.set_state(TcpState::TimeWait);
                            break;
                        }
                        _ => {}
                    }
                }
            }
            EdmEvent::ATEvent(Urc::PingResponse(PingResponse {
                ip, hostname, rtt, ..
            })) => {
                let mut s = socket.borrow_mut();
                if let Some(query) = s.dns_table.get_mut(&hostname) {
                    match query.state {
                        DnsState::Pending if rtt == -1 => {
                            // According to AT manual, rtt = -1 means the PING has timed out
                            query.state = DnsState::Error(PingError::Timeout);
                            query.waker.wake();
                        }
                        DnsState::Pending => {
                            query.state = DnsState::Resolved(ip);
                            query.waker.wake();
                        }
                        _ => {}
                    }
                }
            }
            EdmEvent::ATEvent(Urc::PingErrorResponse(PingErrorResponse { error })) => {
                let mut s = socket.borrow_mut();
                for query in s.dns_table.table.iter_mut() {
                    match query.state {
                        DnsState::Pending => {
                            query.state = DnsState::Error(error);
                            query.waker.wake();
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    fn tx_event(&self) -> Option<TxEvent> {
        let mut s = self.socket.borrow_mut();
        for query in s.dns_table.table.iter_mut() {
            if let DnsState::New = query.state {
                query.state = DnsState::Pending;
                return Some(TxEvent::Dns {
                    hostname: query.domain_name.clone(),
                });
            }
        }

        // Handle delayed close-by-drop here
        if let Some(dropped_peer_handle) = s.dropped_sockets.pop() {
            warn!("Handling dropped socket {}", dropped_peer_handle);
            return Some(TxEvent::Close {
                peer_handle: dropped_peer_handle,
            });
        }

        // Make sure to give all sockets an even opportunity to TX
        let skip = self
            .last_tx_socket
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| {
                let next = v + 1;
                Some(next.rem(s.sockets.sockets.len() as u8))
            })
            .unwrap();

        let SocketStack {
            sockets,
            dns_table,
            credential_map,
            ..
        } = s.deref_mut();

        for (handle, socket) in sockets.iter_mut().skip(skip as usize) {
            match socket {
                #[cfg(feature = "socket-udp")]
                Socket::Udp(_udp) => todo!(),
                #[cfg(feature = "socket-tcp")]
                Socket::Tcp(tcp) => {
                    tcp.poll();

                    match tcp.state() {
                        TcpState::Closed => {
                            if let Some(addr) = tcp.remote_endpoint() {
                                let mut builder = PeerUrlBuilder::new();

                                if let Some(hostname) = dns_table.reverse_lookup(addr.ip()) {
                                    builder.hostname(hostname).port(addr.port())
                                } else {
                                    builder.address(&addr)
                                };

                                if let Some(creds) = credential_map.remove(&handle) {
                                    builder.creds(creds);
                                }

                                let url =
                                    builder.set_local_port(tcp.local_port).tcp::<128>().unwrap();

                                return Some(TxEvent::Connect {
                                    socket_handle: handle,
                                    url,
                                });
                            }
                        }
                        // We transmit data in all states where we may have data in the buffer,
                        // or the transmit half of the connection is still open.
                        TcpState::Established | TcpState::CloseWait | TcpState::LastAck => {
                            if let Some(edm_channel) = tcp.edm_channel {
                                return tcp.tx_dequeue(|payload| {
                                    let len = core::cmp::min(payload.len(), DATA_PACKAGE_SIZE);
                                    let res = if len != 0 {
                                        Some(TxEvent::Send {
                                            edm_channel,
                                            data: heapless::Vec::from_slice(payload).unwrap(),
                                        })
                                    } else {
                                        None
                                    };

                                    (len, res)
                                });
                            }
                        }
                        TcpState::FinWait1 => {
                            return Some(TxEvent::Close {
                                peer_handle: tcp.peer_handle.unwrap(),
                            });
                        }
                        TcpState::Listen => todo!(),
                        TcpState::SynReceived => todo!(),
                        _ => {}
                    };
                }
                _ => {}
            };
        }

        None
    }

    async fn socket_tx(ev: TxEvent, socket: &RefCell<SocketStack>, at: &mut AtHandle<'_, AT>) {
        match ev {
            TxEvent::Connect { socket_handle, url } => {
                match at.send_edm(ConnectPeer { url: &url }).await {
                    Ok(ConnectPeerResponse { peer_handle }) => {
                        let mut s = socket.borrow_mut();
                        let tcp = s
                            .sockets
                            .get_mut::<ublox_sockets::tcp::Socket>(socket_handle);
                        tcp.peer_handle = Some(peer_handle);
                        tcp.set_state(TcpState::SynSent);
                    }
                    Err(e) => {
                        error!("Failed to connect?! {}", e)
                    }
                }
            }
            TxEvent::Send { edm_channel, data } => {
                warn!("Sending {} bytes on {}", data.len(), edm_channel);
                at.send(EdmDataCommand {
                    channel: edm_channel,
                    data: &data,
                })
                .await
                .ok();
            }
            TxEvent::Close { peer_handle } => {
                at.send_edm(ClosePeerConnection { peer_handle }).await.ok();
            }
            TxEvent::Dns { hostname } => {
                match at
                    .send_edm(Ping {
                        hostname: &hostname,
                        retry_num: 1,
                    })
                    .await
                {
                    Ok(_) => {}
                    Err(_) => {
                        let mut s = socket.borrow_mut();
                        if let Some(query) = s.dns_table.get_mut(&hostname) {
                            match query.state {
                                DnsState::Pending => {
                                    query.state = DnsState::Error(PingError::Other);
                                    query.waker.wake();
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }

    fn connect_event(
        channel_id: ChannelId,
        protocol: Protocol,
        endpoint: SocketAddr,
        socket: &RefCell<SocketStack>,
    ) {
        let mut s = socket.borrow_mut();
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
                    Some(udp) if udp.endpoint == Some(endpoint) => {
                        udp.edm_channel = Some(channel_id);
                        udp.set_state(UdpState::Established);
                        break;
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }
}

// TODO: This extra data clone step can probably be avoided by adding a
// waker/context based API to ATAT.
enum TxEvent {
    Connect {
        socket_handle: SocketHandle,
        url: heapless::String<128>,
    },
    Send {
        edm_channel: ChannelId,
        data: heapless::Vec<u8, DATA_PACKAGE_SIZE>,
    },
    Close {
        peer_handle: PeerHandle,
    },
    Dns {
        hostname: heapless::String<MAX_DOMAIN_NAME_LENGTH>,
    },
}

#[cfg(feature = "defmt")]
impl defmt::Format for TxEvent {
    fn format(&self, fmt: defmt::Formatter) {
        match self {
            TxEvent::Connect { .. } => defmt::write!(fmt, "TxEvent::Connect"),
            TxEvent::Send { .. } => defmt::write!(fmt, "TxEvent::Send"),
            TxEvent::Close { .. } => defmt::write!(fmt, "TxEvent::Close"),
            TxEvent::Dns { .. } => defmt::write!(fmt, "TxEvent::Dns"),
        }
    }
}
