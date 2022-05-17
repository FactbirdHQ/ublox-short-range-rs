#[derive(Debug, defmt::Format)]
pub enum Error {
    SetState,
    BadLength,
    Network,
    Pin,
    BaudDetection,
    SocketClosed,
    WrongSocketType,
    SocketNotFound,
    SocketNotConnected,
    MissingSocketSet,
    NetworkState(crate::wifi::connection::NetworkState),
    NoWifiSetup,
    WifiState(crate::wifi::connection::WiFiState),
    Socket(ublox_sockets::Error),
    AT(atat::Error),
    Busy,
    InvalidHex,
    Dns(crate::command::ping::types::PingError),
    DuplicateCredentials,
    Uninitialized,
    Unimplemented,
    SocketMemory,
    SocketMapMemory,
    Timer,
    _Unknown,
}

impl From<atat::Error> for Error {
    fn from(e: atat::Error) -> Self {
        Error::AT(e)
    }
}

impl From<ublox_sockets::Error> for Error {
    fn from(e: ublox_sockets::Error) -> Self {
        Error::Socket(e)
    }
}

/// Error that occurs when attempting to connect to a wireless network.
#[derive(Debug, defmt::Format)]
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
    Internal(Error),
}

impl From<Error> for WifiConnectionError {
    fn from(e: Error) -> Self {
        WifiConnectionError::Internal(e)
    }
}

#[derive(Debug, defmt::Format)]
pub enum WifiError {
    // The specified wifi  is currently disabled. Try switching it on.
    WifiDisabled,
    UnexpectedResponse,
    /// The wifi interface interface failed to switch on.
    InterfaceFailedToOn,
    // IO Error occurred.
    // IoError(IoError),
    // AT Error occurred.
    ATError(atat::Error),
    HexError,
    // FIXME: Temp fix!
    // Other,
}

#[derive(Debug, defmt::Format)]
pub enum WifiHotspotError {
    /// Failed to ceate wireless hotspot.
    CreationFailed,
    /// Failed to stop wireless hotspot service. Try turning off
    /// the wireless interface via ```wifi.turn_off()```.
    FailedToStop,
    /// A wireless interface error occurred.
    Other {
        kind: WifiError,
    },
    Internal(Error),
}

impl From<Error> for WifiHotspotError {
    fn from(e: Error) -> Self {
        WifiHotspotError::Internal(e)
    }
}

impl From<WifiError> for WifiHotspotError {
    fn from(error: WifiError) -> Self {
        WifiHotspotError::Other { kind: error }
    }
}

impl From<atat::Error> for WifiHotspotError {
    fn from(error: atat::Error) -> Self {
        WifiHotspotError::Other {
            kind: WifiError::ATError(error),
        }
    }
}

impl From<WifiError> for WifiConnectionError {
    fn from(error: WifiError) -> Self {
        WifiConnectionError::Other { kind: error }
    }
}

impl From<atat::Error> for WifiConnectionError {
    fn from(error: atat::Error) -> Self {
        WifiConnectionError::Other {
            kind: WifiError::ATError(error),
        }
    }
}

impl From<atat::Error> for WifiError {
    fn from(error: atat::Error) -> Self {
        WifiError::ATError(error)
    }
}
