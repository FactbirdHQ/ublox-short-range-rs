use core::cell::RefCell;

use atat::UrcChannel;

use crate::asynch::{control::ProxyClient, runner::URC_SUBSCRIBERS, state, UbloxUrc};

pub struct Device<'a, const INGRESS_BUF_SIZE: usize, const URC_CAPACITY: usize> {
    pub(crate) state_ch: state::Runner<'a>,
    pub(crate) at_client: RefCell<ProxyClient<'a, INGRESS_BUF_SIZE>>,
    pub(crate) urc_channel: &'a UrcChannel<UbloxUrc, URC_CAPACITY, URC_SUBSCRIBERS>,
}
