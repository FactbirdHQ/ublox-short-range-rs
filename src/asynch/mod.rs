#[cfg(feature = "ppp")]
mod at_udp_socket;
pub mod control;
pub mod network;
mod resources;
pub mod runner;
#[cfg(feature = "internal-network-stack")]
pub mod ublox_stack;

pub(crate) mod state;

pub use resources::Resources;
pub use runner::Runner;
pub use state::LinkState;

#[cfg(feature = "edm")]
pub type UbloxUrc = crate::command::edm::urc::EdmEvent;

#[cfg(not(feature = "edm"))]
pub type UbloxUrc = crate::command::Urc;
