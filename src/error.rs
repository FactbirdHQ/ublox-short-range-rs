use at::Error as ATError;
use heapless::{consts::U64, String};

/// Error that occurs when attempting to connect to a wireless network.
#[derive(Debug)]
pub enum WifiConnectionError {
    /// Failed to connect to wireless network.
    FailedToConnect(String<U64>),
    /// Failed to disconnect from wireless network. Try turning the wireless interface down.
    FailedToDisconnect(String<U64>),
    /// A wireless error occurred.
    Other {
        kind: WifiError,
    },

    BufferOverflow,
    // SsidNotFound,
}

#[derive(Debug)]
pub enum WifiError {
    // The specified wifi  is currently disabled. Try switching it on.
    WifiDisabled,
    UnexpectedResponse,
    /// The wifi interface interface failed to switch on.
    InterfaceFailedToOn,
    // IO Error occurred.
    // IoError(IoError),
    // AT Error occurred.
    ATError(ATError),

    // FIXME: Temp fix!
    Other,
}

#[derive(Debug)]
pub enum WifiHotspotError {
    /// Failed to ceate wireless hotspot.
    CreationFailed,
    /// Failed to stop wireless hotspot service. Try turning off
    /// the wireless interface via ```wifi.turn_off()```.
    // FailedToStop(IoError),
    /// A wireless interface error occurred.
    Other { kind: WifiError },
}

impl From<WifiError> for WifiHotspotError {
    fn from(error: WifiError) -> Self {
        WifiHotspotError::Other { kind: error }
    }
}

impl From<WifiError> for WifiConnectionError {
    fn from(error: WifiError) -> Self {
        WifiConnectionError::Other { kind: error }
    }
}

impl From<ATError> for WifiConnectionError {
    fn from(error: ATError) -> Self {
        WifiConnectionError::Other {
            kind: WifiError::ATError(error),
        }
    }
}

impl From<ATError> for WifiError {
    fn from(error: ATError) -> Self {
        WifiError::ATError(error)
    }
}
