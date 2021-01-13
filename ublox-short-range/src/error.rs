use atat::Error as ATError;
use heapless::{consts::U64, String};
use crate::socket;


#[derive(Debug)]
pub enum Error {
    SetState,
    BadLength,
    Network,
    Pin,
    BaudDetection,
    SocketClosed,
    WrongSocketType,
    SocketNotFound,
    NetworkState(crate::wifi::connection::NetworkState),
    NoNetworkSetup,
    WifiState(crate::wifi::connection::WiFiState),
    Socket(socket::Error),
    BorrowError(core::cell::BorrowError),
    BorrowMutError(core::cell::BorrowMutError),
    AT(atat::Error),
    Busy,
    InvalidHex,
    Dns(crate::command::ping::types::PingError),
    Uninitialized,
    _Unknown,
}

impl From<atat::Error> for Error {
    fn from(e: atat::Error) -> Self {
        Error::AT(e)
    }
}

impl From<socket::Error> for Error {
    fn from(e: crate::socket::Error) -> Self {
        Error::Socket(e)
    }
}

impl From<core::cell::BorrowMutError> for Error {
    fn from(e: core::cell::BorrowMutError) -> Self {
        Error::BorrowMutError(e)
    }
}

impl From<core::cell::BorrowError> for Error {
    fn from(e: core::cell::BorrowError) -> Self {
        Error::BorrowError(e)
    }
}
/// Error that occurs when attempting to connect to a wireless network.
#[derive(Debug)]
pub enum WifiConnectionError {
    /// Failed to connect to wireless network.
    FailedToConnect,
    /// Failed to disconnect from wireless network. Try turning the wireless interface down.
    FailedToDisconnect,
    /// A wireless error occurred.
    Other {
        kind: WifiError,
    },
    WaitingForWifiDeactivation,
    BufferOverflow,
    // SsidNotFound,
    BorrowError(core::cell::BorrowError),
    BorrowMutError(core::cell::BorrowMutError),
    Internal(Error),
}

impl From<Error> for WifiConnectionError{
    fn from(e: Error) -> Self {
        WifiConnectionError::Internal(e)
    }
}

impl From<core::cell::BorrowMutError> for WifiConnectionError {
    fn from(e: core::cell::BorrowMutError) -> Self {
        WifiConnectionError::BorrowMutError(e)
    }
}

impl From<core::cell::BorrowError> for WifiConnectionError {
    fn from(e: core::cell::BorrowError) -> Self {
        WifiConnectionError::BorrowError(e)
    }
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
    HexError,
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
