pub mod control;
pub mod runner;
#[cfg(feature = "ublox-sockets")]
pub mod ublox_stack;

pub(crate) mod state;

use crate::command::edm::{urc::EdmEvent, EdmAtCmdWrapper};
use atat::{asynch::AtatClient, AtatUrcChannel};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};
use embedded_hal::digital::OutputPin;
use runner::Runner;
use state::Device;

use self::control::Control;

// NOTE: Must be pow(2) due to internal usage of `FnvIndexMap`
const MAX_CONNS: usize = 8;

pub struct AtHandle<'d, AT: AtatClient>(&'d Mutex<NoopRawMutex, AT>);

impl<'d, AT: AtatClient> AtHandle<'d, AT> {
    async fn send_edm<Cmd: atat::AtatCmd<LEN>, const LEN: usize>(
        &mut self,
        cmd: Cmd,
    ) -> Result<Cmd::Response, atat::Error> {
        self.send(EdmAtCmdWrapper(cmd)).await
    }

    async fn send<Cmd: atat::AtatCmd<LEN>, const LEN: usize>(
        &mut self,
        cmd: Cmd,
    ) -> Result<Cmd::Response, atat::Error> {
        self.0.lock().await.send_retry::<Cmd, LEN>(&cmd).await
    }
}

pub struct State<AT: AtatClient> {
    ch: state::State,
    at_handle: Mutex<NoopRawMutex, AT>,
}

impl<AT: AtatClient> State<AT> {
    pub fn new(at_handle: AT) -> Self {
        Self {
            ch: state::State::new(),
            at_handle: Mutex::new(at_handle),
        }
    }
}

pub async fn new<'a, AT: AtatClient, SUB: AtatUrcChannel<EdmEvent>, RST: OutputPin>(
    state: &'a mut State<AT>,
    subscriber: &'a SUB,
    reset: RST,
) -> (
    Device<'a, AT>,
    Control<'a, AT>,
    Runner<'a, AT, RST, MAX_CONNS>,
) {
    let (ch_runner, net_device) = state::new(
        &mut state.ch,
        AtHandle(&state.at_handle),
        subscriber.subscribe().unwrap(),
    );
    let state_ch = ch_runner.state_runner();

    let mut runner = Runner::new(
        ch_runner,
        AtHandle(&state.at_handle),
        reset,
        subscriber.subscribe().unwrap(),
    );

    runner.init().await.unwrap();

    let mut control = Control::new(state_ch, AtHandle(&state.at_handle));
    control.init().await.unwrap();

    (net_device, control, runner)
}
