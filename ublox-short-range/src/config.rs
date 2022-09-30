use embedded_hal::digital::{OutputPin, ErrorType};
use heapless::String;

pub struct NoPin;

impl ErrorType for NoPin {
    type Error = core::convert::Infallible;
}

impl OutputPin for NoPin {
    fn set_low(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct Config<RST> {
    pub(crate) rst_pin: Option<RST>,
    pub(crate) hostname: Option<String<20>>,
}

impl Default for Config<NoPin> {
    fn default() -> Self {
        Config {
            rst_pin: None,
            hostname: None,
        }
    }
}

impl<RST> Config<RST>
where
    RST: OutputPin,
{
    pub fn new() -> Self {
        Config {
            rst_pin: None,
            hostname: None,
        }
    }

    pub fn with_rst(self, rst_pin: RST) -> Self {
        Config {
            rst_pin: Some(rst_pin),
            ..self
        }
    }

    pub fn with_hostname(self, hostname: &str) -> Self {
        Config {
            hostname: Some(String::from(hostname)),
            ..self
        }
    }
}
