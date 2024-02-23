pub mod control;
mod resources;
pub mod runner;
#[cfg(feature = "ublox-sockets")]
pub mod ublox_stack;

pub(crate) mod state;

#[cfg(feature = "internal-network-stack")]
mod internal_stack;
#[cfg(feature = "internal-network-stack")]
pub use internal_stack::{new_internal, InternalRunner, Resources};

#[cfg(feature = "ppp")]
mod ppp;
#[cfg(feature = "ppp")]
pub use ppp::{new_ppp, PPPRunner, Resources};

use atat::asynch::AtatClient;
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};

#[cfg(feature = "edm")]
pub type UbloxUrc = crate::command::edm::urc::EdmEvent;

#[cfg(not(feature = "edm"))]
pub type UbloxUrc = crate::command::Urc;

pub struct AtHandle<'d, AT: AtatClient>(&'d Mutex<NoopRawMutex, AT>);

impl<'d, AT: AtatClient> AtHandle<'d, AT> {
    #[cfg(feature = "edm")]
    async fn send<Cmd: atat::AtatCmd>(&mut self, cmd: Cmd) -> Result<Cmd::Response, atat::Error> {
        self.send_raw(crate::command::edm::EdmAtCmdWrapper(cmd))
            .await
    }

    #[cfg(not(feature = "edm"))]
    async fn send<Cmd: atat::AtatCmd>(&mut self, cmd: Cmd) -> Result<Cmd::Response, atat::Error> {
        self.send_raw(cmd).await
    }

    async fn send_raw<Cmd: atat::AtatCmd>(
        &mut self,
        cmd: Cmd,
    ) -> Result<Cmd::Response, atat::Error> {
        self.0.lock().await.send_retry::<Cmd>(&cmd).await
    }
}
