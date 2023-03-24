#[cfg(feature = "socket-tcp")]
pub mod tcp;
#[cfg(feature = "udp")]
pub mod udp;

use core::cell::RefCell;
use core::future::{poll_fn, Future};
use core::task::{Context, Poll};

use embassy_sync::waitqueue::WakerRegistration;
use embassy_time::Instant;
use futures::pin_mut;
use ublox_sockets::SocketSet;

// pub struct StackResources<const SOCK: usize> {
//     sockets: [SocketStorage<'static>; SOCK],
// }

// impl<const SOCK: usize> StackResources<SOCK> {
//     pub fn new() -> Self {
//         Self {
//             sockets: [SocketStorage::EMPTY; SOCK],
//         }
//     }
// }

pub struct Stack<D: atat::asynch::AtatClient> {
    pub(crate) socket: RefCell<SocketStack>,
    inner: RefCell<Inner<D>>,
}

struct Inner<D: atat::asynch::AtatClient> {
    device: D,
    link_up: bool,
}

pub(crate) struct SocketStack {
    pub(crate) sockets: SocketSet<3, 1024>,
    pub(crate) waker: WakerRegistration,
}

impl<D: atat::asynch::AtatClient + 'static> Stack<D> {
    pub fn new<const SOCK: usize>(
        mut device: D,
        // resources: &'static mut StackResources<SOCK>,
    ) -> Self {
        let sockets = SocketSet::new();
        // let sockets = SocketSet::new(&mut resources.sockets[..]);

        let mut socket = SocketStack {
            sockets,
            waker: WakerRegistration::new(),
        };

        let mut inner = Inner {
            device,
            link_up: false,
        };

        Self {
            socket: RefCell::new(socket),
            inner: RefCell::new(inner),
        }
    }

    fn with<R>(&self, f: impl FnOnce(&SocketStack, &Inner<D>) -> R) -> R {
        f(&*self.socket.borrow(), &*self.inner.borrow())
    }

    fn with_mut<R>(&self, f: impl FnOnce(&mut SocketStack, &mut Inner<D>) -> R) -> R {
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

impl<D: atat::asynch::AtatClient + 'static> Inner<D> {
    fn poll(&mut self, cx: &mut Context<'_>, s: &mut SocketStack) {
        s.waker.register(cx.waker());

        let timestamp = Instant::now();
        // let mut smoldev = DriverAdapter {
        //     cx: Some(cx),
        //     inner: &mut self.device,
        // };
        // s.iface.poll(timestamp, &mut smoldev, &mut s.sockets);

        // Update link up
        // let old_link_up = self.link_up;
        // self.link_up = self.device.link_state(cx) == LinkState::Up;

        // // Print when changed
        // if old_link_up != self.link_up {
        //     info!("link_up = {:?}", self.link_up);
        // }

        // if let Some(poll_at) = s.iface.poll_at(timestamp, &mut s.sockets) {
        //     let t = Timer::at(instant_from_smoltcp(poll_at));
        //     pin_mut!(t);
        //     if t.poll(cx).is_ready() {
        //         cx.waker().wake_by_ref();
        //     }
        // }
    }
}
