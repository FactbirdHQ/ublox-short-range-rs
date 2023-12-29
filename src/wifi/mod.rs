pub use ublox_sockets::{PeerHandle, SocketHandle};

use crate::command::edm::types::ChannelId;

pub mod ap;
pub mod connection;
pub mod network;
pub mod options;
pub mod supplicant;

pub mod peer_builder;

pub(crate) const EGRESS_CHUNK_SIZE: usize = 512;
/// The socket map, keeps mappings between `ublox::sockets`s `SocketHandle`,
/// and the modems `PeerHandle` and `ChannelId`. The peer handle is used
/// for controlling the connection, while the channel id is used for sending
/// data over the connection in EDM mode.
pub struct SocketMap {
    channel_map: heapless::FnvIndexMap<ChannelId, SocketHandle, 4>,
    peer_map: heapless::FnvIndexMap<PeerHandle, SocketHandle, 4>,
}

impl Default for SocketMap {
    fn default() -> Self {
        Self::new()
    }
}

impl SocketMap {
    fn new() -> Self {
        Self {
            channel_map: heapless::FnvIndexMap::new(),
            peer_map: heapless::FnvIndexMap::new(),
        }
    }

    pub fn insert_channel(
        &mut self,
        channel_id: ChannelId,
        socket_handle: SocketHandle,
    ) -> Result<(), ()> {
        trace!("[SOCK_MAP] {:?} tied to {:?}", socket_handle, channel_id);
        self.channel_map
            .insert(channel_id, socket_handle)
            .map_err(drop)?;
        Ok(())
    }

    pub fn remove_channel(&mut self, channel_id: &ChannelId) -> Result<(), ()> {
        trace!("[SOCK_MAP] {:?} removed", channel_id);
        self.channel_map.remove(channel_id).ok_or(())?;
        Ok(())
    }

    pub fn channel_to_socket(&self, channel_id: &ChannelId) -> Option<&SocketHandle> {
        self.channel_map.get(channel_id)
    }

    pub fn socket_to_channel_id(&self, socket_handle: &SocketHandle) -> Option<&ChannelId> {
        self.channel_map
            .iter()
            .find_map(|(c, s)| if s == socket_handle { Some(c) } else { None })
    }

    pub fn insert_peer(&mut self, peer: PeerHandle, socket_handle: SocketHandle) -> Result<(), ()> {
        trace!("[SOCK_MAP] {:?} tied to {:?}", socket_handle, peer);
        self.peer_map.insert(peer, socket_handle).map_err(drop)?;
        Ok(())
    }

    pub fn remove_peer(&mut self, peer: &PeerHandle) -> Result<(), ()> {
        trace!("[SOCK_MAP] {:?} removed", peer);
        self.peer_map.remove(peer).ok_or(())?;
        Ok(())
    }

    pub fn peer_to_socket(&self, peer: &PeerHandle) -> Option<&SocketHandle> {
        self.peer_map.get(peer)
    }

    pub fn socket_to_peer(&self, socket_handle: &SocketHandle) -> Option<&PeerHandle> {
        self.peer_map
            .iter()
            .find_map(|(c, s)| if s == socket_handle { Some(c) } else { None })
    }
}
