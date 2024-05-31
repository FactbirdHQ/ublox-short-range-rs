mod at_udp_socket;
pub mod control;
pub mod network;
mod resources;
pub mod runner;
#[cfg(feature = "ublox-sockets")]
pub mod ublox_stack;

pub(crate) mod state;

pub use resources::Resources;
pub use runner::Runner;

#[cfg(feature = "internal-network-stack")]
mod internal_stack;
use embedded_io_async::{BufRead, Error as _, ErrorKind, Read, Write};

#[cfg(feature = "edm")]
pub type UbloxUrc = crate::command::edm::urc::EdmEvent;

#[cfg(not(feature = "edm"))]
pub type UbloxUrc = crate::command::Urc;

pub struct ReadWriteAdapter<R, W>(pub R, pub W);

impl<R, W> embedded_io_async::ErrorType for ReadWriteAdapter<R, W> {
    type Error = ErrorKind;
}

impl<R: Read, W> Read for ReadWriteAdapter<R, W> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.0.read(buf).await.map_err(|e| e.kind())
    }
}

impl<R: BufRead, W> BufRead for ReadWriteAdapter<R, W> {
    async fn fill_buf(&mut self) -> Result<&[u8], Self::Error> {
        self.0.fill_buf().await.map_err(|e| e.kind())
    }

    fn consume(&mut self, amt: usize) {
        self.0.consume(amt)
    }
}

impl<R, W: Write> Write for ReadWriteAdapter<R, W> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.1.write(buf).await.map_err(|e| e.kind())
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.1.flush().await.map_err(|e| e.kind())
    }
}
