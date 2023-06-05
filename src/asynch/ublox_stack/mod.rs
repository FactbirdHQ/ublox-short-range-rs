// #[cfg(feature = "socket-tcp")]
// pub mod tcp;
// #[cfg(feature = "socket-udp")]
// pub mod udp;

use core::cell::RefCell;
use core::future::poll_fn;
use core::task::{Context, Poll};

use crate::command::edm::types::{ChannelId, IPv4ConnectEvent, IPv6ConnectEvent};

use super::MTU;

use ch::driver::{Driver, LinkState, RxToken};
use embassy_net_driver_channel as ch;
use embassy_sync::waitqueue::WakerRegistration;
use embassy_time::Instant;
use serde::{Deserialize, Serialize};
use ublox_sockets::{SocketSet, SocketStorage};

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
    device: ch::Device<'static, MTU>,
    link_up: bool,
}

pub(crate) struct SocketStack {
    pub(crate) sockets: SocketSet<'static>,
    pub(crate) waker: WakerRegistration,
}

impl UbloxStack {
    pub fn new<const SOCK: usize>(
        device: ch::Device<'static, MTU>,
        resources: &'static mut StackResources<SOCK>,
    ) -> Self {
        let sockets = SocketSet::new(&mut resources.sockets[..]);

        let socket = SocketStack {
            sockets,
            waker: WakerRegistration::new(),
        };

        let inner = Inner {
            device,
            link_up: false,
        };

        Self {
            socket: RefCell::new(socket),
            inner: RefCell::new(inner),
        }
    }

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
}

impl Inner {
    fn poll(&mut self, cx: &mut Context<'_>, s: &mut SocketStack) {
        s.waker.register(cx.waker());

        let timestamp = Instant::now();
        // let mut smoldev = DriverAdapter {
        //     cx: Some(cx),
        //     inner: &mut self.device,
        // };
        // s.iface.poll(timestamp, &mut smoldev, &mut s.sockets);

        if let Some((rx, _)) = self.device.receive(cx) {
            rx.consume(|a| {
                s.sockets
                    .get_mut::<ublox_sockets::tcp::Socket>(ublox_sockets::SocketHandle(0))
                    .recv_slice(a)
            })
            .ok();
        }

        // Update link up
        let old_link_up = self.link_up;
        self.link_up = self.device.link_state(cx) == LinkState::Up;

        // Print when changed
        if old_link_up != self.link_up {
            defmt::info!("link_up = {:?}", self.link_up);
        }

        // if let Some(poll_at) = s.iface.poll_at(timestamp, &mut s.sockets) {
        //     let t = Timer::at(instant_from_smoltcp(poll_at));
        //     pin_mut!(t);
        //     if t.poll(cx).is_ready() {
        //         cx.waker().wake_by_ref();
        //     }
        // }
    }
}

#[derive(Serialize, Deserialize)]
pub enum SocketEvent<'a> {
    #[serde(borrow)]
    Data(DataPacket<'a>),
    Ipv4Connect(IPv4ConnectEvent),
    Ipv6Connect(IPv6ConnectEvent),
    Disconnect(ChannelId),
}

impl From<IPv4ConnectEvent> for SocketEvent<'_> {
    fn from(value: IPv4ConnectEvent) -> Self {
        Self::Ipv4Connect(value)
    }
}

impl From<IPv6ConnectEvent> for SocketEvent<'_> {
    fn from(value: IPv6ConnectEvent) -> Self {
        Self::Ipv6Connect(value)
    }
}

impl From<ChannelId> for SocketEvent<'_> {
    fn from(value: ChannelId) -> Self {
        Self::Disconnect(value)
    }
}

impl<'a> From<DataPacket<'a>> for SocketEvent<'a> {
    fn from(value: DataPacket<'a>) -> Self {
        Self::Data(value)
    }
}

#[derive(Serialize, Deserialize)]
pub struct DataPacket<'a> {
    pub edm_channel: ChannelId,
    pub payload: &'a [u8],
}
