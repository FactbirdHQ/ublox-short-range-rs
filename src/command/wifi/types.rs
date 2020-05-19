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
pub enum WifiStationConfigParameter {
    /// <param_val1> decides if the station is active on start up.
    /// • 0 (default): inactive
    /// • 1: active
    ActiveOnStartup = 0,
    ///  SSID - <param_val1> is the Service Set Identifier. The factory default value is an
    /// empty string ("").
    SSID = 1,
    /// Authentication - <param_val> is the authentication type.
    /// • 1 (default): Open
    /// • 2: WPA/WPA2 PSK
    /// • 3: LEAP
    /// • 4: PEAP
    /// • 5: EAP-TLS
    Authentication = 5,
    /// WEP Keys - <param_val1>...<param_val4> are the WEP encryption keys. A WEP
    /// key is either 5 bytes (while using WEP 64), or 13 bytes (while using WEP 128) and if not
    /// used, it can be empty. The keys must be in HEX data format. For example, 010203040
    /// 5 while using WEP 64, or 0102030405060708090A0B0C0D while using WEP 128.
    ///
    /// "WEP Shared Key Authentication" is not supported; only "WEP Open Key
    /// Authentication " is supported.
    WEPKeys = 6,
    ///  Active Key - <param_val1> is the WEP active TX key (factory default 0 means that
    /// Open authentication with WEP encryption is disabled). Range 1-4.
    ActiveKey = 7,
    /// PSK/Passphrase - <param_val1> is the PSK (32 HEX values) or Passphrase (8-63
    /// ASCII characters as a string) for WPA/WPA2 PSK.
    WPA_PSKOrPassphrase = 8,
    /// Password - <param_val1> is the password for LEAP and PEAP; string with a
    /// maximum length of 31.
    EAPPassword = 9,
    /// User name - <param_val1> is the public user name for LEAP and PEAP; string with
    /// a maximum length of 31.
    UserName = 10,
    /// Domain name - <param_val1> is the public domain name for LEAP and PEAP; string
    /// with a maximum length of 63. The domain name is an optional parameter.
    DomainName = 11,
    /// Client certificate name - <param_val1> is the internal client certificate name
    /// for EAP-TLS as defined in the SSL/TLS certificates and private keys manager
    /// +USECMNG command; string with a maximum length of 32.
    /// Supported software versions 4.0.0 onwards
    ClientCertificateName = 12,
    /// Client private key - <param_val1> is the internal client private key name for EAP-
    /// TLS as defined in the SSL/TLS certificates and private keys manager +USECMNG
    /// command; string with a maximum length of 32.
    /// Supported software versions 4.0.0 onwards
    ClientPrivateKey = 13,
    /// CA certificate name - <param_val1> is the internal CA certificate name for EAP-
    /// TLS as defined in the SSL/TLS certificates and private keys manager +USECMNG
    /// command; string with a maximum length of 32.
    /// Supported software versions 5.0.0 onwards
    CACertificateName = 14,
    /// Validate CA certificate. The default value is 1; Setting this value to 0 means no CA
    /// Certificate validation has been done. For example at+uwsc=0,15,0 would mean that
    /// the server CA Certificate is not validated during authentication.
    /// Supported software versions 5.0.0 onwards
    ValidateCACertificate = 15,
    /// IPv4 Mode - <param_val1> to set the way to retrieve an IP address
    /// • 1: Static
    /// • 2 (default): DHCP
    IPv4Mode = 100,
    /// IPv4 address - <param_val> is the IPv4 address. The factory default value is
    /// 0.0.0.0. Valid only if param_tag 100 is set to Static.
    IPv4Address = 101,
    /// Subnet mask - <param_val> is the subnet mask. The factory default value is
    /// 0.0.0.0. Valid only if param_tag 100 is set to Static.
    SubnetMask = 102,
    /// Default gateway - <param_val> is the default gateway. The factory default value
    /// is 0.0.0.0. Valid only if param_tag 100 is set to Static.
    DefaultGateway = 103,
    /// DNS server 1 - <param_val> is the primary DNS server IP address. The factory
    /// default value is 0.0.0.0. Valid only if param_tag 100 is set to Static.
    DNSServer1 = 104,
    /// DNS server 2 - <param_val> is the primary DNS server IP address. The factory
    /// default value is 0.0.0.0. Valid only if param_tag 100 is set to Static.
    DNSServer2 = 105,
    /// Address conflict detection. The factory default value is 0 (disabled).
    /// • 0: Disabled
    /// • 1: Enabled
    AddressConflictDetection = 106,
    /// IPv6 Mode - <param_val1> to set the way to retrieve an IP address
    /// • 1 (default): Link Local IpAddress
    IPv6Mode = 200,
    /// <param_val> is the IPv6 link local address. If the value is not set, the link local
    /// address is automatically generated from the interface IEEE 48 bit MAC identifier.
    IPv6LinkLocalAddress = 201,
    /// <param_val> is the Wi-Fi beacon listen interval in units of beacon interval. The
    /// factory default value is 0, listen on all beacons.
    /// • Valid values 0-16
    WiFiBeaconListenInteval = 300,
    /// <param_val> Enables DTIM in power save. If the DTIM is enabled and the module
    /// is in power save, the access point sends an indication when new data is available. If
    /// disabled, the module polls for data every beacon listen interval. The factory default
    /// value is enabled.
    /// • 0: Disabled
    /// • 1: Enabled
    /// To use WEP with open authentication, the WEP key index must be different from zero
    /// (0).
    DTIMInPowerSave = 301,
}

#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
pub enum WiFiConfigValue{
    Number(u32),
    String(String<consts::u64>),
    Array(&u8),
    IpAddr(IpAddr),
}

#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum OperationMode{
    Infrastructure = 1,
    Ad_Hoc = 2,
}

#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum StatusId{
    SSID = 0,
    BSSID = 1,
    Channel = 2,
    ///The <status_val> is the current status of the station, possible values of status_val
    /// are:
    /// • 0: Disabled,
    /// • 1: Disconnected,
    /// • 2: Connected,
    Status = 6,
    /// The <status_val> is the mobility domain of the last or current connection
    /// This tag is supported by ODIN-W2 from software version 6.0.0 onwards only.
    MobilityDomain = 7,
    /// The <status_val> is the region to which the module complies according to the
    /// accepted Wi-Fi channels:
    /// • 0: World
    /// • 1: FCC
    /// • 2: ETSI
    /// • 3: ALL (test modes only)
    /// This tag is supported by ODIN-W2 from software version 6.0.0 onwards only.
    Region = 8,
}
#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum WifiConfigParameter{
    /// Wi-Fi enabled
    /// • 0: Disabled
    /// • 1:* Enabled
    /// • 2: Auto.
    WifiEnabled = 0,
    /// Wi-Fi power save mode
    // • 0: Active mode
    // • 1: Sleep mode
    // • 2 (default): Deep sleep mode
    PowerSaveMode = 1,
    /// <param_val> is the transmit power level in dBm. Valid values are 0-20 and 255.
    /// Adaptive transmit power level control is enabled with 255. The factory default value is
    /// 255.
    PowerLevel = 2,
    /// Good RSSI value
    /// When an AP is found with better or equal RSSI, the module will abort the scanning
    /// and connect to the AP. Valid values are -128 to 0. The default value is -55.
    GoodRSSIValue = 5,
    ///  Bad RSSI value
    /// This value is defined when you are in an area with bad coverage. That is, the fast scan
    /// sleep timeout (param_tag 8) will be used to find a better alternative. Valid values are -
    /// 128 to 0. The default value is -70.
    /// Supported software versions 5.0.0 onwards
    BadRSSIValue = 6,
    /// Slow scan sleep timeout
    /// <param_val> is the timeout in ms for scanning two channels when the module is
    /// connected to an AP with an RSSI value that is above the Bad RSSI value (param_tag
    /// 6). Set to 0 to turn the neighborhood watch off when there is a good signal strength.
    /// Valid values are 0 - 2147483647. The default value is 2000.
    /// Supported software versions 5.0.0 onwards
    SlowScanSleepTimeout = 7,
    /// Fast scan sleep timeout
    /// <param_val> is the timeout in ms for scanning two channels when the module is
    /// connected to an AP with an RSSI value that is below the Bad RSSI value (param_tag
    /// 6). Set to 0 to turn off roaming. Valid values are 0 - 2147483647. The default value is
    /// 150.
    /// Supported software versions 5.0.0 onwards
    FastScanSleepTimeout = 8,
    /// Last BSSID block time
    /// <param_val> is the time in seconds a switch to the last connected AP is blocked.
    /// Valid values are 0 - 2147483. The default value is 5.
    /// Supported software versions 5.0.0 onwards
    LastBSSIDBlockTime = 9,
    /// Drop network on link loss
    /// • 0 (default): Do not drop the network when there is a Wi-Fi link loss
    /// • 1: Drop the network when the Wi-Fi link is lost; data may be lost with this option.
    /// Supported software versions 5.0.0 onwards
    DropNetworkOnLinkLoss = 10,
    /// Force world mode
    /// • 0: Use all channels in the channel list; See +UWCL for more information. The
    ///      channel list will be filtered by 802.11d.
    /// • 1 (default): Lock device to world mode. The channel list (+UWCL) is filtered and only
    ///     the channels in the following ranges will be used - 1-11, 36-64, 100-116, 132-140.
    ///     For the updated "Force world mode" settings to take affect, the Wi-Fi radio must
    ///     be restarted. This can be done by the Wi-Fi disable/enable command (parameter
    ///     0) or by storing the setting (&W) to non-volatile memory and restarting the
    ///     module.
    /// Supported software versions 5.0.0 onwards
    ForceWorldMode = 11,
    /// Fast transition mode (802.11r)
    /// • 0: Disabled, never use fast transitions.
    /// • 1: Over air, use fast transitions "Over air" instead of "Over DS", even though "Over
    ///      DS" support is announced by the APs.
    /// • 2 (default): Over DS, follow the mode announced by the APs
    /// Supported software versions 6.0.0 onwards
    FastTransitionMode = 12,
    /// Scan listen interval
    /// <param_val> is the timeout (in ms) between scanning one channel and another. The
    /// default value is 0 ms.
    /// Supported software versions 6.0.0 onwards
    ScanListenInterval = 14,
    /// Remain on channel
    /// • 1 (default): Enable remain on channel
    /// • 0: Disable remain on channel
    /// Supported software versions 6.0.0 onwards
    RemainOnChannel = 15,
    /// Station TX rates bit mask where bit masks are defined according to:
    /// 0x00000001: Rate 1 Mbps
    /// 0x00000002: Rate 2 Mbps
    /// 0x00000004: Rate 5.5 Mbps
    /// 0x00000008: Rate 11 Mbps
    /// 0x00000010: Rate 6 Mbps
    /// 0x00000020: Rate 9 Mbps
    /// 0x00000040: Rate 12 Mbps
    /// 0x00000080: Rate 18 Mbps
    /// 0x00000100: Rate 24 Mbps
    /// 0x00000200: Rate 36 Mbps
    /// 0x00000400: Rate 48 Mbps
    /// 0x00000800: Rate 54 Mbps
    /// 0x00001000: Rate MCS 0
    /// 0x00002000: Rate MCS 1
    /// 0x00004000: Rate MCS 2
    /// 0x00008000: Rate MCS 3
    /// 0x00010000: Rate MCS 4
    /// 0x00020000: Rate MCS 5
    /// 0x00040000: Rate MCS 6
    /// 0x00080000: Rate MCS 7
    /// 0x00100000: Rate MCS 8
    /// 0x00200000: Rate MCS 9
    /// 0x00400000: Rate MCS 10
    /// 0x00800000: Rate MCS 11
    /// 0x01000000: Rate MCS 12
    /// 0x02000000: Rate MCS 13
    /// 0x04000000: Rate MCS 14
    /// 0x08000000: Rate MCS 15
    /// Default value is 0, which means that all rates are enabled.
    /// Supported software versions 7.0.0 onwards
    StationTxRates = 16,
    /// Station short packet retry limit. Default value is 0x00141414.
    /// The definition of retry limits are listed below:
    /// • Bits 31-24: Reserved
    /// • Bits 23-16: EAPOL & Broadcast (0x01-0xFF)
    /// • Bits 15-8: MGMT (0x01-0xFF)
    /// • Bits 7-0: Data (0x01-0xFF)
    /// Supported software versions 7.0.0 onwards
    StationShortPacketRetryLimit = 17,
    /// Station long packet retry limit. Default value is 0x00141414.
    /// The definition of retry limits are listed below:
    /// • Bits 31-24: Reserved
    /// • Bits 23-16: EAPOL & Broadcast (0x01-0xFF)
    /// • Bits 15-8: MGMT (0x01-0xFF)
    /// • Bits 7-0: Data (0x01-0xFF)
    /// Supported software versions 7.0.0 onwards
    StationLongPacketRetryLimit = 18,
    /// AP short packet retry limit. Default value is 0x00141414.
    /// The definition of retry limits are listed below:
    /// • Bits 31-24: Reserved
    /// • Bits 23-16: EAPOL & Broadcast (0x01-0xFF)
    /// • Bits 15-8: MGMT (0x01-0xFF)
    /// • Bits 7-0: Data (0x01-0xFF)
    /// Supported software versions 7.0.0 onwards
    APShortPacketRetryLimit = 19,
    /// AP long packet retry limit. Default value is 0x00141414.
    /// The definition of retry limits are listed below:
    /// • Bits 31-24: Reserved
    /// • Bits 23-16: EAPOL & Broadcast (0x01-0xFF)
    /// • Bits 15-8: MGMT (0x01-0xFF)
    /// • Bits 7-0: Data (0x01-0xFF)
    /// Supported software versions 7.0.0 onwards
    APLongPacketRetryLimit = 20,
    ///  Scan Type
    /// • 1 (default): Active scan
    /// • 2: Passive scan
    /// Supported software versions 7.0.0 onwards
    ScanType = 21,
    /// Scan Filter
    /// • 0 (default): Do not filter scan results
    /// • 1: Filter scan results; the module will try to only send one scan response for each
    ///     BSSID. In environments with a high number of networks, this may not work.
    /// Supported software versions 7.0.0 onwards
    ScanFilter = 22,
    /// Enable block acknowledgement
    /// • 0 (default): Disable block acknowledgement
    /// • 1: Enable block acknowledgement
    /// Supported software versions 7.0.2 onwards
    BlockAcknowledgment = 23,
    /// Minimum TLS version. Possible values are:
    /// • 1 (default): TLS v1.0
    /// • 2: TLS v1.1
    /// • 3: TLS v1.2
    /// Supported software versions 7.0.2 onwards
    MinimumTlsVersion = 24,
    /// Maximum TLS version. Possible values are:
    /// • 1: TLS v1.0
    /// • 2: TLS v1.1
    /// • 3 (default): TLS v1.2
    /// Supported software versions 7.0.2 onwards
    MaximumTlsVersion = 25,
}
#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
pub enum ConfigValue {
    Unsigned(u32),
    Signed(i32)
}

#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum WatchdogSetting{
    DisconnectReset = 0,
}

#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum AccessPointId{
    Id0 = 0,
}


#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum AccessPointConfig{
    /// <param_val1> decides if the access point is active on start up.
    /// • 0 (default): inactive
    /// • 1: active
    ActiveOnStartup = 0,
    /// SSID - <param_val1> is the Service Set identification of the access point. The
    /// factory-programmed value is ("UBXWifi").
    SSID = 1,
    /// <param_val1> is the channel. Factory programmed value is 6.
    Channel = 4,
    /// Security mode
    /// <param_val1>:
    /// • 1: Open
    /// • 2 (default): WPA2 (AES-CCMP)
    /// • 3: WPA/WPA2 Mixed mode (RC4-TKIP + AES-CCMP)
    /// • 4: WPA (RC4-TKIP)
    /// <param_val2>:
    /// • 1: Open
    /// • 2 (default): Pre shared key PSK
    SecurityMode = 5,
    /// PSK/Passphrase - <param_val1> is the PSK (32 HEX values) or Passphrase (8-63
    /// ascii characters as a string) for WPA and WPA2, default: "ubx-wifi". This tag does not
    /// support reading.
    PSK_Passphrase = 8,
    /// <param_val1> is a bitmask representing the mandatory 802.11b rates.
    /// • Bit 0 (default): 1 Mbit/s
    /// • Bit 1: 2 Mbit/s
    /// • Bit 2: 5.5 Mbit/s
    /// • Bit 3: 11 Mbit/s
    Rates802_11b = 12,
    /// <param_val1> is a bitmask representing the mandatory 802.11ag rates.
    /// • Bit 0 (default): 6 Mbit/s
    /// • Bit 1: 9
    /// • Bit 2: 12 Mbit/s
    /// • Bit 3: 18 Mbit/s
    /// • Bit 4: 24 Mbit/s
    /// • Bit 5: 36 Mbit/s
    /// • Bit 6: 48 Mbit/s
    /// • Bit 7: 54 Mbit/s
    Rates802_11ag = 13,
    /// <param_val1> Protected Management Frames (PMF)
    /// • 0: PMF Disable (PMF Capable = 0, PMF Required = 0)
    /// • 1 (default): PMF Optional (PMF Capable = 1, PMF Required = 0)
    /// • 2: PMF Required (PMF Capable = 1, PMF Required = 1)
    /// Supported software versions 6.0.0 onwards
    ProtectedManagementFrames = 14,
    /// <param_val1> Access point supported rates bit mask where the bit masks are
    /// defined according to:
    /// • 0x00000001: Rate 1 Mbit/s
    /// • 0x00000002: Rate 2 Mbit/s
    /// • 0x00000004: Rate 5.5 Mbit/s
    /// • 0x00000008: Rate 11 Mbit/s
    /// • 0x00000010: Rate 6 Mbit/s
    /// • 0x00000020: Rate 9 Mbit/s
    /// • 0x00000040: Rate 12 Mbit/s
    /// • 0x00000080: Rate 18 Mbit/s
    /// • 0x00000100: Rate 24 Mbit/s
    /// • 0x00000200: Rate 36 Mbit/s
    /// • 0x00000400: Rate 48 Mbit/s
    /// • 0x00000800: Rate 54 Mbit/s
    /// • 0x00001000: Rate MCS 0
    /// • 0x00002000: Rate MCS 1
    /// • 0x00004000: Rate MCS 2
    /// • 0x00008000: Rate MCS 3
    /// • 0x00010000: Rate MCS 4
    /// • 0x00020000: Rate MCS 5
    /// • 0x00040000: Rate MCS 6
    /// • 0x00080000: Rate MCS 7
    /// • 0x00100000: Rate MCS 8
    /// • 0x00200000: Rate MCS 9
    /// • 0x00400000: Rate MCS 10
    /// • 0x00800000: Rate MCS 11
    /// • 0x01000000: Rate MCS 12
    /// • 0x02000000: Rate MCS 13
    /// • 0x04000000: Rate MCS 14
    /// • 0x08000000: Rate MCS 15
    /// The default value is 0, which means that all rates are enabled.
    /// Supported software versions 6.0.0 onwards
    APRates = 15,
    /// <param_val1> Hidden SSID configuration.
    /// • Bit 0 (default): Disable hidden SSID
    /// • Bit 1: Enable hidden SSID
    /// Supported software versions 6.0.0 onwards
    HiddenSSID = 16,
    /// White List - <param_val1>...<param_val10> List of MAC addresses of stations that
    /// is allowed to connect or 0 to allow all. The factory default is 0.
    WhiteList = 19,
    /// Black List - <param_val1>...<param_val10> List of MAC addresses of stations that
    /// will be rejected or 0 to not reject any. The factory default is 0.
    BlackList = 20,
    /// IPv4 Mode - <param_val1> to set the way to retrieve an IP address
    /// • 1:(default) Static
    IPv4Mode = 100,
    /// <param_val> is the IPv4 address. The factory default value is 192.168.2.1
    IPv4Address = 101,
    /// <param_val> is the subnet mask. The factory default value is 255.255.255.0
    SubnetMask = 102,
    /// <param_val> is the default gateway. The factory default value is 192.168.2.1
    DefaultGateway = 103,
    /// <param_val> is the primary DNS server IP address. The factory default value is 0.0.0.0
    PrimaryDNS = 104,
    /// <param_val> is the secondary DNS server IP address. The factory default value is
    /// 0.0.0.0
    SecondaryDNS = 105,
    /// <param_val> is the DHCP server configuration.
    /// • 0 (default): Disable DHCP server
    /// • 1 Enable DHCP server. The DHCP Server will provide addresses according to the
    /// following formula: (Static address and subnet mask) + 100
    DHCPConfig = 106,
    /// Address conflict detection. The factory default value is 0 (disabled).
    /// • 0: Disabled
    /// • 1: Enabled
    /// Supported software versions 6.0.0 onwards
    AddressConflictDetection = 107,
    ///  IPv6 Mode - <param_val> to set the way to retrieve an IP address
    /// • 1 (default): Link Local IP address
    IPv6Mode = 200,
    /// <param_val> is the IPv6 link local address. If the value is not set, the link local
    /// address is automatically generated from the interface IEEE 48 bit MAC identifier. The
    /// factory default value is:
    IPv6LinkLocalAddress = 201,
    /// <param_val> is the DTIM interval. The factory default value is 1. Valid values are
    /// 1 to 100.
    DTIM = 300,
}

#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
pub enum AccessPointConfigValue{
    Unsigned(u32),
    String(String<consts::U64>),
    SecurityMode((u8,u8)),
    //TODO fix me
    List((String<consts::U64>,String<consts::U64>,String<consts::U64>)), 
}

#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum AccessPointAction{
    /// Reset; it clears the specified profile resetting all the parameters to their factory
    /// programmed values
    Reset = 0,
    /// Store; validates the configuration, calculates the PSK for WPA and WPA2 (if not
    /// already calculated) and saves the configuration.
    Store = 1,
    /// Load: it reads all the parameters from memory
    Load = 2,
    /// Activate; validates the configuration, calculates the PSK for WPA and WPA2 (if
    /// not already calculated) and activates the specified profile. It will try to connect if not
    /// connected.
    Activate = 3,
    /// Deactivate; it deactivates the specified profile. Disconnects the profile, if connected
    /// and may reconnect to other active networks
    Deactivate = 4,
}

#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum AccessPointStatusId{
    /// The <status_val> is the currently used SSID.
    SSID = 0,
    /// The <status_val> is the currently used BSSID.
    BSSID = 1,
    /// The <status_val> is the currently used channel.
    Channel = 2,
    /// The <status_val> is the current status of the access point.
    /// • 0: disabled
    /// • 1: enabled
    Status = 3,
}

#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
pub enum AccessPointStatusValue{
    Unsigned(u8),
    String(String<consts::U64>),
}