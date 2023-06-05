use heapless::String;

#[derive(Debug)]
pub struct Config {
    pub(crate) hostname: Option<String<20>>,
    pub(crate) tls_in_buffer_size: Option<u16>,
    pub(crate) tls_out_buffer_size: Option<u16>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            hostname: None,
            tls_in_buffer_size: None,
            tls_out_buffer_size: None,
        }
    }
}

impl Config {
    pub fn new() -> Self {
        Config {
            hostname: None,
            tls_in_buffer_size: None,
            tls_out_buffer_size: None,
        }
    }

    pub fn with_hostname(self, hostname: &str) -> Self {
        Config {
            hostname: Some(String::from(hostname)),
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
