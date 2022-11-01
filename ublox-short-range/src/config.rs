use embedded_hal::digital::{ErrorType, OutputPin};
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
    pub(crate) tls_in_buffer_size: Option<u16>,
    pub(crate) tls_out_buffer_size: Option<u16>,
    pub(crate) max_urc_attempts: u8,
    pub(crate) network_up_bug: bool,
}

impl Default for Config<NoPin> {
    fn default() -> Self {
        Config {
            rst_pin: None,
            hostname: None,
            tls_in_buffer_size: None,
            tls_out_buffer_size: None,
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
            tls_in_buffer_size: None,
            tls_out_buffer_size: None,
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
    ///
    /// For Odin:
    /// Minimum is 512 and maximum is 16K (16384).
    /// DEFAULT_TLS_IN_BUFFER_SIZE (7800)
    pub fn tls_in_buffer_size(self, bytes: u16) -> Self {
        assert!(bytes > 512);
        Config {
            tls_in_buffer_size: Some(bytes),
            ..self
        }
    }

    /// Experimental use of undocumented setting for TLS buffers
    ///
    /// For Odin:
    /// Minimum is 512 and maximum is 16K (16384).
    /// DEFAULT_TLS_OUT_BUFFER_SIZE (3072)
    pub fn tls_out_buffer_size(self, bytes: u16) -> Self {
        assert!(bytes > 512);
        Config {
            tls_out_buffer_size: Some(bytes),
            ..self
        }
    }
}
