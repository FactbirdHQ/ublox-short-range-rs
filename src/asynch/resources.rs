use atat::{ResponseSlot, UrcChannel};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Channel};

use super::{
    runner::{MAX_CMD_LEN, URC_SUBSCRIBERS},
    state, UbloxUrc,
};

pub struct Resources<const INGRESS_BUF_SIZE: usize, const URC_CAPACITY: usize> {
    pub(crate) ch: state::State,

    pub(crate) res_slot: ResponseSlot<INGRESS_BUF_SIZE>,
    pub(crate) req_slot: Channel<NoopRawMutex, heapless::Vec<u8, MAX_CMD_LEN>, 1>,
    pub(crate) urc_channel: UrcChannel<UbloxUrc, URC_CAPACITY, { URC_SUBSCRIBERS }>,
    pub(crate) ingress_buf: [u8; INGRESS_BUF_SIZE],
}

impl<const INGRESS_BUF_SIZE: usize, const URC_CAPACITY: usize> Default
    for Resources<INGRESS_BUF_SIZE, URC_CAPACITY>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<const INGRESS_BUF_SIZE: usize, const URC_CAPACITY: usize>
    Resources<INGRESS_BUF_SIZE, URC_CAPACITY>
{
    pub fn new() -> Self {
        Self {
            ch: state::State::new(),

            res_slot: ResponseSlot::new(),
            req_slot: Channel::new(),
            urc_channel: UrcChannel::new(),
            ingress_buf: [0; INGRESS_BUF_SIZE],
        }
    }
}
