//! Argument and parameter types used by WiFi Commands and Responses

use crate::command::OnOff;
use atat::atat_derive::AtatEnum;
use atat::heapless_bytes::Bytes;
use heapless::{String, Vec};
use no_std_net::{Ipv4Addr, Ipv6Addr};
use serde::Deserialize;

#[derive(Clone, PartialEq, AtatEnum)]
#[repr(u16)]
pub enum WifiStationConfigParameter {
    /// <param_val1> decides if the station is active on start up.
    /// - 0 (default): Inactive
    /// - 1: active
    ActiveOnStartup = 0,
    ///  SSID - <param_val1> is the Service Set Identifier. The factory default
    /// value is an empty string ("").
    SSID = 2,
    /// Authentication - <param_val> is the authentication type.
    /// - 1 (default): Open
    /// - 2: WPA/WPA2 PSK
    /// - 3: LEAP
    /// - 4: PEAP
    /// - 5: EAP-TLS
    Authentication = 5,
    /// WEP Keys - <param_val1>...<param_val4> are the WEP encryption keys. A
    /// WEP key is either 5 bytes (while using WEP 64), or 13 bytes (while using
    /// WEP 128) and if not used, it can be empty. The keys must be in HEX data
    /// format. For example, 010203040 5 while using WEP 64, or
    /// 0102030405060708090A0B0C0D while using WEP 128.
    ///
    /// "WEP Shared Key Authentication" is not supported; only "WEP Open Key
    /// Authentication " is supported.
    WEPKeys = 6,
    ///  Active Key - <param_val1> is the WEP active TX key (factory default 0
    /// means that Open authentication with WEP encryption is disabled). Range
    /// 1-4.
    ActiveKey = 7,
    /// PSK/Passphrase - <param_val1> is the PSK (32 HEX values) or Passphrase
    /// (8-63 ASCII characters as a string) for WPA/WPA2 PSK.
    WpaPskPassphrase = 8,
    /// Password - <param_val1> is the password for LEAP and PEAP; string with a
    /// maximum length of 31.
    EAPPassword = 9,
    /// User name - <param_val1> is the public user name for LEAP and PEAP;
    /// string with a maximum length of 31.
    UserName = 10,
    /// Domain name - <param_val1> is the public domain name for LEAP and PEAP;
    /// string with a maximum length of 63. The domain name is an optional
    /// parameter.
    DomainName = 11,
    /// Client certificate name - <param_val1> is the internal client
    /// certificate name for EAP-TLS as defined in the SSL/TLS certificates and
    /// private keys manager +USECMNG command; string with a maximum length of
    /// 32. Supported software versions 4.0.0 onwards
    ClientCertificateName = 12,
    /// Client private key - <param_val1> is the internal client private key
    /// name for EAP- TLS as defined in the SSL/TLS certificates and private
    /// keys manager +USECMNG command; string with a maximum length of 32.
    /// Supported software versions 4.0.0 onwards
    ClientPrivateKey = 13,
    /// CA certificate name - <param_val1> is the internal CA certificate name
    /// for EAP- TLS as defined in the SSL/TLS certificates and private keys
    /// manager +USECMNG command; string with a maximum length of 32. Supported
    /// software versions 5.0.0 onwards
    CACertificateName = 14,
    /// Validate CA certificate. The default value is 1; Setting this value to 0
    /// means no CA Certificate validation has been done. For example
    /// at+uwsc=0,15,0 would mean that the server CA Certificate is not
    /// validated during authentication. Supported software versions 5.0.0
    /// onwards
    ValidateCACertificate = 15,
    /// IPv4 Mode - <param_val1> to set the way to retrieve an IP address
    /// - 1: Static
    /// - 2 (default): DHCP
    IPv4Mode = 100,
    /// IPv4 address - <param_val> is the IPv4 address. The factory default
    /// value is 0.0.0.0. Valid only if param_tag 100 is set to Static.
    IPv4Address = 101,
    /// Subnet mask - <param_val> is the subnet mask. The factory default value
    /// is 0.0.0.0. Valid only if param_tag 100 is set to Static.
    SubnetMask = 102,
    /// Default gateway - <param_val> is the default gateway. The factory
    /// default value is 0.0.0.0. Valid only if param_tag 100 is set to Static.
    DefaultGateway = 103,
    /// DNS server 1 - <param_val> is the primary DNS server IP address. The
    /// factory default value is 0.0.0.0. Valid only if param_tag 100 is set to
    /// Static.
    DNSServer1 = 104,
    /// DNS server 2 - <param_val> is the primary DNS server IP address. The
    /// factory default value is 0.0.0.0. Valid only if param_tag 100 is set to
    /// Static.
    DNSServer2 = 105,
    /// Address conflict detection. The factory default value is 0 (disabled).
    /// - 0: Disabled
    /// - 1: Enabled
    AddressConflictDetection = 106,
    /// IPv6 Mode - <param_val1> to set the way to retrieve an IP address
    /// - 1 (default): Link Local IpAddress
    IPv6Mode = 200,
    /// <param_val> is the IPv6 link local address. If the value is not set, the
    /// link local address is automatically generated from the interface IEEE 48
    /// bit MAC identifier.
    IPv6LinkLocalAddress = 201,
    /// <param_val> is the Wi-Fi beacon listen interval in units of beacon
    /// interval. The factory default value is 0, listen on all beacons.
    /// - Valid values 0-16
    WiFiBeaconListenInterval = 300,
    /// <param_val> Enables DTIM in power save. If the DTIM is enabled and the
    /// module is in power save, the access point sends an indication when new
    /// data is available. If disabled, the module polls for data every beacon
    /// listen interval. The factory default value is enabled.
    /// - 0: Disabled
    /// - 1: Enabled To use WEP with open authentication, the WEP key index must
    ///   be different from zero (0).
    DTIMInPowerSave = 301,
}

#[derive(Clone, PartialEq, AtatEnum)]
pub enum WifiStationConfig {
    /// <param_val1> decides if the station is active on start up.
    /// - Off (default): Inactive
    /// - On: active
    #[at_arg(value = 0)]
    ActiveOnStartup(OnOff),
    ///  SSID - <param_val1> is the Service Set Identifier. The factory default
    /// value is an empty string ("").
    #[at_arg(value = 2)]
    SSID(String<64>),
    /// Authentication - <param_val> is the authentication type.
    /// - 1 (default): Open
    /// - 2: WPA/WPA2 PSK
    /// - 3: LEAP
    /// - 4: PEAP
    /// - 5: EAP-TLS
    #[at_arg(value = 5)]
    Authentication(Authentication),
    /// WEP Keys - <param_val1>...<param_val4> are the WEP encryption keys. A
    /// WEP key is either 5 bytes (while using WEP 64), or 13 bytes (while using
    /// WEP 128) and if not used, it can be empty. The keys must be in HEX data
    /// format. For example, 0102030405 while using WEP 64, or
    /// 0102030405060708090A0B0C0D while using WEP 128.
    ///
    /// "WEP Shared Key Authentication" is not supported; only "WEP Open Key
    /// Authentication " is supported.
    #[at_arg(value = 6)]
    WEPKeys(
        String<13>,
        Option<String<13>>,
        Option<String<13>>,
        Option<String<13>>,
        Option<String<13>>,
    ),
    ///  Active Key - <param_val1> is the WEP active TX key (factory default 0
    /// means that Open authentication with WEP encryption is disabled). Range
    /// 1-4.
    #[at_arg(value = 7)]
    ActiveKey(u8),
    /// PSK/Passphrase - <param_val1> is the PSK (32 HEX values) or Passphrase
    /// (8-63 ASCII characters as a string) for WPA/WPA2 PSK.
    #[at_arg(value = 8)]
    WpaPskOrPassphrase(String<64>),
    /// Password - <param_val1> is the password for LEAP and PEAP; string with a
    /// maximum length of 31.
    #[at_arg(value = 9)]
    EAPPassword(String<31>),
    /// User name - <param_val1> is the public user name for LEAP and PEAP;
    /// string with a maximum length of 31.
    #[at_arg(value = 10)]
    UserName(String<31>),
    /// Domain name - <param_val1> is the public domain name for LEAP and PEAP;
    /// string with a maximum length of 63. The domain name is an optional
    /// parameter.
    #[at_arg(value = 11)]
    DomainName(String<63>),
    /// Client certificate name - <param_val1> is the internal client
    /// certificate name for EAP-TLS as defined in the SSL/TLS certificates and
    /// private keys manager +USECMNG command; string with a maximum length of
    /// 32. Supported software versions 4.0.0 onwards
    #[at_arg(value = 12)]
    ClientCertificateName(String<32>),
    /// Client private key - <param_val1> is the internal client private key
    /// name for EAP- TLS as defined in the SSL/TLS certificates and private
    /// keys manager +USECMNG command; string with a maximum length of 32.
    /// Supported software versions 4.0.0 onwards
    #[at_arg(value = 13)]
    ClientPrivateKey(String<32>),
    /// CA certificate name - <param_val1> is the internal CA certificate name
    /// for EAP- TLS as defined in the SSL/TLS certificates and private keys
    /// manager +USECMNG command; string with a maximum length of 32. Supported
    /// software versions 5.0.0 onwards
    #[at_arg(value = 14)]
    CACertificateName(String<32>),
    /// Validate CA certificate. The default value is On; Setting this value to
    /// Off means no CA Certificate validation has been done. For example
    /// at+uwsc=0,15,0 would mean that the server CA Certificate is not
    /// validated during authentication. Supported software versions 5.0.0
    /// onwards
    #[at_arg(value = 15)]
    ValidateCACertificate(OnOff),
    /// IPv4 Mode - <param_val1> to set the way to retrieve an IP address
    /// - 1: Static
    /// - 2 (default): DHCP
    #[at_arg(value = 100)]
    IPv4Mode(IPv4Mode),
    /// IPv4 address - <param_val> is the IPv4 address. The factory default
    /// value is 0.0.0.0. Valid only if param_tag 100 is set to Static.
    #[at_arg(value = 101)]
    IPv4Address(#[at_arg(len = 16)] Ipv4Addr),
    /// Subnet mask - <param_val> is the subnet mask. The factory default value
    /// is 0.0.0.0. Valid only if param_tag 100 is set to Static.
    #[at_arg(value = 102)]
    SubnetMask(#[at_arg(len = 16)] Ipv4Addr),
    /// Default gateway - <param_val> is the default gateway. The factory
    /// default value is 0.0.0.0. Valid only if param_tag 100 is set to Static.
    #[at_arg(value = 103)]
    DefaultGateway(#[at_arg(len = 16)] Ipv4Addr),
    /// DNS server 1 - <param_val> is the primary DNS server IP address. The
    /// factory default value is 0.0.0.0. Valid only if param_tag 100 is set to
    /// Static.
    #[at_arg(value = 104)]
    DNSServer1(#[at_arg(len = 16)] Ipv4Addr),
    /// DNS server 2 - <param_val> is the primary DNS server IP address. The
    /// factory default value is 0.0.0.0. Valid only if param_tag 100 is set to
    /// Static.
    #[at_arg(value = 105)]
    DNSServer2(#[at_arg(len = 16)] Ipv4Addr),
    /// Address conflict detection. The factory default value is 0 (disabled).
    /// - Off: Disabled
    /// - On: Enabled
    #[at_arg(value = 106)]
    AddressConflictDetection(OnOff),
    /// IPv6 Mode - <param_val1> to set the way to retrieve an IP address
    /// - 1 (default): Link Local IpAddress
    #[at_arg(value = 200)]
    IPv6Mode(IPv6Mode),
    /// <param_val> is the IPv6 link local address. If the value is not set, the
    /// link local address is automatically generated from the interface IEEE 48
    /// bit MAC identifier.
    #[at_arg(value = 201)]
    IPv6LinkLocalAddress(#[at_arg(len = 40)] Ipv6Addr),
    /// <param_val> is the Wi-Fi beacon listen interval in units of beacon
    /// interval. The factory default value is 0, listen on all beacons.
    /// - Valid values 0-16
    #[at_arg(value = 300)]
    WiFiBeaconListenInterval(u8),
    /// <param_val> Enables DTIM in power save. If the DTIM is enabled and the
    /// module is in power save, the access point sends an indication when new
    /// data is available. If disabled, the module polls for data every beacon
    /// listen interval. The factory default value is enabled.
    /// - 0: Disabled
    /// - 1: Enabled To use WEP with open authentication, the WEP key index must
    ///   be different from zero (0).
    #[at_arg(value = 301)]
    DTIMInPowerSave(OnOff),
}

#[derive(Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum WifiStationConfigR {
    /// <param_val1> decides if the station is active on start up.
    /// - Off (default): Inactive
    /// - On: active
    #[at_arg(value = 0)]
    ActiveOnStartup(OnOff),
    ///  SSID - <param_val1> is the Service Set Identifier. The factory default
    /// value is an empty string ("").
    #[at_arg(value = 2)]
    SSID(String<64>),
    /// Authentication - <param_val> is the authentication type.
    /// - 1 (default): Open
    /// - 2: WPA/WPA2 PSK
    /// - 3: LEAP
    /// - 4: PEAP
    /// - 5: EAP-TLS
    #[at_arg(value = 5)]
    Authentication(Authentication),
    /// WEP Keys - <param_val1>...<param_val4> are the WEP encryption keys. A
    /// WEP key is either 5 bytes (while using WEP 64), or 13 bytes (while using
    /// WEP 128) and if not used, it can be empty. The keys must be in HEX data
    /// format. For example, 0102030405 while using WEP 64, or
    /// 0102030405060708090A0B0C0D while using WEP 128.
    ///
    /// "WEP Shared Key Authentication" is not supported; only "WEP Open Key
    /// Authentication " is supported.
    #[at_arg(value = 6)]
    WEPKeys(
        String<13>,
        Option<String<13>>,
        Option<String<13>>,
        Option<String<13>>,
        Option<String<13>>,
    ),
    ///  Active Key - <param_val1> is the WEP active TX key (factory default 0
    /// means that Open authentication with WEP encryption is disabled). Range
    /// 1-4.
    #[at_arg(value = 7)]
    ActiveKey(u8),
    /// PSK/Passphrase - <param_val1> is the PSK (32 HEX values) or Passphrase
    /// (8-63 ASCII characters as a string) for WPA/WPA2 PSK.
    #[at_arg(value = 8)]
    WpaPskOrPassphrase(String<64>),
    /// Password - <param_val1> is the password for LEAP and PEAP; string with a
    /// maximum length of 31.
    #[at_arg(value = 9)]
    EAPPassword(String<31>),
    /// User name - <param_val1> is the public user name for LEAP and PEAP;
    /// string with a maximum length of 31.
    #[at_arg(value = 10)]
    UserName(String<31>),
    /// Domain name - <param_val1> is the public domain name for LEAP and PEAP;
    /// string with a maximum length of 63. The domain name is an optional
    /// parameter.
    #[at_arg(value = 11)]
    DomainName(String<63>),
    /// Client certificate name - <param_val1> is the internal client
    /// certificate name for EAP-TLS as defined in the SSL/TLS certificates and
    /// private keys manager +USECMNG command; string with a maximum length of
    /// 32. Supported software versions 4.0.0 onwards
    #[at_arg(value = 12)]
    ClientCertificateName(String<32>),
    /// Client private key - <param_val1> is the internal client private key
    /// name for EAP- TLS as defined in the SSL/TLS certificates and private
    /// keys manager +USECMNG command; string with a maximum length of 32.
    /// Supported software versions 4.0.0 onwards
    #[at_arg(value = 13)]
    ClientPrivateKey(String<32>),
    /// CA certificate name - <param_val1> is the internal CA certificate name
    /// for EAP- TLS as defined in the SSL/TLS certificates and private keys
    /// manager +USECMNG command; string with a maximum length of 32. Supported
    /// software versions 5.0.0 onwards
    #[at_arg(value = 14)]
    CACertificateName(String<32>),
    /// Validate CA certificate. The default value is On; Setting this value to
    /// Off means no CA Certificate validation has been done. For example
    /// at+uwsc=0,15,0 would mean that the server CA Certificate is not
    /// validated during authentication. Supported software versions 5.0.0
    /// onwards
    #[at_arg(value = 15)]
    ValidateCACertificate(OnOff),
    /// IPv4 Mode - <param_val1> to set the way to retrieve an IP address
    /// - 1: Static
    /// - 2 (default): DHCP
    #[at_arg(value = 100)]
    IPv4Mode(IPv4Mode),
    /// IPv4 address - <param_val> is the IPv4 address. The factory default
    /// value is 0.0.0.0. Valid only if param_tag 100 is set to Static.
    #[at_arg(value = 101)]
    IPv4Address(#[at_arg(len = 16)] Ipv4Addr),
    /// Subnet mask - <param_val> is the subnet mask. The factory default value
    /// is 0.0.0.0. Valid only if param_tag 100 is set to Static.
    #[at_arg(value = 102)]
    SubnetMask(#[at_arg(len = 16)] Ipv4Addr),
    /// Default gateway - <param_val> is the default gateway. The factory
    /// default value is 0.0.0.0. Valid only if param_tag 100 is set to Static.
    #[at_arg(value = 103)]
    DefaultGateway(#[at_arg(len = 16)] Ipv4Addr),
    /// DNS server 1 - <param_val> is the primary DNS server IP address. The
    /// factory default value is 0.0.0.0. Valid only if param_tag 100 is set to
    /// Static.
    #[at_arg(value = 104)]
    DNSServer1(#[at_arg(len = 16)] Ipv4Addr),
    /// DNS server 2 - <param_val> is the primary DNS server IP address. The
    /// factory default value is 0.0.0.0. Valid only if param_tag 100 is set to
    /// Static.
    #[at_arg(value = 105)]
    DNSServer2(#[at_arg(len = 16)] Ipv4Addr),
    /// Address conflict detection. The factory default value is 0 (disabled).
    /// - Off: Disabled
    /// - On: Enabled
    #[at_arg(value = 106)]
    AddressConflictDetection(OnOff),
    /// IPv6 Mode - <param_val1> to set the way to retrieve an IP address
    /// - 1 (default): Link Local IpAddress
    #[at_arg(value = 200)]
    IPv6Mode(IPv6Mode),
    /// <param_val> is the IPv6 link local address. If the value is not set, the
    /// link local address is automatically generated from the interface IEEE 48
    /// bit MAC identifier.
    #[at_arg(value = 201)]
    IPv6LinkLocalAddress(#[at_arg(len = 40)] Ipv6Addr),
    /// <param_val> is the Wi-Fi beacon listen interval in units of beacon
    /// interval. The factory default value is 0, listen on all beacons.
    /// - Valid values 0-16
    #[at_arg(value = 300)]
    WiFiBeaconListenInterval(u8),
    /// <param_val> Enables DTIM in power save. If the DTIM is enabled and the
    /// module is in power save, the access point sends an indication when new
    /// data is available. If disabled, the module polls for data every beacon
    /// listen interval. The factory default value is enabled.
    /// - 0: Disabled
    /// - 1: Enabled To use WEP with open authentication, the WEP key index must
    ///   be different from zero (0).
    #[at_arg(value = 301)]
    DTIMInPowerSave(OnOff),
}

#[derive(Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum Authentication {
    Open = 1,
    WpaWpa2Psk = 2,
    LEAP = 3,
    PEAP = 4,
    EAPTLS = 5,
}

#[derive(Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum WifiStationAction {
    /// It clears the specified profile and resets all the parameters to their
    /// factory defaults.
    Reset = 0,
    /// Validates the configuration, calculates the PSK for WPA and WPA2 (if not
    /// already calculated),and saves the configuration.
    Store = 1,
    /// It reads all the parameters from non-volatile memory to run-time memory.
    Load = 2,
    /// Validates the configuration, calculates the PSK for WPA and WPA2 (if not
    /// already calculated), and activates the specified profile from run-time
    /// memory. It will try to connect, if not connected.
    Activate = 3,
    /// It deactivates the specified profile. Disconnects the profile if
    /// connected, and may reconnect to other active network.
    Deactivate = 4,
}

#[derive(Debug, Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum OperationMode {
    Infrastructure = 1,
    AdHoc = 2,
}

#[derive(Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum StatusId {
    SSID = 0,
    BSSID = 1,
    Channel = 2,
    /// The <status_val> is the current status of the station, possible values
    /// of status_val are:
    /// - 0: Disabled,
    /// - 1: Disconnected,
    /// - 2: Connected,
    Status = 3,
    /// The <status_val> is the RSSI value of the current connection; will
    /// return-32768, if not connected.
    Rssi = 6,
    /// The <status_val> is the mobility domain of the last or current
    /// connection This tag is supported by ODIN-W2 from software version 6.0.0
    /// onwards only.
    MobilityDomain = 7,
    /// The <status_val> is the region to which the module complies according to
    /// the accepted Wi-Fi channels:
    /// - 0: World
    /// - 1: FCC
    /// - 2: ETSI
    /// - 3: ALL (test modes only) This tag is supported by ODIN-W2 from
    ///   software version 6.0.0 onwards only.
    Region = 8,
}

#[derive(Clone, PartialEq, Deserialize)]
pub struct ScannedWifiNetwork {
    pub bssid: Bytes<20>,
    pub op_mode: OperationMode,
    pub ssid: String<64>,
    pub channel: u8,
    pub rssi: i32,
    /// Bit 0 = Shared secret Bit 1 = PSK Bit 2 = EAP Bit 3 = WPA Bit 4 = WPA2
    pub authentication_suites: u8,
    /// 1 hexadecimal value Bit 0 = WEP64 Bit 1 = WEP128 Bit 2 = TKIP Bit 3 =
    /// AES/CCMP
    pub unicast_ciphers: u8,
    /// 1 hexadecimal value Bit 0 = WEP64 Bit 1 = WEP128 Bit 2 = TKIP Bit 3 =
    /// AES/CCMP
    pub group_ciphers: u8,
}

#[derive(Clone, PartialEq, AtatEnum)]
pub enum WifiStatus {
    #[at_arg(value = 0)]
    SSID(String<64>),
    #[at_arg(value = 1)]
    BSSID(Bytes<20>),
    #[at_arg(value = 2)]
    Channel(u8),
    /// The <status_val> is the current status of the station, possible values
    /// of status_val are:
    /// - 0: Disabled,
    /// - 1: Disconnected,
    /// - 2: Connected,
    #[at_arg(value = 3)]
    Status(WifiStatusVal),
    /// The <status_val> is the RSSI value of the current connection; will
    /// return-32768, if not connected.
    #[at_arg(value = 6)]
    Rssi(u32),
    /// The <status_val> is the mobility domain of the last or current
    /// connection This tag is supported by ODIN-W2 from software version 6.0.0
    /// onwards only.
    #[at_arg(value = 7)]
    MobilityDomain(String<64>),
    /// The <status_val> is the region to which the module complies according to
    /// the accepted Wi-Fi channels: This tag is supported by ODIN-W2 from
    /// software version 6.0.0 onwards only.
    #[at_arg(value = 8)]
    Region(WifiRegion),
}

#[derive(Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum WifiStatusVal {
    Disabled = 0,
    Disconnected = 1,
    Connected = 2,
}

#[derive(Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum WifiRegion {
    World = 0,
    FCC = 1,
    ETSI = 2,
    /// Test Modes Only
    ALL = 3,
}

#[derive(Clone, PartialEq, AtatEnum)]
pub enum WifiConfigParameter {
    /// Wi-Fi enabled
    WifiEnabled = 0,
    /// Wi-Fi power save mode
    PowerSaveMode = 1,
    /// <param_val> is the transmit power level in dBm. Valid values are 0-20
    /// and 255. Adaptive transmit power level control is enabled with 255. The
    /// factory default value is
    /// 255.
    PowerLevel = 2,
    /// Good RSSI value When an AP is found with better or equal RSSI, the
    /// module will abort the scanning and connect to the AP. Valid values are
    /// -128 to 0. The default value is -55.
    GoodRSSIValue = 5,
    ///  Bad RSSI value This value is defined when you are in an area with bad
    /// coverage. That is, the fast scan sleep timeout (param_tag 8) will be
    /// used to find a better alternative. Valid values are - 128 to 0. The
    /// default value is -70. Supported software versions 5.0.0 onwards
    BadRSSIValue = 6,
    /// Slow scan sleep timeout <param_val> is the timeout in ms for scanning
    /// two channels when the module is connected to an AP with an RSSI value
    /// that is above the Bad RSSI value (param_tag 6). Set to 0 to turn the
    /// neighborhood watch off when there is a good signal strength. Valid
    /// values are 0 - 2147483647. The default value is 2000. Supported software
    /// versions 5.0.0 onwards
    SlowScanSleepTimeout = 7,
    /// Fast scan sleep timeout <param_val> is the timeout in ms for scanning
    /// two channels when the module is connected to an AP with an RSSI value
    /// that is below the Bad RSSI value (param_tag 6). Set to 0 to turn off
    /// roaming. Valid values are 0 - 2147483647. The default value is
    /// 150.
    /// Supported software versions 5.0.0 onwards
    FastScanSleepTimeout = 8,
    /// Last BSSID block time <param_val> is the time in seconds a switch to the
    /// last connected AP is blocked. Valid values are 0 - 2147483. The default
    /// value is 5. Supported software versions 5.0.0 onwards
    LastBSSIDBlockTime = 9,
    /// Drop network on link loss
    /// - Off (default): Do not drop the network when there is a Wi-Fi link loss
    /// - On: Drop the network when the Wi-Fi link is lost; data may be lost
    ///   with this option. Supported software versions 5.0.0 onwards
    DropNetworkOnLinkLoss = 10,
    /// Force world mode
    /// - Off: Use all channels in the channel list; See +UWCL for more
    ///      information. The channel list will be filtered by 802.11d.
    /// - On (default): Lock device to world mode. The channel list (+UWCL) is
    ///     filtered and only the channels in the following ranges will be used
    ///     - 1-11, 36-64, 100-116, 132-140. For the updated "Force world mode"
    ///     settings to take affect, the Wi-Fi radio must be restarted. This can
    ///     be done by the Wi-Fi disable/enable command (parameter
    ///     0) or by storing the setting (&W) to non-volatile memory and
    ///        restarting the module. Supported software versions 5.0.0 onwards
    ForceWorldMode = 11,
    /// Fast transition mode (802.11r) Supported software versions 6.0.0 onwards
    FastTransitionMode = 12,
    /// Scan listen interval <param_val> is the timeout (in ms) between scanning
    /// one channel and another. The default value is 0 ms. Supported software
    /// versions 6.0.0 onwards
    ScanListenInterval = 14,
    /// Remain on channel
    /// - On (default): Enable remain on channel
    /// - Off: Disable remain on channel Supported software versions 6.0.0
    ///   onwards
    RemainOnChannel = 15,
    /// Station TX rates bit mask where bit masks are defined according to:
    /// - 0x00000001: Rate 1 Mbps
    /// - 0x00000002: Rate 2 Mbps
    /// - 0x00000004: Rate 5.5 Mbps
    /// - 0x00000008: Rate 11 Mbps
    /// - 0x00000010: Rate 6 Mbps
    /// - 0x00000020: Rate 9 Mbps
    /// - 0x00000040: Rate 12 Mbps
    /// - 0x00000080: Rate 18 Mbps
    /// - 0x00000100: Rate 24 Mbps
    /// - 0x00000200: Rate 36 Mbps
    /// - 0x00000400: Rate 48 Mbps
    /// - 0x00000800: Rate 54 Mbps
    /// - 0x00001000: Rate MCS 0
    /// - 0x00002000: Rate MCS 1
    /// - 0x00004000: Rate MCS 2
    /// - 0x00008000: Rate MCS 3
    /// - 0x00010000: Rate MCS 4
    /// - 0x00020000: Rate MCS 5
    /// - 0x00040000: Rate MCS 6
    /// - 0x00080000: Rate MCS 7
    /// - 0x00100000: Rate MCS 8
    /// - 0x00200000: Rate MCS 9
    /// - 0x00400000: Rate MCS 10
    /// - 0x00800000: Rate MCS 11
    /// - 0x01000000: Rate MCS 12
    /// - 0x02000000: Rate MCS 13
    /// - 0x04000000: Rate MCS 14
    /// - 0x08000000: Rate MCS 15 Default value is 0, which means that all rates
    /// are enabled. Supported software versions 7.0.0 onwards
    StationTxRates = 16,
    /// Station short packet retry limit. Default value is 0x00141414. The
    /// definition of retry limits are listed below:
    /// - Bits 31-24: Reserved
    /// - Bits 23-16: EAPOL & Broadcast (0x01-0xFF)
    /// - Bits 15-8: MGMT (0x01-0xFF)
    /// - Bits 7-0: Data (0x01-0xFF) Supported software versions 7.0.0 onwards
    StationShortPacketRetryLimit = 17,
    /// Station long packet retry limit. Default value is 0x00141414. The
    /// definition of retry limits are listed below:
    /// - Bits 31-24: Reserved
    /// - Bits 23-16: EAPOL & Broadcast (0x01-0xFF)
    /// - Bits 15-8: MGMT (0x01-0xFF)
    /// - Bits 7-0: Data (0x01-0xFF) Supported software versions 7.0.0 onwards
    StationLongPacketRetryLimit = 18,
    /// AP short packet retry limit. Default value is 0x00141414. The definition
    /// of retry limits are listed below:
    /// - Bits 31-24: Reserved
    /// - Bits 23-16: EAPOL & Broadcast (0x01-0xFF)
    /// - Bits 15-8: MGMT (0x01-0xFF)
    /// - Bits 7-0: Data (0x01-0xFF) Supported software versions 7.0.0 onwards
    APShortPacketRetryLimit = 19,
    /// AP long packet retry limit. Default value is 0x00141414. The definition
    /// of retry limits are listed below:
    /// - Bits 31-24: Reserved
    /// - Bits 23-16: EAPOL & Broadcast (0x01-0xFF)
    /// - Bits 15-8: MGMT (0x01-0xFF)
    /// - Bits 7-0: Data (0x01-0xFF) Supported software versions 7.0.0 onwards
    APLongPacketRetryLimit = 20,
    ///  Scan Type
    /// - 1 (default): Active scan
    /// - 2: Passive scan Supported software versions 7.0.0 onwards
    ScanType = 21,
    /// Scan Filter
    /// - Off (default): Do not filter scan results
    /// - On: Filter scan results; the module will try to only send one scan
    ///     response for each BSSID. In environments with a high number of
    ///     networks, this may not work. Supported software versions 7.0.0
    ///     onwards
    ScanFilter = 22,
    /// Enable block acknowledgement
    /// - Off (default): Disable block acknowledgement
    /// - On: Enable block acknowledgement Supported software versions 7.0.2
    ///   onwards
    BlockAcknowledgment = 23,
    /// Minimum TLS version. Default: TLS v1.0 Supported software versions 7.0.2
    /// onwards
    MinimumTlsVersion = 24,
    /// Maximum TLS version. Default: TLS v1.2 Supported software versions 7.0.2
    /// onwards
    MaximumTlsVersion = 25,
}

#[derive(Clone, PartialEq, AtatEnum)]
pub enum WifiConfig {
    /// Wi-Fi enabled
    #[at_arg(value = 0)]
    WifiEnabled(WifiMode),
    /// Wi-Fi power save mode
    #[at_arg(value = 1)]
    PowerSaveMode(PowerSaveMode),
    /// <param_val> is the transmit power level in dBm. Valid values are 0-20
    /// and 255. Adaptive transmit power level control is enabled with 255. The
    /// factory default value is
    /// 255.
    #[at_arg(value = 2)]
    PowerLevel(u8),
    /// Good RSSI value When an AP is found with better or equal RSSI, the
    /// module will abort the scanning and connect to the AP. Valid values are
    /// -128 to 0. The default value is -55.
    #[at_arg(value = 5)]
    GoodRSSIValue(i32),
    ///  Bad RSSI value This value is defined when you are in an area with bad
    /// coverage. That is, the fast scan sleep timeout (param_tag 8) will be
    /// used to find a better alternative. Valid values are - 128 to 0. The
    /// default value is -70. Supported software versions 5.0.0 onwards
    #[at_arg(value = 6)]
    BadRSSIValue(i32),
    /// Slow scan sleep timeout <param_val> is the timeout in ms for scanning
    /// two channels when the module is connected to an AP with an RSSI value
    /// that is above the Bad RSSI value (param_tag 6). Set to 0 to turn the
    /// neighborhood watch off when there is a good signal strength. Valid
    /// values are 0 - 2147483647. The default value is 2000. Supported software
    /// versions 5.0.0 onwards
    #[at_arg(value = 7)]
    SlowScanSleepTimeout(u32),
    /// Fast scan sleep timeout <param_val> is the timeout in ms for scanning
    /// two channels when the module is connected to an AP with an RSSI value
    /// that is below the Bad RSSI value (param_tag 6). Set to 0 to turn off
    /// roaming. Valid values are 0 - 2147483647. The default value is
    /// 150.
    /// Supported software versions 5.0.0 onwards
    #[at_arg(value = 8)]
    FastScanSleepTimeout(u32),
    /// Last BSSID block time <param_val> is the time in seconds a switch to the
    /// last connected AP is blocked. Valid values are 0 - 2147483. The default
    /// value is 5. Supported software versions 5.0.0 onwards
    #[at_arg(value = 9)]
    LastBSSIDBlockTime(u32),
    /// Drop network on link loss
    /// - Off (default): Do not drop the network when there is a Wi-Fi link loss
    /// - On: Drop the network when the Wi-Fi link is lost; data may be lost
    ///   with this option. Supported software versions 5.0.0 onwards
    #[at_arg(value = 10)]
    DropNetworkOnLinkLoss(OnOff),
    /// Force world mode
    /// - Off: Use all channels in the channel list; See +UWCL for more
    ///      information. The channel list will be filtered by 802.11d.
    /// - On (default): Lock device to world mode. The channel list (+UWCL) is
    ///     filtered and only the channels in the following ranges will be used
    ///     - 1-11, 36-64, 100-116, 132-140. For the updated "Force world mode"
    ///     settings to take affect, the Wi-Fi radio must be restarted. This can
    ///     be done by the Wi-Fi disable/enable command (parameter
    ///     0) or by storing the setting (&W) to non-volatile memory and
    ///        restarting the module. Supported software versions 5.0.0 onwards
    #[at_arg(value = 11)]
    ForceWorldMode(OnOff),
    /// Fast transition mode (802.11r) Supported software versions 6.0.0 onwards
    #[at_arg(value = 12)]
    FastTransitionMode(FastTransitionMode),
    /// Scan listen interval <param_val> is the timeout (in ms) between scanning
    /// one channel and another. The default value is 0 ms. Supported software
    /// versions 6.0.0 onwards
    #[at_arg(value = 14)]
    ScanListenInterval(u32),
    /// Remain on channel
    /// - On (default): Enable remain on channel
    /// - Off: Disable remain on channel Supported software versions 6.0.0
    ///   onwards
    #[at_arg(value = 15)]
    RemainOnChannel(u8),
    /// Station TX rates bit mask where bit masks are defined according to:
    /// 0x00000001: Rate 1 Mbps 0x00000002: Rate 2 Mbps 0x00000004: Rate 5.5
    /// Mbps 0x00000008: Rate 11 Mbps 0x00000010: Rate 6 Mbps 0x00000020: Rate 9
    /// Mbps 0x00000040: Rate 12 Mbps 0x00000080: Rate 18 Mbps 0x00000100: Rate
    /// 24 Mbps 0x00000200: Rate 36 Mbps 0x00000400: Rate 48 Mbps 0x00000800:
    /// Rate 54 Mbps 0x00001000: Rate MCS 0 0x00002000: Rate MCS 1 0x00004000:
    /// Rate MCS 2 0x00008000: Rate MCS 3 0x00010000: Rate MCS 4 0x00020000:
    /// Rate MCS 5 0x00040000: Rate MCS 6 0x00080000: Rate MCS 7 0x00100000:
    /// Rate MCS 8 0x00200000: Rate MCS 9 0x00400000: Rate MCS 10 0x00800000:
    /// Rate MCS 11 0x01000000: Rate MCS 12 0x02000000: Rate MCS 13 0x04000000:
    /// Rate MCS 14 0x08000000: Rate MCS 15 Default value is 0, which means that
    /// all rates are enabled. Supported software versions 7.0.0 onwards
    #[at_arg(value = 16)]
    StationTxRates(u32),
    /// Station short packet retry limit. Default value is 0x00141414. The
    /// definition of retry limits are listed below:
    /// - Bits 31-24: Reserved
    /// - Bits 23-16: EAPOL & Broadcast (0x01-0xFF)
    /// - Bits 15-8: MGMT (0x01-0xFF)
    /// - Bits 7-0: Data (0x01-0xFF) Supported software versions 7.0.0 onwards
    #[at_arg(value = 17)]
    StationShortPacketRetryLimit(u32),
    /// Station long packet retry limit. Default value is 0x00141414. The
    /// definition of retry limits are listed below:
    /// - Bits 31-24: Reserved
    /// - Bits 23-16: EAPOL & Broadcast (0x01-0xFF)
    /// - Bits 15-8: MGMT (0x01-0xFF)
    /// - Bits 7-0: Data (0x01-0xFF) Supported software versions 7.0.0 onwards
    #[at_arg(value = 18)]
    StationLongPacketRetryLimit(u32),
    /// AP short packet retry limit. Default value is 0x00141414. The definition
    /// of retry limits are listed below:
    /// - Bits 31-24: Reserved
    /// - Bits 23-16: EAPOL & Broadcast (0x01-0xFF)
    /// - Bits 15-8: MGMT (0x01-0xFF)
    /// - Bits 7-0: Data (0x01-0xFF) Supported software versions 7.0.0 onwards
    #[at_arg(value = 19)]
    APShortPacketRetryLimit(u32),
    /// AP long packet retry limit. Default value is 0x00141414. The definition
    /// of retry limits are listed below:
    /// - Bits 31-24: Reserved
    /// - Bits 23-16: EAPOL & Broadcast (0x01-0xFF)
    /// - Bits 15-8: MGMT (0x01-0xFF)
    /// - Bits 7-0: Data (0x01-0xFF) Supported software versions 7.0.0 onwards
    #[at_arg(value = 20)]
    APLongPacketRetryLimit(u32),
    ///  Scan Type
    /// - 1 (default): Active scan
    /// - 2: Passive scan Supported software versions 7.0.0 onwards
    #[at_arg(value = 21)]
    ScanType(ScanType),
    /// Scan Filter
    /// - Off (default): Do not filter scan results
    /// - On: Filter scan results; the module will try to only send one scan
    ///     response for each BSSID. In environments with a high number of
    ///     networks, this may not work. Supported software versions 7.0.0
    ///     onwards
    #[at_arg(value = 22)]
    ScanFilter(OnOff),
    /// Enable block acknowledgement
    /// - Off (default): Disable block acknowledgement
    /// - On: Enable block acknowledgement Supported software versions 7.0.2
    ///   onwards
    #[at_arg(value = 23)]
    BlockAcknowledgment(OnOff),
    /// Minimum TLS version. Default: TLS v1.0 Supported software versions 7.0.2
    /// onwards
    #[at_arg(value = 24)]
    MinimumTlsVersion(TLSVersion),
    /// Maximum TLS version. Default: TLS v1.2 Supported software versions 7.0.2
    /// onwards
    #[at_arg(value = 25)]
    MaximumTlsVersion(TLSVersion),
}

#[derive(Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum WifiMode {
    Disable = 0,
    Enabled = 1,
    Auto = 2,
}

#[derive(Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum PowerSaveMode {
    ActiveMode = 0,
    SleepMode = 1,
    /// Default
    DeepSleepMode = 2,
}

#[derive(Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum FastTransitionMode {
    /// - 0: Disabled, never use fast transitions.
    Disabled = 0,
    /// - 1: Over air, use fast transitions "Over air" instead of "Over DS",
    ///      even though "Over DS" support is announced by the APs.
    OverAir = 1,
    /// - 2 (default): Over DS, follow the mode announced by the APs
    OverDS = 2,
}

#[derive(Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum ScanType {
    /// Default
    ActiveScan = 1,
    PassiveScan = 0,
}

#[derive(Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum TLSVersion {
    TLSv1_0 = 1,
    TLSv1_1 = 2,
    TLSv1_2 = 3,
}

#[derive(Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum WatchdogSetting {
    DisconnectReset = 0,
}

#[derive(Clone, Copy, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum AccessPointId {
    Id0 = 0,
}

#[derive(Clone, PartialEq, AtatEnum)]
pub enum AccessPointConfig {
    /// <param_val1> decides if the access point is active on start up.
    /// - 0 (default): Inactive
    /// - 1: active
    #[at_arg(value = 0)]
    ActiveOnStartup(OnOff),
    /// SSID - <param_val1> is the Service Set identification of the access
    /// point. The factory-programmed value is ("UBXWifi").
    #[at_arg(value = 2)]
    SSID(String<64>),
    /// <param_val1> is the channel. Factory programmed value is 6.
    #[at_arg(value = 4)]
    Channel(u8),
    /// Security mode <param_val1>:
    /// - 1: Open
    /// - 2 (default): WPA2 (AES-CCMP)
    /// - 3: WPA/WPA2 Mixed mode (RC4-TKIP + AES-CCMP)
    /// - 4: WPA (RC4-TKIP)
    ///
    /// <param_val2>:
    /// - 1: Open
    /// - 2 (default): Pre shared key PSK
    #[at_arg(value = 5)]
    SecurityMode(SecurityMode, SecurityModePSK),
    /// PSK/Passphrase - <param_val1> is the PSK (32 HEX values) or Passphrase
    /// (8-63 ascii characters as a string) for WPA and WPA2, default:
    /// "ubx-wifi". This tag does not support reading.
    #[at_arg(value = 8)]
    PSKPassphrase(PasskeyR),
    /// <param_val1> is a bitmask representing the mandatory 802.11b rates.
    /// - Bit 0 (default): 1 Mbit/s
    /// - Bit 1: 2 Mbit/s
    /// - Bit 2: 5.5 Mbit/s
    /// - Bit 3: 11 Mbit/s
    #[at_arg(value = 12)]
    Rates802_11b(u8),
    /// <param_val1> is a bitmask representing the mandatory 802.11ag rates.
    /// - Bit 0 (default): 6 Mbit/s
    /// - Bit 1: 9
    /// - Bit 2: 12 Mbit/s
    /// - Bit 3: 18 Mbit/s
    /// - Bit 4: 24 Mbit/s
    /// - Bit 5: 36 Mbit/s
    /// - Bit 6: 48 Mbit/s
    /// - Bit 7: 54 Mbit/s
    #[at_arg(value = 13)]
    Rates802_11ag(u8),
    /// <param_val1> Protected Management Frames (PMF) Supported software
    /// versions 6.0.0 onwards
    #[at_arg(value = 14)]
    ProtectedManagementFrames(PMF),
    /// <param_val1> Access point supported rates bit mask where the bit masks
    /// are defined according to:
    /// - 0x00000001: Rate 1 Mbit/s
    /// - 0x00000002: Rate 2 Mbit/s
    /// - 0x00000004: Rate 5.5 Mbit/s
    /// - 0x00000008: Rate 11 Mbit/s
    /// - 0x00000010: Rate 6 Mbit/s
    /// - 0x00000020: Rate 9 Mbit/s
    /// - 0x00000040: Rate 12 Mbit/s
    /// - 0x00000080: Rate 18 Mbit/s
    /// - 0x00000100: Rate 24 Mbit/s
    /// - 0x00000200: Rate 36 Mbit/s
    /// - 0x00000400: Rate 48 Mbit/s
    /// - 0x00000800: Rate 54 Mbit/s
    /// - 0x00001000: Rate MCS 0
    /// - 0x00002000: Rate MCS 1
    /// - 0x00004000: Rate MCS 2
    /// - 0x00008000: Rate MCS 3
    /// - 0x00010000: Rate MCS 4
    /// - 0x00020000: Rate MCS 5
    /// - 0x00040000: Rate MCS 6
    /// - 0x00080000: Rate MCS 7
    /// - 0x00100000: Rate MCS 8
    /// - 0x00200000: Rate MCS 9
    /// - 0x00400000: Rate MCS 10
    /// - 0x00800000: Rate MCS 11
    /// - 0x01000000: Rate MCS 12
    /// - 0x02000000: Rate MCS 13
    /// - 0x04000000: Rate MCS 14
    /// - 0x08000000: Rate MCS 15 The default value is 0, which means that all
    ///   rates are enabled. Supported software versions 6.0.0 onwards
    #[at_arg(value = 15)]
    APRates(u32),
    /// <param_val1> Hidden SSID configuration.
    /// - Bit 0 (default): Disable hidden SSID
    /// - Bit 1: Enable hidden SSID Supported software versions 6.0.0 onwards
    #[at_arg(value = 16)]
    HiddenSSID(OnOff),
    /// White List - <param_val1>...<param_val10> List of MAC addresses of
    /// stations that is allowed to connect or 0 to allow all. The factory
    /// default is 0.
    #[at_arg(value = 19)]
    WhiteList(String<20>, String<20>, String<20>),
    /// Black List - <param_val1>...<param_val10> List of MAC addresses of
    /// stations that will be rejected or 0 to not reject any. The factory
    /// default is 0.
    #[at_arg(value = 20)]
    BlackList(String<20>, String<20>, String<20>),
    /// IPv4 Mode - <param_val1> to set the way to retrieve an IP address
    /// - 1:(default) Static
    #[at_arg(value = 100)]
    IPv4Mode(IPv4Mode),
    /// <param_val> is the IPv4 address. The factory default value is
    /// 192.168.2.1
    #[at_arg(value = 101)]
    IPv4Address(#[at_arg(len = 16)] Ipv4Addr),
    /// <param_val> is the subnet mask. The factory default value is
    /// 255.255.255.0
    #[at_arg(value = 102)]
    SubnetMask(#[at_arg(len = 16)] Ipv4Addr),
    /// <param_val> is the default gateway. The factory default value is
    /// 192.168.2.1
    #[at_arg(value = 103)]
    DefaultGateway(#[at_arg(len = 16)] Ipv4Addr),
    /// <param_val> is the primary DNS server IP address. The factory default
    /// value is 0.0.0.0
    #[at_arg(value = 104)]
    PrimaryDNS(#[at_arg(len = 16)] Ipv4Addr),
    /// <param_val> is the secondary DNS server IP address. The factory default
    /// value is 0.0.0.0
    #[at_arg(value = 105)]
    SecondaryDNS(#[at_arg(len = 16)] Ipv4Addr),
    /// <param_val> is the DHCP server configuration.
    /// - 0 (default): Disable DHCP server
    /// - 1 Enable DHCP server. The DHCP Server will provide addresses according
    ///   to the following formula: (Static address and subnet mask) + 100
    #[at_arg(value = 106)]
    DHCPServer(OnOff),
    /// Address conflict detection. The factory default value is 0 (disabled).
    /// - 0: Disabled
    /// - 1: Enabled Supported software versions 6.0.0 onwards
    #[at_arg(value = 107)]
    AddressConflictDetection(OnOff),
    ///  IPv6 Mode - <param_val> to set the way to retrieve an IP address
    /// - 1 (default): Link Local IP address
    #[at_arg(value = 200)]
    IPv6Mode(IPv6Mode),
    /// <param_val> is the IPv6 link local address. If the value is not set, the
    /// link local address is automatically generated from the interface IEEE 48
    /// bit MAC identifier. The factory default value is:
    #[at_arg(value = 201)]
    IPv6LinkLocalAddress(#[at_arg(len = 40)] Ipv6Addr),
    /// <param_val> is the DTIM interval. The factory default value is 1. Valid
    /// values are 1 to 100.
    #[at_arg(value = 300)]
    DTIM(u8),
}

#[derive(Clone, PartialEq, AtatEnum)]
#[repr(u16)]
pub enum AccessPointConfigParameter {
    /// <param_val1> decides if the access point is active on start up.
    /// - 0 (default): Inactive
    /// - 1: active
    ActiveOnStartup = 0,
    /// SSID - <param_val1> is the Service Set identification of the access
    /// point. The factory-programmed value is ("UBXWifi").
    SSID = 2,
    /// <param_val1> is the channel. Factory programmed value is 6.
    Channel = 4,
    /// Security mode <param_val1>:
    /// - 1: Open
    /// - 2 (default): WPA2 (AES-CCMP)
    /// - 3: WPA/WPA2 Mixed mode (RC4-TKIP + AES-CCMP)
    /// - 4: WPA (RC4-TKIP) <param_val2>:
    /// - 1: Open
    /// - 2 (default): Pre shared key PSK
    SecurityMode = 5,
    /// PSK/Passphrase - <param_val1> is the PSK (32 HEX values) or Passphrase
    /// (8-63 ascii characters as a string) for WPA and WPA2, default:
    /// "ubx-wifi". This tag does not support reading.
    PSKPassphrase = 8,
    /// <param_val1> is a bitmask representing the mandatory 802.11b rates.
    /// - Bit 0 (default): 1 Mbit/s
    /// - Bit 1: 2 Mbit/s
    /// - Bit 2: 5.5 Mbit/s
    /// - Bit 3: 11 Mbit/s
    Rates802_11b = 12,
    /// <param_val1> is a bitmask representing the mandatory 802.11ag rates.
    /// - Bit 0 (default): 6 Mbit/s
    /// - Bit 1: 9
    /// - Bit 2: 12 Mbit/s
    /// - Bit 3: 18 Mbit/s
    /// - Bit 4: 24 Mbit/s
    /// - Bit 5: 36 Mbit/s
    /// - Bit 6: 48 Mbit/s
    /// - Bit 7: 54 Mbit/s
    Rates802_11ag = 13,
    /// <param_val1> Protected Management Frames (PMF) Supported software
    /// versions 6.0.0 onwards
    ProtectedManagementFrames = 14,
    /// <param_val1> Access point supported rates bit mask where the bit masks
    /// are defined according to:
    /// - 0x00000001: Rate 1 Mbit/s
    /// - 0x00000002: Rate 2 Mbit/s
    /// - 0x00000004: Rate 5.5 Mbit/s
    /// - 0x00000008: Rate 11 Mbit/s
    /// - 0x00000010: Rate 6 Mbit/s
    /// - 0x00000020: Rate 9 Mbit/s
    /// - 0x00000040: Rate 12 Mbit/s
    /// - 0x00000080: Rate 18 Mbit/s
    /// - 0x00000100: Rate 24 Mbit/s
    /// - 0x00000200: Rate 36 Mbit/s
    /// - 0x00000400: Rate 48 Mbit/s
    /// - 0x00000800: Rate 54 Mbit/s
    /// - 0x00001000: Rate MCS 0
    /// - 0x00002000: Rate MCS 1
    /// - 0x00004000: Rate MCS 2
    /// - 0x00008000: Rate MCS 3
    /// - 0x00010000: Rate MCS 4
    /// - 0x00020000: Rate MCS 5
    /// - 0x00040000: Rate MCS 6
    /// - 0x00080000: Rate MCS 7
    /// - 0x00100000: Rate MCS 8
    /// - 0x00200000: Rate MCS 9
    /// - 0x00400000: Rate MCS 10
    /// - 0x00800000: Rate MCS 11
    /// - 0x01000000: Rate MCS 12
    /// - 0x02000000: Rate MCS 13
    /// - 0x04000000: Rate MCS 14
    /// - 0x08000000: Rate MCS 15 The default value is 0, which means that all
    ///   rates are enabled. Supported software versions 6.0.0 onwards
    APRates = 15,
    /// <param_val1> Hidden SSID configuration.
    /// - Bit 0 (default): Disable hidden SSID
    /// - Bit 1: Enable hidden SSID Supported software versions 6.0.0 onwards
    HiddenSSID = 16,
    /// White List - <param_val1>...<param_val10> List of MAC addresses of
    /// stations that is allowed to connect or 0 to allow all. The factory
    /// default is 0.
    WhiteList = 19,
    /// Black List - <param_val1>...<param_val10> List of MAC addresses of
    /// stations that will be rejected or 0 to not reject any. The factory
    /// default is 0.
    BlackList = 20,
    /// IPv4 Mode - <param_val1> to set the way to retrieve an IP address
    /// - 1:(default) Static
    IPv4Mode = 100,
    /// <param_val> is the IPv4 address. The factory default value is
    /// 192.168.2.1
    IPv4Address = 101,
    /// <param_val> is the subnet mask. The factory default value is
    /// 255.255.255.0
    SubnetMask = 102,
    /// <param_val> is the default gateway. The factory default value is
    /// 192.168.2.1
    DefaultGateway = 103,
    /// <param_val> is the primary DNS server IP address. The factory default
    /// value is 0.0.0.0
    PrimaryDNS = 104,
    /// <param_val> is the secondary DNS server IP address. The factory default
    /// value is 0.0.0.0
    SecondaryDNS = 105,
    /// <param_val> is the DHCP server configuration.
    /// - 0 (default): Disable DHCP server
    /// - 1 Enable DHCP server. The DHCP Server will provide addresses according
    ///   to the following formula: (Static address and subnet mask) + 100
    DHCPServer = 106,
    /// Address conflict detection. The factory default value is 0 (disabled).
    /// - 0: Disabled
    /// - 1: Enabled Supported software versions 6.0.0 onwards
    AddressConflictDetection = 107,
    ///  IPv6 Mode - <param_val> to set the way to retrieve an IP address
    /// - 1 (default): Link Local IP address
    IPv6Mode = 200,
    /// <param_val> is the IPv6 link local address. If the value is not set, the
    /// link local address is automatically generated from the interface IEEE 48
    /// bit MAC identifier. The factory default value is:
    IPv6LinkLocalAddress = 201,
    /// <param_val> is the DTIM interval. The factory default value is 1. Valid
    /// values are 1 to 100.
    DTIM = 301,
}

#[derive(Clone, PartialEq, AtatEnum)]
#[repr(u16)]
pub enum AccessPointConfigResponse {
    /// <param_val1> decides if the access point is active on start up.
    /// - 0 (default): Inactive
    /// - 1: active
    #[at_arg(value = 0)]
    ActiveOnStartup(OnOff),
    /// SSID - <param_val1> is the Service Set identification of the access
    /// point. The factory-programmed value is ("UBXWifi").
    #[at_arg(value = 2)]
    SSID(String<64>),
    /// <param_val1> is the channel. Factory programmed value is 6.
    #[at_arg(value = 4)]
    Channel(u8),
    /// Security mode <param_val1>:
    /// - 1: Open
    /// - 2 (default): WPA2 (AES-CCMP)
    /// - 3: WPA/WPA2 Mixed mode (RC4-TKIP + AES-CCMP)
    /// - 4: WPA (RC4-TKIP) <param_val2>:
    /// - 1: Open
    /// - 2 (default): Pre shared key PSK
    #[at_arg(value = 5)]
    SecurityMode(SecurityMode, SecurityModePSK),
    /// PSK/Passphrase - <param_val1> is the PSK (32 HEX values) or Passphrase
    /// (8-63 ascii characters as a string) for WPA and WPA2, default:
    /// "ubx-wifi". This tag does not support reading.
    #[at_arg(value = 8)]
    PSKPassphrase(PasskeyR),
    /// <param_val1> is a bitmask representing the mandatory 802.11b rates.
    /// - Bit 0 (default): 1 Mbit/s
    /// - Bit 1: 2 Mbit/s
    /// - Bit 2: 5.5 Mbit/s
    /// - Bit 3: 11 Mbit/s
    #[at_arg(value = 12)]
    Rates802_11b(u8),
    /// <param_val1> is a bitmask representing the mandatory 802.11ag rates.
    /// - Bit 0 (default): 6 Mbit/s
    /// - Bit 1: 9
    /// - Bit 2: 12 Mbit/s
    /// - Bit 3: 18 Mbit/s
    /// - Bit 4: 24 Mbit/s
    /// - Bit 5: 36 Mbit/s
    /// - Bit 6: 48 Mbit/s
    /// - Bit 7: 54 Mbit/s
    #[at_arg(value = 13)]
    Rates802_11ag(u8),
    /// <param_val1> Protected Management Frames (PMF) Supported software
    /// versions 6.0.0 onwards
    #[at_arg(value = 14)]
    ProtectedManagementFrames(PMF),
    /// <param_val1> Access point supported rates bit mask where the bit masks
    /// are defined according to:
    /// - 0x00000001: Rate 1 Mbit/s
    /// - 0x00000002: Rate 2 Mbit/s
    /// - 0x00000004: Rate 5.5 Mbit/s
    /// - 0x00000008: Rate 11 Mbit/s
    /// - 0x00000010: Rate 6 Mbit/s
    /// - 0x00000020: Rate 9 Mbit/s
    /// - 0x00000040: Rate 12 Mbit/s
    /// - 0x00000080: Rate 18 Mbit/s
    /// - 0x00000100: Rate 24 Mbit/s
    /// - 0x00000200: Rate 36 Mbit/s
    /// - 0x00000400: Rate 48 Mbit/s
    /// - 0x00000800: Rate 54 Mbit/s
    /// - 0x00001000: Rate MCS 0
    /// - 0x00002000: Rate MCS 1
    /// - 0x00004000: Rate MCS 2
    /// - 0x00008000: Rate MCS 3
    /// - 0x00010000: Rate MCS 4
    /// - 0x00020000: Rate MCS 5
    /// - 0x00040000: Rate MCS 6
    /// - 0x00080000: Rate MCS 7
    /// - 0x00100000: Rate MCS 8
    /// - 0x00200000: Rate MCS 9
    /// - 0x00400000: Rate MCS 10
    /// - 0x00800000: Rate MCS 11
    /// - 0x01000000: Rate MCS 12
    /// - 0x02000000: Rate MCS 13
    /// - 0x04000000: Rate MCS 14
    /// - 0x08000000: Rate MCS 15 The default value is 0, which means that all
    ///   rates are enabled. Supported software versions 6.0.0 onwards
    #[at_arg(value = 15)]
    APRates(u32),
    /// <param_val1> Hidden SSID configuration.
    /// - Bit 0 (default): Disable hidden SSID
    /// - Bit 1: Enable hidden SSID Supported software versions 6.0.0 onwards
    #[at_arg(value = 16)]
    HiddenSSID(OnOff),
    /// White List - <param_val1>...<param_val10> List of MAC addresses of
    /// stations that is allowed to connect or 0 to allow all. The factory
    /// default is 0.
    #[at_arg(value = 19)]
    WhiteList(String<64>, String<64>, String<64>),
    /// Black List - <param_val1>...<param_val10> List of MAC addresses of
    /// stations that will be rejected or 0 to not reject any. The factory
    /// default is 0.
    #[at_arg(value = 20)]
    BlackList(String<64>, String<64>, String<64>),
    /// IPv4 Mode - <param_val1> to set the way to retrieve an IP address
    /// - 1:(default) Static
    #[at_arg(value = 100)]
    IPv4Mode(IPv4Mode),
    /// <param_val> is the IPv4 address. The factory default value is
    /// 192.168.2.1
    #[at_arg(value = 101)]
    IPv4Address(#[at_arg(len = 16)] Ipv4Addr),
    /// <param_val> is the subnet mask. The factory default value is
    /// 255.255.255.0
    #[at_arg(value = 102)]
    SubnetMask(#[at_arg(len = 16)] Ipv4Addr),
    /// <param_val> is the default gateway. The factory default value is
    /// 192.168.2.1
    #[at_arg(value = 103)]
    DefaultGateway(#[at_arg(len = 16)] Ipv4Addr),
    /// <param_val> is the primary DNS server IP address. The factory default
    /// value is 0.0.0.0
    #[at_arg(value = 104)]
    PrimaryDNS(#[at_arg(len = 16)] Ipv4Addr),
    /// <param_val> is the secondary DNS server IP address. The factory default
    /// value is 0.0.0.0
    #[at_arg(value = 105)]
    SecondaryDNS(#[at_arg(len = 16)] Ipv4Addr),
    /// <param_val> is the DHCP server configuration.
    /// - 0 (default): Disable DHCP server
    /// - 1 Enable DHCP server. The DHCP Server will provide addresses according
    ///   to the following formula: (Static address and subnet mask) + 100
    #[at_arg(value = 106)]
    DHCPServer(OnOff),
    /// Address conflict detection. The factory default value is 0 (disabled).
    /// - 0: Disabled
    /// - 1: Enabled Supported software versions 6.0.0 onwards
    #[at_arg(value = 107)]
    AddressConflictDetection(OnOff),
    ///  IPv6 Mode - <param_val> to set the way to retrieve an IP address
    /// - 1 (default): Link Local IP address
    #[at_arg(value = 200)]
    IPv6Mode(IPv6Mode),
    /// <param_val> is the IPv6 link local address. If the value is not set, the
    /// link local address is automatically generated from the interface IEEE 48
    /// bit MAC identifier. The factory default value is:
    #[at_arg(value = 201)]
    IPv6LinkLocalAddress(#[at_arg(len = 40)] Ipv6Addr),
    /// <param_val> is the DTIM interval. The factory default value is 1. Valid
    /// values are 1 to 100.
    #[at_arg(value = 301)]
    DTIM(u8),
}

#[derive(Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum SecurityMode {
    Open = 1,
    Wpa2AesCcmp = 2,
    WpaWpa2Mixed = 3,
    WpaRC4Tkip = 4,
}

#[derive(Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum SecurityModePSK {
    Open = 1,
    PSK = 2,
}

#[derive(Clone, PartialEq, AtatEnum)]
pub enum Passkey<'a> {
    Passphrase(#[at_arg(len = 64)] &'a str),
    PSK(#[at_arg(len = 64)] &'a [u8]),
}

#[derive(Clone, PartialEq, AtatEnum)]
pub enum PasskeyR {
    Passphrase(String<64>),
    PSK(Vec<u8, 64>),
}

#[derive(Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum PMF {
    /// PMF Disable (PMF Capable = 0, PMF Required = 0)
    Disable = 0,
    /// (default): PMF Optional (PMF Capable = 1, PMF Required = 0)
    Optional = 1,
    /// PMF Required (PMF Capable = 1, PMF Required = 1)
    Required = 2,
}

#[derive(Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum IPv4Mode {
    Cleared = 0,
    Static = 1,
    DHCP = 2,
}

#[derive(Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum IPv6Mode {
    LinkLocalIPAddress = 1,
}

#[derive(Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum AccessPointAction {
    /// Reset; it clears the specified profile resetting all the parameters to
    /// their factory programmed values
    Reset = 0,
    /// Store; validates the configuration, calculates the PSK for WPA and WPA2
    /// (if not already calculated) and saves the configuration.
    Store = 1,
    /// Load: it reads all the parameters from memory
    Load = 2,
    /// Activate; validates the configuration, calculates the PSK for WPA and
    /// WPA2 (if not already calculated) and activates the specified profile. It
    /// will try to connect if not connected.
    Activate = 3,
    /// Deactivate; it deactivates the specified profile. Disconnects the
    /// profile, if connected and may reconnect to other active networks
    Deactivate = 4,
}

#[derive(Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum AccessPointStatusId {
    /// The <status_val> is the currently used SSID.
    SSID = 0,
    /// The <status_val> is the currently used BSSID.
    BSSID = 1,
    /// The <status_val> is the currently used channel.
    Channel = 2,
    /// The <status_val> is the current status of the access point.
    /// - 0: disabled
    /// - 1: enabled
    Status = 3,
}

#[derive(Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum AccessPointStatus {
    /// The <status_val> is the currently used SSID.
    #[at_arg(value = 0)]
    SSID(String<64>),
    /// The <status_val> is the currently used BSSID.
    #[at_arg(value = 1)]
    BSSID(Bytes<20>),
    /// The <status_val> is the currently used channel.
    #[at_arg(value = 2)]
    Channel(u32),
    /// The <status_val> is the current status of the access point.
    /// - 0: disabled
    /// - 1: enabled
    #[at_arg(value = 3)]
    Status(OnOff),
}

#[derive(Clone, PartialEq, AtatEnum)]
pub enum AccessPointStatusValue {
    Unsigned(u8),
    String(String<64>),
}

#[derive(Debug, Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum DisconnectReason {
    Unknown = 0,
    RemoteClose = 1,
    OutOfRange = 2,
    Roaming = 3,
    SecurityProblems = 4,
    NetworkDisabled = 5,
}
