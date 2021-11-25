//! Argument and parameter types used by Network Commands and Responses

use atat::atat_derive::AtatEnum;
use atat::heapless_bytes::Bytes;
use embedded_nal::{IpAddr, Ipv4Addr, Ipv6Addr};
use heapless::String;
#[derive(Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum OnOff {
    Off = 0,
    On = 1,
}

#[derive(Clone, PartialEq, AtatEnum)]
pub enum NetworkStatus {
    /// 0: The <status_val> is the interface hardware address (displayed only if applicable).
    #[at_arg(value = 0)]
    HardwareAddress(String<64>),
    /// 1: The <status_val> is the current status of the network interface (Layer-3).
    /// - 0: Network down
    /// - 1: Network up
    #[at_arg(value = 1)]
    Status(OnOff),
    /// 2: The <interface_type> is the interface type.
    /// - 0: Unknown
    /// - 1: Wi-Fi Station
    /// - 2: Wi-Fi Access Point
    /// - 3: Ethernet
    /// - 4: PPP
    /// - 5: Bridge
    /// - 6: Bluetooth PAN - This interface type is supported by ODIN-W2 from software
    /// version 5.0.0 onwards only.
    #[at_arg(value = 2)]
    InterfaceType(InterfaceType),
    /// 101: The <status_val> is the currently used IPv4_Addr (omitted if no IP address has
    /// been acquired).
    #[at_arg(value = 101)]
    IPv4Address(#[at_arg(len = 16)] Bytes<16>),
    /// 102: The <status_val> is the currently used subnet mask (omitted if no IP address
    /// has been acquired).
    #[at_arg(value = 102)]
    SubnetMask(#[at_arg(len = 16)] Bytes<16>),
    /// 103: The <status_val> is the currently used gateway (omitted if no IP address has
    /// been acquired).
    #[at_arg(value = 103)]
    Gateway(#[at_arg(len = 16)] Bytes<16>),
    /// 104: The <status_val> is the current primary DNS server.
    #[at_arg(value = 104)]
    PrimaryDNS(#[at_arg(len = 16)] Bytes<16>),
    /// 105: The <status_val> is the current secondary DNS server.
    #[at_arg(value = 105)]
    SecondaryDNS(#[at_arg(len = 16)] Bytes<16>),
    /// 201: The <status_val> is the current IPv6 link local address.
    #[at_arg(value = 201)]
    IPv6LinkLocalAddress(#[at_arg(len = 40)] Bytes<40>),
    /// 210-212: The <status_val> is an IPv6 address. For ODIN-W2, the IPv6 addresses are
    /// only sent from software version 7.0.0 onwards.
    #[at_arg(value = 210)]
    IPv6Address1(NetworkIpv6Status),
    /// 210-212: The <status_val> is an IPv6 address. For ODIN-W2, the IPv6 addresses are
    /// only sent from software version 7.0.0 onwards.
    #[at_arg(value = 211)]
    IPv6Address2(NetworkIpv6Status),
    /// 210-212: The <status_val> is an IPv6 address. For ODIN-W2, the IPv6 addresses are
    /// only sent from software version 7.0.0 onwards.
    #[at_arg(value = 212)]
    IPv6Address3(NetworkIpv6Status),
}

#[derive(Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum NetworkStatusParameter {
    /// 0: The <status_val> is the interface hardware address (displayed only if applicable).
    HardwareAddress = 0,
    /// 1: The <status_val> is the current status of the network interface (Layer-3).
    /// - 0: Network down
    /// - 1: Network up
    Status = 1,
    /// 2: The <interface_type> is the interface type.
    /// - 0: Unknown
    /// - 1: Wi-Fi Station
    /// - 2: Wi-Fi Access Point
    /// - 3: Ethernet
    /// - 4: PPP
    /// - 5: Bridge
    /// - 6: Bluetooth PAN - This interface type is supported by ODIN-W2 from software
    /// version 5.0.0 onwards only.
    InterfaceType = 2,
    /// 101: The <status_val> is the currently used IPv4_Addr (omitted if no IP address has
    /// been acquired).
    IPv4Address = 101,
    /// 102: The <status_val> is the currently used subnet mask (omitted if no IP address
    /// has been acquired).
    SubnetMask = 102,
    /// 103: The <status_val> is the currently used gateway (omitted if no IP address has
    /// been acquired).
    Gateway = 103,
    /// 104: The <status_val> is the current primary DNS server.
    PrimaryDNS = 104,
    /// 105: The <status_val> is the current secondary DNS server.
    SecondaryDNS = 105,
    /// 201: The <status_val> is the current IPv6 link local address.
    IPv6LinkLocalAddress = 201,
    /// 210-212: The <status_val> is an IPv6 address. For ODIN-W2, the IPv6 addresses are
    /// only sent from software version 7.0.0 onwards.
    IPv6Address1 = 210,
    /// 210-212: The <status_val> is an IPv6 address. For ODIN-W2, the IPv6 addresses are
    /// only sent from software version 7.0.0 onwards.
    IPv6Address2 = 211,
    /// 210-212: The <status_val> is an IPv6 address. For ODIN-W2, the IPv6 addresses are
    /// only sent from software version 7.0.0 onwards.
    IPv6Address3 = 212,
}

#[derive(Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum InterfaceType {
    Unknown = 0,
    WifiStation = 1,
    WifiAccessPoint = 2,
    Ethernet = 3,
    PPP = 4,
    Bridge = 5,
    BluetoothPAN = 6,
}

#[derive(Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum NetworkIpv6Status {
    /// Invalid
    Invalid = 0,
    /// Tentative
    Tentative = 1,
    /// Preferred
    Preferred = 2,
    /// Deprecated
    Deprecated = 3,
}

#[derive(Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum RoutingTag {
    Enabled = 0,
}

#[derive(Clone, PartialEq, AtatEnum)]
pub enum BridgeConfigId {
    Id1 = 0,
    Id2 = 1,
}

#[derive(Clone, PartialEq, AtatEnum)]
pub enum BridgeConfig {
    /// <param_val1> decides if the bridge is active on start up.
    /// - 0 (default): Inactive
    /// - 1: Active
    #[at_arg(value = 0)]
    ActiveOnStartup(OnOff),
    /// Link layer list. The list defines the interfaces that shall be bridged.
    /// The factory default value is an empty list.
    /// The following interfaces can be bridged:
    /// - 1: Wi-Fi Station
    /// - 2: Wi-Fi Access Point
    /// - 3: Ethernet
    /// - 6: Bluetooth PAN - This interface is supported by ODIN-W2 from software version
    /// 6.0.0 onwards only.
    /// For example, AT+UBRGC = 0,1,1,3. This adds the Wi-Fi station and Ethernet
    /// interfaces to the bridge.
    #[at_arg(value = 1)]
    LinkLayerList(Option<u8>, Option<u8>, Option<u8>, Option<u8>),
    /// IP interface list. This list defines the interfaces that accept IP
    /// traffic. The factory default value is an empty list.
    /// The following interfaces can accept the IP traffic:
    /// - 1: Wi-Fi Station
    /// - 2: Wi-Fi Access Point
    /// - 3: Ethernet
    /// - 6: Bluetooth PAN - This interface is supported by ODIN-W2 from software version
    /// 6.0.0 onwards only.
    /// For example, AT+UBRGC = 0,2,1,3. This allows the Wi-Fi station and Ethernet
    /// interfaces to accept IP traffic.
    #[at_arg(value = 2)]
    IPInterfaceList(Option<u8>, Option<u8>, Option<u8>, Option<u8>),
    /// IPv4 Mode - <param_val1> to set the way to retrieve an IP address
    /// - 0 (default): None
    /// - 1: Static
    #[at_arg(value = 100)]
    IPv4Mode(IPv4Mode),
    /// <param_val> is the IPv4 address. The factory default value is 0.0.0.0
    #[at_arg(value = 101)]
    IPv4Address(#[at_arg(len = 16)] IpAddr),
    /// <param_val> is the subnet mask. The factory default value is 0.0.0.0
    #[at_arg(value = 102)]
    SubnetMask(#[at_arg(len = 16)] IpAddr),
    /// <param_val> is the default gateway. The factory default value is 0.0.0.0
    #[at_arg(value = 103)]
    DefaultGateway(#[at_arg(len = 16)] IpAddr),
    /// <param_val> is the primary DNS server IP address. The factory default value is 0
    /// .0.0.0
    #[at_arg(value = 104)]
    PrimaryDNS(#[at_arg(len = 16)] IpAddr),
    /// <param_val> is the secondary DNS server IP address. The factory default value is
    /// 0.0.0.0
    #[at_arg(value = 105)]
    SecondaryDNS(#[at_arg(len = 16)] IpAddr),
    /// <param_val> is the DHCP server configuration.
    /// - 0 (default): Disable DHCP server
    /// - 1: Enable DHCP server. The DHCP Server will provide addresses according to the
    /// following formula - "(Static address and subnet mask) + 100". If DHCP Server is
    /// enabled, the IPv4 Mode should be set to static.
    #[at_arg(value = 106)]
    DHCPServer(OnOff),
    /// Address conflict detection. The factory default value is 0 (disabled). This tag is
    /// supported by ODIN-W2 from software version 6.0.0 onwards only.
    /// - 0: Disabled
    /// - 1: Enabled
    #[at_arg(value = 107)]
    AddressConflictDetection(OnOff),
}

#[derive(Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum IPv4Mode {
    Static = 1,
    DHCP = 2,
}

#[derive(Clone, PartialEq, AtatEnum)]
pub enum BridgeAction {
    /// Clears the configuration and reset all parameters to factory defaults.
    Reset = 0,
    /// Validates and saves the configuration.
    Store = 1,
    /// Reads the configuration from non-volatile memory to run-time memory.
    Load = 2,
    /// Validates and activates the configuration.
    /// - When a bridge is activated, the data on all network interfaces connected to
    /// the bridge is handled by the bridge. The IP configuration set in the individual
    /// network interface configurations is not used while the IP configuration of the
    /// bridge is used.
    /// - The MAC address of the bridge will be set to the Ethernet MAC address but
    /// with the U/L bit set to 1 for a locally administered address.
    Activate = 3,
    /// Deactivates the configuration. After deactivating a bridge
    /// configuration, the network interfaces connected to the bridge must be deactivated
    /// and activated to restore the IP settings.
    Deactivate = 4,
}

#[derive(Clone, PartialEq, AtatEnum)]
pub enum Timing {
    /// 0: Max initial probe delay [ms] (default: 1000 ms)
    #[at_arg(value = 0)]
    InitialProbeDelayMax(u16),
    /// 1: Number of probes (default: 3)
    #[at_arg(value = 1)]
    NumOfProbes(u16),
    /// 2: Minimum probe interval [ms] (default: 1000 ms)
    /// The probe Interval is a random value between minimum probe interval and
    /// minimum probe interval plus 1000. That is, if set to 1000, the delay will be between
    /// one and two seconds for each additional probe request.
    #[at_arg(value = 2)]
    MinimumProbeInterval(u16),
    /// 3: Announce wait time after last probe [ms] (default: 2000 ms)
    #[at_arg(value = 3)]
    AnaunceWaitTime(u16),
    /// 4: Number of announce messages (default: 2)
    #[at_arg(value = 4)]
    NumOfAnnaunceMessages(u16),
    /// 5: Announce interval [ms] (default: 2000 ms)
    #[at_arg(value = 5)]
    AnnounceInterval(u16),
}

#[derive(Debug, Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum ErrorType {
    IpAddressConflict = 128,
}
