use core::str::FromStr;
use heapless::{consts, String, Vec};

use no_std_net::{Ipv4Addr, Ipv6Addr};

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u8)]
pub enum BTMode {
    /// Bluetooth classic
    Classic = 0,
    /// Bluetooth Low Energy
    LowEnergy = 1,
    /// Bluetooth Classic and Low Energy
    ClassicAndLowEnergy = 2,
}

/// Wi-Fi configuration id\
/// Possible values: 0-1
pub type ConfigId = u8;

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u8)]
pub enum AuthentificationType {
    Open = 1,
    WpaWpa2 = 2,
    Leap = 3,
    Peap = 4,
}

impl From<u8> for AuthentificationType {
    fn from(v: u8) -> AuthentificationType {
        match v {
            1 => AuthentificationType::Open,
            2 => AuthentificationType::WpaWpa2,
            3 => AuthentificationType::Leap,
            _ => AuthentificationType::Peap,
        }
    }
}

impl From<&str> for AuthentificationType {
    fn from(v: &str) -> AuthentificationType {
        AuthentificationType::from(v.parse::<u8>().unwrap())
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u8)]
pub enum STAAction {
    /// It clears the specified profile resetting all the parameters to their factory defaults
    Reset = 0,
    /// Validates the configuration, calculates the PSK for WPA and WPA2 (if not already calculated) and saves the configuration
    Store = 1,
    /// It reads all the parameters from memory
    Load = 2,
    /// Validates the configuration, calculates the PSK for WPA and WPA2 (if not already calculated) and
    /// activates the specified profile. It will try to connect if not connected
    Activate = 3,
    /// It deactivates the specified profile. Disconnects the profile if connected and may reconnect to other active network
    Deactivate = 4,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u8)]
pub enum Ipv4Mode {
    Static = 1,
    Dhcp = 2,
}

impl From<u8> for Ipv4Mode {
    fn from(v: u8) -> Ipv4Mode {
        match v {
            1 => Ipv4Mode::Static,
            _ => Ipv4Mode::Dhcp,
        }
    }
}

impl From<&str> for Ipv4Mode {
    fn from(v: &str) -> Ipv4Mode {
        Ipv4Mode::from(v.parse::<u8>().unwrap())
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u8)]
pub enum Ipv6Mode {
    LinkLocalIP = 1,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Copy)]
pub enum STAStatus {
    /// Currently used SSID
    SSID = 0,
    /// Currently used BSSID
    BSSID = 1,
    /// Currently used channel
    Channel = 2,
    /// Current status of the station, possible values are:\
    /// 0: Disabled\
    /// 1: Disconnected\
    /// 2: Connected
    Status = 3,
    /// RSSI value of the current connection
    RSSI = 6,
    All = 999,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Copy)]
pub enum WIFIWatchDogTypeSet {
    DisconnectReset(bool),
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u8)]
pub enum WIFIWatchDogTypeGet {
    DisconnectReset = 1,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WatchDogType {
    /// Time in milliseconds before DCE disconnects if a write isn't acknowledged.\
    /// 0: Disabled\
    /// > 0: Timeout in milliseconds (factory default value: 10000 ms)
    WriteTimeout = 0,
    /// Time in milliseconds before DCE disconnects if no activity is detected.\
    /// 0 (factory default): Disabled\
    /// > 0: Timeout in milliseconds
    InactivityTimeout = 1,

    All = 999,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum UWSCSetTag {
    /// If the station is active on startup
    ActiveOnStartup(bool),
    /// Service Set Identifier. The factory default value is an empty stri)
    SSID(String<at::MaxCommandLen>),
    /// Basic Service Set Identification (MAC_Addr of the Access Point). May be
    /// omitted. The factory default value is "000000000000"
    BSSID(String<at::MaxCommandLen>),
    /// Authentication type
    Authentication(AuthentificationType),
    /// WEP encryption keys. A WEP key is either 5 or 13 bytes and if not used it can be left empty
    WEPKeys(
        Vec<u8, consts::U13>,
        Vec<u8, consts::U13>,
        Vec<u8, consts::U13>,
        Vec<u8, consts::U13>,
    ),
    /// WEP active TX key (factory default 0 means that Open authentication
    /// with WEP encryption is disabled). Range 1-4.
    ActiveKey(u8),
    /// Passphrase (8-63 ascii characters as a string) for WPA and WPA2
    Passphrase(String<at::MaxCommandLen>),
    /// Password for LEAP, PEAP and EAP-TLS
    Password(String<at::MaxCommandLen>),
    /// Public user name, String, max length 64
    PublicUserName(String<at::MaxCommandLen>),
    /// Public domain name, String, max length 64
    PublicDomainName(String<at::MaxCommandLen>),
    /// Set the way to retreive an IP address
    Ipv4Mode(Ipv4Mode),
    /// IPv4 address. The factory default value is 0.0.0.0
    Ipv4Address(Ipv4Addr),
    /// Subnet mask. The factory default value is 0.0.0.0
    SubnetMask(Ipv4Addr),
    /// Default gateway. The factory default value is 0.0.0.0
    DefaultGateway(Ipv4Addr),
    /// Primary DNS server IP address. The factory default value is 0.0.0.0
    PrimaryDns(Ipv4Addr),
    /// Secondary DNS server IP address. The factory default value is 0.0.0.0
    SecondaryDns(Ipv4Addr),
    /// Set the way to retreive an IP address (IPv6)
    Ipv6Mode(Ipv6Mode),
    /// IPv6 link local address. If value is not set the link local address is automatically
    /// generated from the interface IEEE 48 bit MAC identifier.The factory default value is ::
    Ipv6Address(Ipv6Addr),
}

impl From<&[&str]> for UWSCSetTag {
    fn from(vals: &[&str]) -> UWSCSetTag {
        match vals[0].parse::<u8>().unwrap() {
            0 => UWSCSetTag::ActiveOnStartup(vals[1].parse::<u8>().unwrap() == 1),
            2 => UWSCSetTag::SSID(String::from(vals[1])),
            5 => UWSCSetTag::Authentication(AuthentificationType::from(vals[1])),
            // 6 => UWSCSetTag::WEPKeys(true),
            7 => UWSCSetTag::ActiveKey(vals[1].parse::<u8>().unwrap()),
            8 => UWSCSetTag::Passphrase(String::from(vals[1])),
            9 => UWSCSetTag::Password(String::from(vals[1])),
            10 => UWSCSetTag::PublicUserName(String::from(vals[1])),
            11 => UWSCSetTag::PublicDomainName(String::from(vals[1])),
            // 12 => UWSCSetTag::ClientCertificateName(true),
            // 13 => UWSCSetTag::ClientPrivateKey(true),
            // 14 => UWSCSetTag::CACertificateName(true),
            // 15 => UWSCSetTag::ValidateCACertificate(true),
            100 => UWSCSetTag::Ipv4Mode(Ipv4Mode::from(vals[1])),
            101 => UWSCSetTag::Ipv4Address(Ipv4Addr::from_str(vals[1]).unwrap()),
            102 => UWSCSetTag::SubnetMask(Ipv4Addr::from_str(vals[1]).unwrap()),
            103 => UWSCSetTag::DefaultGateway(Ipv4Addr::from_str(vals[1]).unwrap()),
            104 => UWSCSetTag::PrimaryDns(Ipv4Addr::from_str(vals[1]).unwrap()),
            105 => UWSCSetTag::SecondaryDns(Ipv4Addr::from_str(vals[1]).unwrap()),
            // 107 => UWSCSetTag::AddressConflictDetection(true),
            200 => UWSCSetTag::Ipv6Mode(Ipv6Mode::LinkLocalIP),
            201 => UWSCSetTag::Ipv6Address(Ipv6Addr::from_str(vals[1]).unwrap()),
            _ => UWSCSetTag::ActiveOnStartup(true),
            // 300 => UWSCSetTag::WifiBeaconList(true),
            // 301 => UWSCSetTag::EnableDTIM(true),
            // _ => UWSCSetTag::Reserved,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Copy)]
pub enum UWSCGetTag {
    /// If the station is active on startup
    ActiveOnStartup = 0,
    /// Service Set Identifier. The factory default value is an empty stri)
    SSID = 2,
    /// Basic Service Set Identification (MAC_Addr of the Access Point). May be
    /// omitted. The factory default value is "000000000000"
    BSSID = 3,
    /// Authentication type
    Authentication = 5,
    /// WEP active TX key (factory default 0 means that Open authentication
    /// with WEP encryption is disabled). Range 1-4.
    ActiveKey = 7,
    /// Public user name, String, max length 64
    PublicUserName = 10,
    /// Public domain name, String, max length 64
    PublicDomainName = 11,
    /// Set the way to retreive an IP address
    Ipv4Mode = 100,
    /// IPv4 address. The factory default value is 0.0.0.0
    Ipv4Address = 101,
    /// Subnet mask. The factory default value is 0.0.0.0
    SubnetMask = 102,
    /// Default gateway. The factory default value is 0.0.0.0
    DefaultGateway = 103,
    /// Primary DNS server IP address. The factory default value is 0.0.0.0
    PrimaryDns = 104,
    /// Secondary DNS server IP address. The factory default value is 0.0.0.0
    SecondaryDns = 105,
    /// Set the way to retreive an IP address (IPv6)
    Ipv6Mode = 200,
    /// IPv6 link local address. If value is not set the link local address is automatically
    /// generated from the interface IEEE 48 bit MAC identifier.The factory default value is ::
    Ipv6Address = 201,

    All = 999,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u8)]
/// Network interface type
pub enum InterfaceType {
    /// Wi-Fi Station
    WifiStation = 0,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u8)]
pub enum StatusId {
    /// Interface HW address (only shown if applicable)
    HWAddress = 0,
    /// Current status of the network interface (Layer-3)
    NetworkInterface = 1,
    /// Currently used IPv4_Addr (omitted if no IP address has been acquired)
    Ipv4Address = 101,
    /// Currently used subnet mask (omitted if no IP address has been acquired)
    SubnetMask = 102,
    /// Currently used gateway (omitted if no IP address has been acquired)
    DefaultGateway = 103,
    /// Current primary DNS server
    PrimaryDns = 104,
    /// Current secondary DNS server
    SecondaryDns = 105,
    /// Current IPv6 link local address
    Ipv6Address = 201,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u8)]
pub enum DTRValue {
    /// DTR line is ignored
    Ignore = 0,
    /// (default and factory default value): upon an ON-to-OFF transition of circuit 108/2, the DCE enters
    /// command mode and issues and OK result code
    EnterCmdMode = 1,
    /// upon an ON-to-OFF transition of circuit 108/2, the DCE performs an orderly disconnect of all radio links
    /// and peer connections. No new connections will be established while circuit 108/2 remains OFF
    DisconnectLinks = 2,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u8)]
pub enum DSRValue {
    /// sets DSR line to ON
    On = 0,
    /// (default value and factory default value): sets the DSR line to OFF in command mode and ON when not
    /// in command mode
    OffInCmdOtherwiseOn = 1,
    /// Sets the DSR line to ON in data mode when at least one remote peer is connected, all other cases it's
    /// set to off
    OnWhenConnectedPeers = 2,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u8)]
pub enum Mode {
    /// Command Mode. (factory default)
    CmdMode = 0,
    /// Data Mode
    DataMode = 1,
    /// Extended Data Mode
    ExtendedDataMode = 2,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u8)]
pub enum InterfaceId {
    /// Bluetooth
    Bluetooth = 1,
    /// WiFi
    WiFi = 2,
    /// Ethernet
    Ethernet = 3,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Copy)]
pub enum BaudRate {
    Baud2400 = 2400,
    Baud4800 = 4800,
    Baud9600 = 9600,
    Baud19200 = 19200,
    Baud38400 = 38400,
    Baud57600 = 57600,
    /// Factory default value
    Baud115200 = 115_200,
    Baud230400 = 230_400,
    Baud460800 = 460_800,
    Baud921600 = 921_600,
    Baud1360000 = 1_360_000,
    Baud2625000 = 2_625_000,
    Baud3000000 = 3_000_000,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u8)]
pub enum FlowControl {
    /// CTS/RTS used for flow control (factory default)
    Used = 1,
    /// CTS/RTS not used
    NotUsed = 2,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u8)]
pub enum StopBits {
    /// 1 stop bit (factory default)
    StopBits1 = 1,
    /// 2 stop bit
    StopBits2 = 2,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u8)]
pub enum Parity {
    /// No parity (factory default)
    NoParity = 1,
    /// Odd parity
    OddParity = 2,
    /// Even parity
    EvenParity = 3,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u8)]
pub enum ChangeAfterConfirm {
    /// Do not change, it must be stored and reset before the new setting is applied
    NoChange = 0,
    /// Change after OK. The DTE should wait at least 40 ms before sending a new command
    Change = 1,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Copy)]
pub enum PeerConfigSet {
    /// Keep remote peer in Command mode\
    /// false: Disconnect peers when entering Command mode
    /// true: Keep connections when entering Command mode (default)
    KeepPeerInCmdMode(bool),
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Copy)]
pub enum PeerConfigGet {
    /// Keep remote peer in Command mode
    KeepPeerInCmdMode = 0,
    All = 999,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u8)]
pub enum DiscoverabilityMode {
    /// GAP non-discoverable mode
    NonDiscoverable = 1,
    /// GAP limited discoverable mode
    LimitedDiscoverable = 2,
    /// GAP general discoverable mode (default)
    GeneralDiscoverable = 3,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u8)]
pub enum ConnectabilityMode {
    /// GAP non-connectable mode
    NonConnectable = 1,
    /// GAP connectable mode (default)
    Connectable = 2,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u8)]
pub enum PairingMode {
    /// GAP non-pairing mode
    NonPairing = 1,
    /// GAP pairing mode (default)
    Pairing = 2,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u8)]
/// For security modes 3, 4 and 5 the DCE must be in Command or Extended Data mode to be
/// able to do bonding because user interaction might be required
pub enum SecurityMode {
    /// Security Disabled. Should not be used in real life application.\
    /// - Auto accept (No man-in-the-middle attack protection, encryption enabled)
    Disabled = 1,
    /// Security Enabled - Just Works\
    /// - Auto accept (no man-in-the-middle attack protection, encryption enabled)\
    /// This security mode is intended for pairing in safe environments. When this mode is set,
    /// pairability (see ) is automatically disabled. In data mode, pairing can be enabled for 60 +UBTPM
    /// seconds by pressing the "External Connect" button for at least 5 seconds. When the module is
    /// pairable, the LED will blink. If the mode is changed from Just Works to another, pairability must
    /// be enabled again using the command
    JustWorks = 2,
    /// Security Enabled - Display Only*\
    /// - Service level authentication and encryption enabled. User should be presented a passkey.\
    /// This security mode is used when the device has a display that can present a 6-digit value that
    /// the user shall enter on the remote device
    DisplayOnly = 3,
    /// Security Enabled - Display Yes/No*\
    /// - Service level authentication and encryption enabled. User should compare two values.\
    /// This security mode is used when the device has a display that can present a 6-digit value that
    /// the user shall verify with yes or no to the remote device's presented value.
    /// Invalid for Bluetooth Low Energy
    DisplayYesNo = 4,
    /// Security Enabled - Keyboard Only*\
    /// - Service level authentication and encryption enabled. User should enter a passkey.\
    /// This security mode is used when the device only has a keyboard where the user can enter a
    /// 6-digit value that is presented on the remote device
    KeyboardOnly = 5,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u8)]
pub enum SecurityModeBT2_0 {
    /// Disabled, no pairing is allowed with Bluetooth 2.0 devices (factory default)
    Disabled = 0,
    /// Pairing is allowed with Bluetooth 2.0 devices using the fixed_pin
    Enabled = 1,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u8)]
/// Enabling P-256 Elliptical curve based encryption is memory intensive. Hence,
/// when enabled, the memory reserved for other functionalities will be affected.
/// For NINA-B1 and ANNA-B112, if LE role (AT+UBTLE) is Simultaneous Peripheral
/// and Central and Secure Connection is enabled, the device will not be able to
/// support more than 1 central link and 1 peripheral link (AT+UBTCFG).
pub enum SecurityType {
    /// Secure Simple Pairing mode (factory default) \
    /// The legacy mode used for pairing Bluetooth LE
    SSPM = 0,
    /// Secure Connections Mode
    /// The P-256 Elliptic curve is used for pairing and AES-CCM is used for encryption of the
    /// Bluetooth LE link. The secure simple pairing will be used if there is no support from
    /// the remote side
    SCM = 1,
    /// FIPS mode \
    /// Strictly uses Secure Connections. Pairing requests will be rejected if the remote
    /// device does not support this mode
    FIPS = 2,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u8)]
pub enum InquiryType {
    /// General extended inquiry (default)
    GeneralExtended = 1,
    /// Limited extended inquiry
    LimitedExtended = 2,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u8)]
pub enum DiscoveryType {
    /// All, display all found devices, each device will only be displayed once
    All = 1,
    /// General inquiry, display devices in General discoverablitiy mode, each device will only be
    /// displayed once (default)
    General = 2,
    /// Limited inquiry, display devices in Limited discoverability mode, each device will only be displayed
    /// once
    Limited = 3,
    /// All with no filter, display all found device. Devices can be displayed multiple times
    AllNoFilter = 4,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u8)]
pub enum DiscoveryMode {
    /// Active (default)
    Active = 1,
    /// Passive, no scan response data will be received
    Passive = 2,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u8)]
pub enum OPMode {
    /// Infrastructure
    Infrastructure = 1,
    /// Ad-hoc
    AdHoc = 2,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u8)]
pub enum BTRole {
    /// Disabled (default)
    Disabled = 0,
    /// Low Energy Central
    LowEnergyCentral = 1,
    /// Low Energy Peripheral
    LowEnergyPeripheral = 2,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u8)]
pub enum ServiceType {
    /// Serial Port Profile
    SerialPort = 0,
    /// Dial-Up Networking Profile
    DialUp = 1,
    /// SPP iPhone
    SPP = 2,
    /// UUID (Android)
    UUID = 3,
    /// Device Id
    DeviceId = 4,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u8)]
pub enum PeerProfile {
    /// Serial Port Profile
    SPP = 1,
    /// Dial-Up Networking Profile
    DUN = 2,
    /// UUID (Android)
    UUID = 3,
    /// SPP iPhone
    SPS = 4,
    /// Reservce
    Reserved = 5,
}
