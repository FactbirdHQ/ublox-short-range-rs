use embedded_hal::digital::OutputPin;
use embedded_io_async::{Read, Write};

use crate::{command::system::types::BaudRate, DEFAULT_BAUD_RATE};

pub trait WifiConfig<'a> {
    type ResetPin: OutputPin;

    const AT_CONFIG: atat::Config = atat::Config::new();

    // Transport settings
    const FLOW_CONTROL: bool = false;
    const BAUD_RATE: BaudRate = DEFAULT_BAUD_RATE;

    #[cfg(feature = "internal-network-stack")]
    const TLS_IN_BUFFER_SIZE: Option<u16> = None;
    #[cfg(feature = "internal-network-stack")]
    const TLS_OUT_BUFFER_SIZE: Option<u16> = None;

    #[cfg(feature = "ppp")]
    const PPP_CONFIG: embassy_net_ppp::Config<'a>;

    fn reset_pin(&mut self) -> Option<&mut Self::ResetPin> {
        None
    }
}

pub trait Transport: Write + Read {
    fn set_baudrate(&mut self, baudrate: u32);
    fn split_ref(&mut self) -> (impl Write, impl Read);
}
