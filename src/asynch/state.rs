#![allow(dead_code)]

use core::cell::RefCell;
use core::future::poll_fn;
use core::task::{Context, Poll};

use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::waitqueue::WakerRegistration;

use crate::connection::{WiFiState, WifiConnection};

/// The link state of a network device.
#[derive(PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum LinkState {
    /// The link is down.
    Down,
    /// The link is up.
    Up,
}

pub struct State {
    shared: Mutex<NoopRawMutex, RefCell<Shared>>,
}

impl State {
    pub const fn new() -> Self {
        Self {
            shared: Mutex::new(RefCell::new(Shared {
                link_state: LinkState::Down,
                wifi_connection: WifiConnection::new(),
                state_waker: WakerRegistration::new(),
                connection_waker: WakerRegistration::new(),
            })),
        }
    }
}

/// State of the LinkState
pub struct Shared {
    link_state: LinkState,
    wifi_connection: WifiConnection,
    state_waker: WakerRegistration,
    connection_waker: WakerRegistration,
}

#[derive(Clone)]
pub struct Runner<'d> {
    shared: &'d Mutex<NoopRawMutex, RefCell<Shared>>,
}

impl<'d> Runner<'d> {
    pub fn new(state: &'d mut State) -> Self {
        Self {
            shared: &state.shared,
        }
    }

    pub fn set_link_state(&mut self, state: LinkState) {
        self.shared.lock(|s| {
            let s = &mut *s.borrow_mut();
            s.link_state = state;
            s.state_waker.wake();
        });
    }

    pub fn link_state(&mut self, cx: &mut Context) -> LinkState {
        self.shared.lock(|s| {
            let s = &mut *s.borrow_mut();
            s.state_waker.register(cx.waker());
            s.link_state
        })
    }

    pub fn update_connection_with(&self, f: impl FnOnce(&mut WifiConnection)) {
        self.shared.lock(|s| {
            let s = &mut *s.borrow_mut();
            f(&mut s.wifi_connection);
            info!(
                "Connection status changed! Connected: {:?}",
                s.wifi_connection.is_connected()
            );

            if s.wifi_connection.network_up
                && matches!(s.wifi_connection.wifi_state, WiFiState::Connected)
            {
                s.link_state = LinkState::Up;
            } else {
                s.link_state = LinkState::Down;
            }

            s.state_waker.wake();
            s.connection_waker.wake();
        })
    }

    pub fn is_connected(&self, cx: Option<&mut Context>) -> bool {
        self.shared.lock(|s| {
            let s = &mut *s.borrow_mut();
            if let Some(cx) = cx {
                s.connection_waker.register(cx.waker());
            }
            s.wifi_connection.is_connected()
        })
    }

    pub async fn wait_connection_change(&mut self) -> bool {
        let old_state = self
            .shared
            .lock(|s| s.borrow().wifi_connection.is_connected());

        poll_fn(|cx| {
            let current_state = self.is_connected(Some(cx));
            if current_state != old_state {
                return Poll::Ready(current_state);
            }
            Poll::Pending
        })
        .await
    }
}
