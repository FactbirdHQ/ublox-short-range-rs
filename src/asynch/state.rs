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
    /// Device is not yet initialized.
    Uninitialized,
    /// The link is down.
    Down,
    /// The link is up.
    Up,
}

pub(crate) struct State {
    shared: Mutex<NoopRawMutex, RefCell<Shared>>,
}

impl State {
    pub(crate) const fn new() -> Self {
        Self {
            shared: Mutex::new(RefCell::new(Shared {
                link_state: LinkState::Uninitialized,
                wifi_connection: WifiConnection::new(),
                state_waker: WakerRegistration::new(),
                connection_waker: WakerRegistration::new(),
            })),
        }
    }
}

/// State of the LinkState
pub(crate) struct Shared {
    link_state: LinkState,
    wifi_connection: WifiConnection,
    state_waker: WakerRegistration,
    connection_waker: WakerRegistration,
}

#[derive(Clone)]
pub(crate) struct Runner<'d> {
    shared: &'d Mutex<NoopRawMutex, RefCell<Shared>>,
}

impl<'d> Runner<'d> {
    pub(crate) fn new(state: &'d mut State) -> Self {
        Self {
            shared: &state.shared,
        }
    }

    pub(crate) fn mark_initialized(&self) {
        self.shared.lock(|s| {
            let s = &mut *s.borrow_mut();
            s.link_state = LinkState::Down;
            s.state_waker.wake();
        })
    }

    pub(crate) async fn wait_for_initialized(&self) {
        if self.link_state(None) != LinkState::Uninitialized {
            return;
        }

        poll_fn(|cx| {
            if self.link_state(Some(cx)) != LinkState::Uninitialized {
                return Poll::Ready(());
            }
            Poll::Pending
        })
        .await
    }

    pub(crate) fn link_state(&self, cx: Option<&mut Context>) -> LinkState {
        self.shared.lock(|s| {
            let s = &mut *s.borrow_mut();
            if let Some(cx) = cx {
                s.state_waker.register(cx.waker());
            }
            s.link_state
        })
    }

    pub(crate) async fn wait_for_link_state(&self, ls: LinkState) {
        if self.link_state(None) == ls {
            return;
        }

        poll_fn(|cx| {
            if self.link_state(Some(cx)) == ls {
                return Poll::Ready(());
            }
            Poll::Pending
        })
        .await
    }

    pub(crate) fn update_connection_with(&self, f: impl FnOnce(&mut WifiConnection)) {
        self.shared.lock(|s| {
            let s = &mut *s.borrow_mut();
            f(&mut s.wifi_connection);
            info!(
                "Connection status changed! Connected: {:?}",
                s.wifi_connection.is_connected()
            );

            s.link_state = if s.wifi_connection.is_connected() {
                LinkState::Up
            } else {
                LinkState::Down
            };

            s.state_waker.wake();
            s.connection_waker.wake();
        })
    }

    pub(crate) fn connection_down(&self, cx: Option<&mut Context>) -> bool {
        self.shared.lock(|s| {
            let s = &mut *s.borrow_mut();
            if let Some(cx) = cx {
                s.connection_waker.register(cx.waker());
            }
            s.wifi_connection.ipv4.is_none() && s.wifi_connection.ipv6.is_none()
        })
    }

    pub(crate) async fn wait_connection_down(&self) {
        if self.connection_down(None) {
            return;
        }

        poll_fn(|cx| {
            if self.connection_down(Some(cx)) {
                return Poll::Ready(());
            }
            Poll::Pending
        })
        .await
    }

    pub(crate) fn is_connected(&self, cx: Option<&mut Context>) -> bool {
        self.shared.lock(|s| {
            let s = &mut *s.borrow_mut();
            if let Some(cx) = cx {
                s.connection_waker.register(cx.waker());
            }
            s.wifi_connection.is_connected()
        })
    }

    pub(crate) fn wifi_state(&self, cx: Option<&mut Context>) -> WiFiState {
        self.shared.lock(|s| {
            let s = &mut *s.borrow_mut();
            if let Some(cx) = cx {
                s.connection_waker.register(cx.waker());
            }
            s.wifi_connection.wifi_state
        })
    }

    pub(crate) async fn wait_for_wifi_state_change(&self) -> WiFiState {
        let old_state = self.wifi_state(None);

        poll_fn(|cx| {
            let new_state = self.wifi_state(Some(cx));
            if old_state != new_state {
                return Poll::Ready(new_state);
            }
            Poll::Pending
        })
        .await
    }
}
