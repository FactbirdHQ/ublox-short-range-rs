use ublox_sockets::SocketHandle;

use crate::command::edm::types::ChannelId;

pub mod ap;
pub mod connection;
pub mod dns;
pub mod network;
pub mod options;
pub mod sta;
pub mod tls;

pub mod peer_builder;

#[cfg(feature = "socket-udp")]
pub mod udp_stack;

#[cfg(feature = "socket-tcp")]
pub mod tcp_stack;

pub(crate) const EGRESS_CHUNK_SIZE: usize = 512;

pub struct EdmMap(heapless::FnvIndexMap<ChannelId, SocketHandle, 8>);

impl Default for EdmMap {
    fn default() -> Self {
        Self::new()
    }
}

impl EdmMap {
    pub fn new() -> Self {
        Self(heapless::FnvIndexMap::new())
    }

    pub fn insert(&mut self, channel_id: ChannelId, socket_handle: SocketHandle) -> Result<(), ()> {
        defmt::trace!("[EDM_MAP] {:?} tied to {:?}", socket_handle, channel_id);
        self.0.insert(channel_id, socket_handle).map_err(drop)?;
        Ok(())
    }

    pub fn remove(&mut self, channel_id: &ChannelId) -> Result<(), ()> {
        defmt::trace!("[EDM_MAP] {:?} removed", channel_id);
        self.0.remove(channel_id).ok_or(())?;
        Ok(())
    }

    pub fn socket_handle(&self, channel_id: &ChannelId) -> Option<&SocketHandle> {
        self.0.get(channel_id)
    }

    pub fn channel_id(&self, socket_handle: &SocketHandle) -> Option<&ChannelId> {
        self.0
            .iter()
            .find_map(|(c, s)| if s == socket_handle { Some(c) } else { None })
    }
}
