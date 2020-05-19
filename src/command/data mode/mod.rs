//! ### 20 - GPIO Commands
//! The section describes the AT commands used to configure the GPIO pins provided by u-blox cellular modules
//! ### GPIO functions
//! On u-blox cellular modules, GPIO pins can be opportunely configured as general purpose input or output.
//! Moreover GPIO pins of u-blox cellular modules can be configured to provide custom functions via +UGPIOC
//! AT command. The custom functions availability can vary depending on the u-blox cellular modules series and
//! version: see Table 53 for an overview of the custom functions supported by u-blox cellular modules. \
//! The configuration of the GPIO pins (i.e. the setting of the parameters of the +UGPIOC AT command) is saved
//! in the NVM and used at the next power-on.
pub mod responses;
pub mod types;

use atat::atat_derive::AtatCmd;
use heapless::{consts, String};
use responses::*;
use types::*;

use super::NoResponse;

/// 5.1 Enter data mode O
///
/// Requests the module to move to the new mode.
/// After executing the data mode command or the extended data mode command, a delay of 50 ms is
/// required before start of data transmission.
#[derive(Clone, AtatCmd)]
#[at_cmd("O", NoResponse, timeout_ms = 10000)]
pub struct ChangeMode{
    #[at_arg(position = 0)]
    pub mode: Mode,
}

/// 5.2 Connect peer +UDCP
///
/// Connects to an enabled service on a remote device. When the host connects to a
/// service on a remote device, it implicitly registers to receive the "Connection Closed"
/// event.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UDCP", ConnectPeerResponse, timeout_ms = 10000)]
pub struct ConnectPeer {
    #[at_arg(position = 0)]
    pub url: String<consts::U64>,
    // pub url: URL,
}

/// 5.3 Close peer connection +UDCPC
///
/// Closes an existing peer connection.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UDCPC", NoResponse, timeout_ms = 10000)]
pub struct ClosePeerConnection{
    #[at_arg(position = 0)]
    pub peer_handle: u32,
}

/// 5.4 Default remote peer +UDDRP
///
/// The default remote peer command works for Bluetooth BR/EDR, Bluetooth low energy (SPS), TCP, and UDP.
/// The DCE will connect to a default remote peer when entering either the Data mode or Extended data mode
/// (either by command or at start up, if defined by the Module Start Mode +UMSM command).
#[derive(Clone, AtatCmd)]
#[at_cmd("+UDDRP", NoResponse, timeout_ms = 10000)]
pub struct SetDefaultRemotePeer{
    /// For ODIN-W2, the peer ID can be 0-6. 
    pub peer_id: u8,
    pub url: String<consts::U64>,
    pub connect_scheme: ConnectScheme,
}

/// 5.5 Peer list +UDLP
///
/// This command reads the connected peers (peer handle).
#[derive(Clone, AtatCmd)]
#[at_cmd("+UDLP?", PeerListResponse, timeout_ms = 10000)]
pub struct PeerList;

/// 5.6 Server configuration +UDSC
///
/// Writes server configuration. Only one option from option2 is to be used. 
#[derive(Clone, AtatCmd)]
#[at_cmd("+UDSC", NoResponse, timeout_ms = 10000)]
pub struct ServerConfiguration{
    /// 0-6, the server ID to configure. Disable an active server first before changing.
    #[at_arg(position = 0)]
    pub id: u8,
    #[at_arg(position = 1)]
    pub server_type: ServerType,
    #[at_arg(position = 2)]
    pub option1: Option<String<consts::U64>>,
    /// For UDP,<option2> specifies the behaviour of incoming data.
    #[at_arg(position = 3)]
    pub option2_1: Option<UDPBehaviour>,
    /// For UUID,<option2> specifies the 128-bit UUID identifier.
    #[at_arg(position = 3)]
    pub option2_2: Option<&u8>,
    /// For ATP, <option2> specifies the listening port if the AT-service is started on a TCP or
    /// UDP interface
    #[at_arg(position = 3)]
    pub option2_3: Option<u16>,
    /// For TCP, <option2> specifies if there should be an immediate flush after a write.
    #[at_arg(position = 3)]
    pub option2_4: Option<ImmediateFlush>,
    /// For UDP, <option3> specifies IP version of the started service.
    #[at_arg(position = 4)]
    pub option3: Option<IPVersion>,
}

/// 5.7 Server flags +UDSF
///
/// Bit 0, remote configuration: When the remote configuration bit is set, the module will look for the escape
/// sequence over the air (see S2 command). When the escape sequence is detected, the channel will enter
/// command mode and parse AT commands. The command mode is exited by sending an ATO to the module (see
/// O command).
#[derive(Clone, AtatCmd)]
#[at_cmd("+UDSF", NoResponse, timeout_ms = 10000)]
pub struct SetServerFlags{
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
#[at_cmd("+UDWS", NoResponse, timeout_ms = 10000)]
pub struct SetWatchdogSettings{
    #[at_arg(position = 0)]
    pub setting_type: WatchdogSetting,
    #[at_arg(position = 1)]
    pub value: u32,
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
#[at_cmd("+UDCFG", NoResponse, timeout_ms = 10000)]
pub struct SetPeerConfiguration{
    #[at_arg(position = 0)]
    pub parameter: PeerConfigParameter,
    #[at_arg(position = 1)]
    pub config_value: u32,
}

/// 5.12 Bind +UDBIND
///
/// Writes backspace character.
/// This setting changes the decimal value of the character recognized by the DCE as a
/// request to delete from the command line, the immediately preceding character.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UDBIND", BindResponse, timeout_ms = 10000)]
pub struct SetBind{
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
#[at_cmd("+UDBINDC", NoResponse, timeout_ms = 10000)]
pub struct SoftwareUpdate{
    #[at_arg(position = 0)]
    pub stream_id: SoftwareUpdateMode,
    #[at_arg(position = 1)]
    pub channel_id: SoftwareUpdateBaudRate
}