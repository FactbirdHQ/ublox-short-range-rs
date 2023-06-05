pub mod control;
pub mod runner;
#[cfg(feature = "ublox-sockets")]
pub mod ublox_stack;

use crate::command::edm::urc::EdmEvent;
use atat::{asynch::AtatClient, UrcSubscription};
use ch::Device;
use embassy_net_driver_channel as ch;
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};
use embedded_hal::digital::OutputPin;
use runner::Runner;

use self::control::Control;

pub struct State<AT: AtatClient> {
    ch: ch::State<MTU, 1, 1>,
    at_handle: Mutex<NoopRawMutex, AT>,
}

impl<AT: AtatClient> State<AT> {
    pub fn new(at_handle: AT) -> Self {
        Self {
            ch: ch::State::new(),
            at_handle: Mutex::new(at_handle),
        }
    }
}

pub const MTU: usize = 4096 + 2; // DATA_PACKAGE_SIZE

pub async fn new<'a, AT: AtatClient, RST: OutputPin>(
    state: &'a mut State<AT>,
    urc_subscription: UrcSubscription<'a, EdmEvent>,
    reset: RST,
) -> (Device<'a, MTU>, Control<'a, AT>, Runner<'a, AT, RST>) {
    let (ch_runner, net_device) = ch::new(&mut state.ch, [0; 6]);
    let state_ch = ch_runner.state_runner();

    let mut runner = Runner::new(ch_runner, &state.at_handle, reset, urc_subscription);

    runner.init().await.unwrap();

    let mut control = Control::new(state_ch, &state.at_handle);
    control.init().await.unwrap();

    (net_device, control, runner)
}
