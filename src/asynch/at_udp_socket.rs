use embassy_net::{udp::UdpSocket, Ipv4Address};
use embedded_io_async::{Read, Write};

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
