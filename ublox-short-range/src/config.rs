use embedded_hal::digital::blocking::OutputPin;
use heapless::String;

pub struct NoPin;

impl OutputPin for NoPin {
    type Error = core::convert::Infallible;

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
    pub(crate) max_tls_in_buffer: bool,
    pub(crate) max_urc_attempts: u8,
    pub(crate) network_up_bug: bool,
}

impl Default for Config<NoPin> {
    fn default() -> Self {
        Config {
            rst_pin: None,
            hostname: None,
            max_tls_in_buffer: false,
            max_urc_attempts: 5,
            network_up_bug: true,
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
            max_tls_in_buffer: false,
            max_urc_attempts: 5,
            network_up_bug: true,
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

    pub fn max_urc_attempts(self, max_attempts: u8) -> Self {
        Config {
            max_urc_attempts: max_attempts,
            ..self
        }
    }

    /// Experimental use of undocumented setting for TLS buffers
    /// Used to enable streaming TLS with client certificate
    pub fn max_tls_in_buffer(self) -> Self {
        Config {
            max_tls_in_buffer: true,
            ..self
        }
    }
}
