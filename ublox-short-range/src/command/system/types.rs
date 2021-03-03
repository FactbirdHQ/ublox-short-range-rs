//! Argument and parameter types used by System Commands and Responses

use atat::atat_derive::AtatEnum;

#[derive(Debug, Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum Mode {
    /// Turn off
    Off = 0,
    /// Turn on
    On = 1,
}

/// DTR behavior
#[derive(Debug, Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum DTRMode {
    /// DTR line is ignored.
    Ignore = 0,
    /// (default and factory default value): Upon an ASSERTED to DEASSERTED transition
    /// of the DTR line, in data mode, the DCE enters the command mode and issues an OK
    /// result code.
    Default = 1,
    /// Upon an ASSERTED to DEASSERTED transition of the DTR line, in data mode,
    /// the DCE performs an orderly disconnect of all the Bluetooth radio links and peer
    /// connections. No new connections will be established while the DTR line remains
    /// DEASSERTED.
    DisconnectPeers = 2,
}

#[derive(Debug, Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum DSRAssertMode {
    /// ASSERT DSR
    Assert = 0,
    /// ASSERT DSR line in data mode and DEASSERT
    /// the DSR line in command mode
    DataMode = 1,
    /// ASSERT the DSR line when at least one remote peer is connected and DEASSERT
    /// DSR line when no remote peers are connected. See Connect Peer +UDCP and Default
    /// remote peer +UDDRP for definition of the remote peer. This applies to both incoming
    /// and outgoing connections.
    WhenPeersConected = 2,
}

/// Echo on
#[derive(Debug, Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum EchoOn {
    ///  Unit does not echo the characters in command mode
    Off = 0,
    /// Unit echoes the characters in command mode. (default)
    On = 1,
}

#[derive(Debug, Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum SoftwareUpdateMode {
    ///  u-connect software update using serial port
    SoftwareUpdate = 0,
}

#[derive(Debug, Clone, PartialEq, AtatEnum)]
#[at_enum(u32)]
pub enum SoftwareUpdateBaudRate {
    /// Default
    B115200 = 115200,
    B230400 = 230400,
    B460800 = 460800,
}

#[derive(Debug, Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum ModuleStartMode {
    /// Default
    CommandMode = 0,
    DataMode = 1,
    ExtendedDataMode = 2,
    PPPMode = 3,
}

#[derive(Debug, Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum InserfaceID {
    Bluetooth = 0,
    WiFi = 1,
    Ethernet = 2,
}

#[derive(Debug, Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum StatusID {
    /// The <status_val>is the uptime in seconds. That is, the seconds since last reboot
    Uptime = 0,
    /// The <status_val>is the current status of the settings
    /// • 0: Not saved. That is, there are some changes since the last stored command.
    /// • 1: Saved
    SavedStatus = 1,
}

#[derive(Debug, Clone, PartialEq, AtatEnum)]
#[at_enum(u32)]
/// ODIN-W2:
/// 19200 - 5250000. The module will set a baud rate as close as possible to the
/// requested baud rate. Recommended baud rates: 9600, 14400, 19200, 28800,
/// 38400, 57600, 76800, 115200, 230400, 250000, 460800, 921600, 3000000.
pub enum BaudRate {
    B9600 = 9600,
    B14400 = 14400,
    B19200 = 19200,
    B28800 = 28800,
    B38400 = 38400,
    B57600 = 57600,
    B76800 = 76800,
    B115200 = 115200,
    B230400 = 230400,
    B250000 = 250000,
    B460800 = 460800,
    B921600 = 921600,
    B3000000 = 3000000,
    B5250000 = 5250000,
}

#[derive(Debug, Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum FlowControl {
    /// (Default) CTS/RTS used for flow control
    On = 1,
    /// CTS/RTS not used.
    Off = 2,
}

#[derive(Debug, Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum StopBits {
    /// (Default): 1 stop bit
    One = 1,
    /// 2 stop bits
    Two = 2,
}

#[derive(Debug, Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum Parity {
    /// (Default) no parity
    None = 1,
    /// Odd parity
    Odd = 2,
    /// Even parity
    Even = 3,
}

#[derive(Debug, Clone, PartialEq, AtatEnum)]
#[repr(u8)]
/// When operating in the extended data mode, the change_after_confirm has no
/// direct effect. Settings must be stored to the profile and the module must be
/// rebooted before applying the settings.
pub enum ChangeAfterConfirm {
    /// Do not change; it must be stored and reset before applying the new setting
    StoreAndReset = 0,
    /// (Default) Change after OK. The DTE should wait at least 40 ms before sending a
    /// new command.
    ChangeAfterOK = 1,
}

#[derive(Debug, Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum PowerRegulatorSettings {
    /// Switch automatically between DC/DC and LDO regulators.
    SwitchAutomatically = 0,
    /// Disable DC/DC and use only LDO regulator.
    LDO = 1,
}

#[derive(Debug, Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum LPODetection {
    Detected = 1,
    NotDetected = 0,
}
