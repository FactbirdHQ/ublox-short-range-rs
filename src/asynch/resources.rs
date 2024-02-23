use core::mem::MaybeUninit;

use atat::{asynch::Client, ResponseSlot, UrcChannel};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};
use embedded_io_async::Write;

use super::{state, UbloxUrc};

pub struct UbxResources<
    W: Write,
    const CMD_BUF_SIZE: usize,
    const INGRESS_BUF_SIZE: usize,
    const URC_CAPACITY: usize,
> {
    pub(crate) ch: state::State,

    pub(crate) res_slot: ResponseSlot<INGRESS_BUF_SIZE>,
    pub(crate) urc_channel: UrcChannel<UbloxUrc, URC_CAPACITY, 2>,
    pub(crate) cmd_buf: [u8; CMD_BUF_SIZE],
    pub(crate) ingress_buf: [u8; INGRESS_BUF_SIZE],

    pub(crate) at_client: MaybeUninit<Mutex<NoopRawMutex, Client<'static, W, INGRESS_BUF_SIZE>>>,

    #[cfg(feature = "ppp")]
    pub(crate) ppp_state: embassy_net_ppp::State<2, 2>,

    #[cfg(feature = "ppp")]
    pub(crate) control_rx: embassy_sync::pipe::Pipe<NoopRawMutex, { super::ppp::SOCKET_BUF_SIZE }>,
    #[cfg(feature = "ppp")]
    pub(crate) control_tx: embassy_sync::pipe::Pipe<NoopRawMutex, { super::ppp::SOCKET_BUF_SIZE }>,
}

impl<
        W: Write,
        const CMD_BUF_SIZE: usize,
        const INGRESS_BUF_SIZE: usize,
        const URC_CAPACITY: usize,
    > UbxResources<W, CMD_BUF_SIZE, INGRESS_BUF_SIZE, URC_CAPACITY>
{
    pub fn new() -> Self {
        Self {
            ch: state::State::new(),

            res_slot: ResponseSlot::new(),
            urc_channel: UrcChannel::new(),
            cmd_buf: [0; CMD_BUF_SIZE],
            ingress_buf: [0; INGRESS_BUF_SIZE],

            at_client: MaybeUninit::uninit(),

            #[cfg(feature = "ppp")]
            ppp_state: embassy_net_ppp::State::new(),

            #[cfg(feature = "ppp")]
            control_rx: embassy_sync::pipe::Pipe::new(),
            #[cfg(feature = "ppp")]
            control_tx: embassy_sync::pipe::Pipe::new(),
        }
    }
}
