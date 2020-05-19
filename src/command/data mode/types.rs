//! Argument and parameter types used by GPIO Commands and Responses

use serde_repr::{Deserialize_repr, Serialize_repr};
use ufmt::derive::uDebug;
use no_std_net::IpAddr;

#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum OnOff{
    Off = 0,
    On = 1,
}

#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum Mode {
    /// Command mode
    CommandMode = 0,
    /// 1: Data mode (default)
    DataMode = 1,
    /// 2: Extended data mode (EDM): For NINA-B1 and ANNA-B112, the EDM is supported
    /// only from software version 2.0.0 onwards.
    ExtendedDataMode = 2,
    /// 3: PPP mode: Supported by ODIN-W2 only.
    PPPMode = 3,
}

#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum ConnectScheme{
    /// Always connected - Keep the peer connected when not in command mode.
    /// That is, on errors and remote disconnect, the peer will automatically try to
    /// reconnect.
    /// For the Always connected connection scheme, the reconnect timeout
    /// interval (in milliseconds) can optionally be selected by setting the parameter
    /// "ac-to" to the query string, "spp://0012f3000001/?ac-to=5000,2". Default
    /// value: 10000 ms.
    /// Supported by: ODIN-W2 from software version 7.1.0 onwards.
    AlwaysConnected = 0b010,
    /// External connect - Trigger connection to peer on external signal connect
    /// event. The connect event is generated when the signal SWITCH_0 in ODIN-W2 is kept low
    /// for at least 200 ms but not more than 1000 ms while the device is in the data mode.
    ExternalConnect = 0b100,
    /// Always connected - External Connect
    Both = 0b110,
}

#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum ServerType {
    Dissabled = 0,
    TCP = 1,
    UDP = 2,
    SSP = 3,
    DUN = 4,
    UUID = 5,
    SPS = 6,
    ATP = 8,
}

#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum UDPBehaviour {
    /// No connect. This will trigger an +UUDPC URC immediately (with
    /// broadcast as remote_ip and 0 as remote port); but this will not cause any new
    /// +UUDPC when the data is received. So, it will not be possible to extract the data
    /// source. This is typically used together with the data mode.
    NoConnect = 0,
    /// Auto connect.This will spawn a new peer and trigger a +UUDPC URC so that the
    /// host can respond to the sender. Further incoming data from the same source will be
    /// received on the newly created peer. The originally created server will still be active
    /// to listen for new data. This is typically used together with the Extended data mode.
    AutoConnect = 1,
}

#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum ImmediateFlush{
    Disable = 0,
    Enable = 1,
}

#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum IPVersion{
    /// Default
    IPv4 = 0,
    IPv6 = 1,
}

#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum RemoteConfiguration {
    Disable = 0,
    Enable = 1,
}

#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum WatchdogSetting {
    /// SPP (and all SPP based protocols like DUN) write timeout: <value>is the time in
    /// milliseconds before DCE disconnects if a write is not acknowledged.
    /// • 0: Disabled
    /// • > 0: Timeout in milliseconds (factory default value: 10000 ms)
    SPP = 0,
    /// Inactivity timeout: <value> is the time in milliseconds before DCE disconnects all
    /// links when no data activity in the system is detected.
    /// • 0 (factory default): Disabled
    /// • > 0: Timeout in milliseconds
    InactivityTimeout = 1,
    /// Bluetooth disconnect reset: <value> defines if the DCE shall reset on any dropped
    /// Bluetooth connection (not on an actively closed connection)
    /// • 0 (factory default): Disabled
    /// • 1: Enabled
    BluetoothDisconnectReset = 2,
    /// Wi-Fi Station disconnect reset: <value> defines if the DCE shall reset on dropped
    /// Wi-Fi Station connection (not on actively closed connection)
    /// • 0 (factory default): Disabled
    /// • 1: Enabled
    WiFiDisconnectReset = 3,
    /// Wi-Fi connect timeout: <param_val1> is the time, in seconds, that an ongoing
    /// connection attempt, for a station, will proceed before a Wi-Fi recovery is done. Note
    /// that after the recovery, the connection attempt will continue and there is no need
    /// for additional user activity. Recommended value is 30s and it should not be set lower
    /// than 20s. The default value is 0, which means that the watchdog is disabled.
    WiFiConnectTomeout = 4,
    /// Net Up timeout: <param_val1> is the time, in seconds, allowed between a +UUWLE
    /// (link connected) event and a +UUNU (net up) event. If the +UUNU is not received
    /// within the set time, the link is automatically disconnected and connected again
    /// shortly. Typically, this watchdog is set to ensure that active Bluetooth links get
    /// enough air time to avoid link loss. The watchdog is disabled by default, value 0, and
    /// an enabled recommended value is 3 seconds. Also, the link supervision time for the
    /// Bluetooth links should be increased from the default value of 2s (see the parameter
    /// tag 7 in +UBTCFG for more information).
    NetUpTimeout = 5,
}

#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum PeerConfigParameter {
    /// Keep remote peer in the command mode
    /// • 0: Disconnect peers when entering the command mode
    /// • 1 (default): Keep connections when entering the command mode
    KeepInCommandMode = 0,
    /// The module will be reset to factory default settings if it detects the following
    /// sequence on the DTR line: 1 second silence, 5 transfers from DEASSERTED to
    /// ASSERTED within 1 second, and 1 second silence.
    /// AT&D settings does not affect this.
    /// • 0: Disabled
    /// • 1 (default): Enabled
    DTRReset = 1,
    /// Number of allowed TCP links.
    /// ODIN-W2:
    /// • 1-8: Default is 2.
    AllowedTCPLinks = 2,
    /// DSR activation bit mask.
    /// Defines the condition when the DSR line is asserted. The default value for the bit
    /// mask corresponds to the previous behaviour of the &S2 AT command.
    /// • Bit 0: Activate DSR if any data peer is connected (old behavior)
    /// • Bit 1: Activate DSR if a Bluetooth LE bonded device is connected
    /// • Bit 2: Activate DSR on any Bluetooth LE GAP connection
    DSRActivationBitMask = 3,
    /// Always connected reconnect time out
    /// • 100-60000 milliseconds before trying to reconnect a default remote peer with
    /// always connected bit set (Default is 10000)
    ReconnectTimeout = 4,
    /// TCP out of sequence queue length
    /// • 0-15: Queue length for TCP packets arriving out of sequence (Default is 3). If
    /// multiple TCP links are used, this should be low.
    TCPOutOfSequenceQueue = 5,
}