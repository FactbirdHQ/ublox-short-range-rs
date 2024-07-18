use embassy_net::{udp::UdpSocket, Ipv4Address};
use embedded_io_async::{Read, Write};

use crate::config::Transport;

pub struct AtUdpSocket<'a>(pub(crate) UdpSocket<'a>);

impl<'a> AtUdpSocket<'a> {
    pub(crate) const PPP_AT_PORT: u16 = 23;
}

impl<'a> embedded_io_async::ErrorType for &AtUdpSocket<'a> {
    type Error = core::convert::Infallible;
}

impl<'a> Read for &AtUdpSocket<'a> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let (len, _) = self.0.recv_from(buf).await.unwrap();
        Ok(len)
    }
}

impl<'a> Write for &AtUdpSocket<'a> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.0
            .send_to(
                buf,
                (Ipv4Address::new(172, 30, 0, 251), AtUdpSocket::PPP_AT_PORT),
            )
            .await
            .unwrap();

        Ok(buf.len())
    }
}

impl<'a> Transport for AtUdpSocket<'a> {
    fn set_baudrate(&mut self, _baudrate: u32) {
        // Nothing to do here
    }

    fn split_ref(&mut self) -> (impl Write, impl Read) {
        (&*self, &*self)
    }
}

impl<'a> embedded_io_async::ErrorType for AtUdpSocket<'a> {
    type Error = core::convert::Infallible;
}

impl<'a> Read for AtUdpSocket<'a> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let (len, _) = self.0.recv_from(buf).await.unwrap();
        Ok(len)
    }
}

impl<'a> Write for AtUdpSocket<'a> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.0
            .send_to(
                buf,
                (Ipv4Address::new(172, 30, 0, 251), AtUdpSocket::PPP_AT_PORT),
            )
            .await
            .unwrap();

        Ok(buf.len())
    }
}
