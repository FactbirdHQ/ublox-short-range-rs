use embassy_rp::uart;
use ublox_short_range::atat;

pub struct TxWrap<TX: embedded_io::asynch::Write>(pub TX);

impl<TX: embedded_io::asynch::Write> embedded_io::Io for TxWrap<TX> {
    type Error = <TX as embedded_io::Io>::Error;
}

impl<TX: embedded_io::asynch::Write> embedded_io::asynch::Write for TxWrap<TX> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.0.write(buf).await
    }
}

impl<T: embassy_rp::uart::Instance> atat::UartExt for TxWrap<uart::BufferedUartTx<'static, T>> {
    type Error = ();

    fn set_baudrate(&mut self, baud: u32) -> Result<(), Self::Error> {
        let r = T::regs();

        let clk_base = 125_000_000;

        let baud_rate_div = (8 * clk_base) / baud;
        let mut baud_ibrd = baud_rate_div >> 7;
        let mut baud_fbrd = ((baud_rate_div & 0x7f) + 1) / 2;

        if baud_ibrd == 0 {
            baud_ibrd = 1;
            baud_fbrd = 0;
        } else if baud_ibrd >= 65535 {
            baud_ibrd = 65535;
            baud_fbrd = 0;
        }

        r.uartcr().modify(|m| {
            m.set_uarten(false);
        });

        // Load PL011's baud divisor registers
        r.uartibrd()
            .write_value(embassy_rp::pac::uart::regs::Uartibrd(baud_ibrd));
        r.uartfbrd()
            .write_value(embassy_rp::pac::uart::regs::Uartfbrd(baud_fbrd));

        // PL011 needs a (dummy) line control register write to latch in the
        // divisors. We don't want to actually change LCR contents here.
        r.uartlcr_h().modify(|_| {});

        r.uartcr().modify(|m| {
            m.set_uarten(true);
        });

        Ok(())
    }
}
