//! ### 5 - Data Mode
pub mod responses;
pub mod types;
pub mod urc;

use atat::atat_derive::AtatCmd;
use heapless::String;
use responses::*;
use types::*;

use super::NoResponse;

/// 5.1 Enter data mode O
///
/// Requests the module to move to the new mode.
/// After executing the data mode command or the extended data mode command, a delay of 50 ms is
/// required before start of data transmission.
#[derive(Clone, AtatCmd)]
#[at_cmd("O", NoResponse, timeout_ms = 1000, value_sep = false)]
pub struct ChangeMode {
    #[at_arg(position = 0)]
    pub mode: Mode,
}

/// 5.2 Connect peer +UDCP
///
/// Connects to an enabled service on a remote device. When the host connects to a
/// service on a remote device, it implicitly registers to receive the "Connection Closed"
/// event.
#[cfg(feature = "ublox-sockets")]
#[derive(Clone, AtatCmd)]
#[at_cmd("+UDCP", ConnectPeerResponse, timeout_ms = 5000)]
pub struct ConnectPeer<'a> {
    #[at_arg(position = 0, len = 128)]
    pub url: &'a str,
}

/// 5.3 Close peer connection +UDCPC
///
/// Closes an existing peer connection.
#[cfg(feature = "ublox-sockets")]
#[derive(Clone, AtatCmd)]
#[at_cmd("+UDCPC", NoResponse, timeout_ms = 1000)]
pub struct ClosePeerConnection {
    #[at_arg(position = 0, len = 1)]
    pub peer_handle: PeerHandle,
}

/// 5.4 Default remote peer +UDDRP
///
/// The default remote peer command works for Bluetooth BR/EDR, Bluetooth low energy (SPS), TCP, and UDP.
/// The DCE will connect to a default remote peer when entering either the Data mode or Extended data mode
/// (either by command or at start up, if defined by the Module Start Mode +UMSM command).
#[derive(Clone, AtatCmd)]
#[at_cmd("+UDDRP", NoResponse, timeout_ms = 1000)]
pub struct SetDefaultRemotePeer<'a> {
    /// For ODIN-W2, the peer ID can be 0-6.
    #[at_arg(position = 0)]
    pub peer_id: u8,
    #[at_arg(position = 1, len = 128)]
    pub url: &'a str,
    #[at_arg(position = 2)]
    pub connect_scheme: ConnectScheme,
}

/// 5.5 Peer list +UDLP
///
/// This command reads the connected peers (peer handle).
#[cfg(feature = "ublox-sockets")]
#[derive(Clone, AtatCmd)]
#[at_cmd("+UDLP?", PeerListResponse, timeout_ms = 1000)]
pub struct PeerList;

/// 5.6 Server configuration +UDSC
///
/// Writes server configuration. Only one option from option2 is to be used.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UDSC", NoResponse, timeout_ms = 1000)]
pub struct ServerConfiguration {
    /// 0-6, the server ID to configure. Disable an active server first before changing.
    #[at_arg(position = 0)]
    pub id: u8,
    #[at_arg(position = 1)]
    pub server_config: ServerType,
}

/// 5.6 Server configuration +UDSC
///
/// Writes server configuration. Only one option from option2 is to be used.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UDSC", NoResponse, timeout_ms = 10000)]
pub struct ServerConfigurationUrl {
    /// 0-6, the server ID to configure. Disable an active server first before changing.
    #[at_arg(position = 0)]
    pub id: u8,
    #[at_arg(position = 1)]
    pub server_config: String<128>,
}

/// 5.7 Server flags +UDSF
///
/// Bit 0, remote configuration: When the remote configuration bit is set, the module will look for the escape
/// sequence over the air (see S2 command). When the escape sequence is detected, the channel will enter
/// command mode and parse AT commands. The command mode is exited by sending an ATO to the module (see
/// O command).
#[derive(Clone, AtatCmd)]
#[at_cmd("+UDSF", NoResponse, timeout_ms = 1000)]
pub struct SetServerFlags {
    /// Id as given by AT+UDSC
    #[at_arg(position = 0)]
    pub id: u8,
    /// Allow remote configuration
    #[at_arg(position = 1)]
    pub flag: RemoteConfiguration,
}

/// 5.8 Watchdog settings +UDWS
///
/// The data watchdog functionality is active only in the data or extended data mode. Additionally, the power
/// mode must also be set to online or sleep mode.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UDWS", NoResponse, timeout_ms = 1000)]
pub struct SetWatchdogSettings {
    #[at_arg(position = 0)]
    pub setting_type: WatchdogSetting,
}

/// 5.9 Configuration +UDCFG
///
/// Writes peer configuration.
///
/// Suported parameter tags | Software Version
/// ------------------------|------------------
/// 0,1                     |   All versions
/// 2                       |    >= 4.0.0
/// 4,5                     |    >= 7.0.0
#[derive(Clone, AtatCmd)]
#[at_cmd("+UDCFG", NoResponse, timeout_ms = 1000)]
pub struct SetPeerConfiguration {
    #[at_arg(position = 0)]
    pub parameter: PeerConfigParameter,
}

/// 5.12 Bind +UDBIND
///
/// Writes backspace character.
/// This setting changes the decimal value of the character recognized by the DCE as a
/// request to delete from the command line, the immediately preceding character.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UDBIND", BindResponse, timeout_ms = 1000)]
pub struct SetBind {
    #[at_arg(position = 0)]
    pub stream_id_1: u8,
    #[at_arg(position = 1)]
    pub stream_id_2: u8,
}

/// 5.13 Bind to channel +UDBINDC
///
/// Binds Stream with Id <StreamId> to channel with Id <ChannelId>. Stream ids are
/// provided on response of a successful connection. Channel id is provided on response
/// of a successful bind command.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UDBINDC", NoResponse, timeout_ms = 1000)]
pub struct SoftwareUpdate {
    #[at_arg(position = 0)]
    pub stream_id: u8,
    #[at_arg(position = 1)]
    pub channel_id: u8,
}
