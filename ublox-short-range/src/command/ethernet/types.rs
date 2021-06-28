//! Argument and parameter types used by Ethernet Commands and Responses

use atat::atat_derive::AtatEnum;
use embedded_nal::Ipv4Addr;

#[derive(Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum OnOff {
    Off = 0,
    On = 1,
}

#[derive(Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum EthernetConfigParameter {
    /// <param_val> decides if the network is active on start up.
    /// • 0 (default): Inactive
    /// • 1: active
    ActiveOnStartup = 0,
    /// <param_val> Phy support mode
    /// • 0: disabled
    /// • 1 (default): enabled
    /// Not available for ODIN-W2 software versions 2.0.0 or 2.0.1. Default PHY values will be used.
    PhySupport = 1,
    /// <param_val> Ethernet speed
    /// • 0 (default): 100 Mbit/s
    /// • 1: 10 Mbit/s
    /// Not available for ODIN-W2 software versions 2.0.0 or 2.0.1. Default PHY values will be used.
    Speed = 2,
    /// <param_val> Ethernet Duplex mode
    /// • 0 (default): Full duplex
    /// • 1: Half duplex
    /// Not available for ODIN-W2 software versions 2.0.0 or 2.0.1. Default PHY values will be used.
    DuplexMode = 3,
    /// <param_val> Auto-negotiation (of speed and duplex mode)
    /// • 0: disabled
    /// • 1 (default): enabled
    /// Not available for ODIN-W2 software versions 2.0.0 or 2.0.1. Default PHY values will be used.
    AutoNegotiation = 4,
    /// <param_val> is the Phy address. The factory default value is 0x3 (for ODIN) and 0x0
    /// (for NINA-W13 and NINA-W15).
    PhyAddress = 5,
    /// IPv4 Mode - <param_val1> to set the way to retrieve an IP address
    /// • 1 (default): Static
    /// • 2: DHCP
    IPv4Mode = 100,
    /// <param_val> is the IPv4 address. The factory default value is 0.0.0.0
    IPv4Address = 101,
    /// <param_val> is the subnet mask. The factory default value is 0.0.0.0
    SubnetMask = 102,
    /// <param_val> is the default gateway. The factory default value is 0.0.0.0
    DefaultGateway = 103,
    /// <param_val> is the primary DNS server IP address. The factory default value is 0
    /// .0.0.0
    PrimaryDNS = 104,
    /// <param_val> is the secondary DNS server IP address. The factory default value is
    /// 0.0.0.0
    SecondaryDNS = 105,
    /// Address conflict detection. The factory default value is 0 (disabled). This tag is
    /// supported by ODIN-W2 from software version 6.0.0 onwards only.
    /// • 0: Disabled
    /// • 1: Enabled
    AddressConflictDetection = 107,
}

#[derive(Clone, PartialEq, AtatEnum)]
pub enum EthernetConfig {
    /// <param_val> decides if the network is active on start up.
    /// • 0 (default): Inactive
    /// • 1: active
    #[at_arg(value = 0)]
    ActiveOnStartup(OnOff),
    /// <param_val> Phy support mode
    /// • 0: disabled
    /// • 1 (default): enabled
    /// Not available for ODIN-W2 software versions 2.0.0 or 2.0.1. Default PHY values will be used.
    #[at_arg(value = 1)]
    PhySupport(OnOff),
    /// <param_val> Ethernet speed
    /// • 0 (default): 100 Mbit/s
    /// • 1: 10 Mbit/s
    /// Not available for ODIN-W2 software versions 2.0.0 or 2.0.1. Default PHY values will be used.
    #[at_arg(value = 2)]
    Speed(EthernetSpeed),
    /// <param_val> Ethernet Duplex mode
    /// • 0 (default): Full duplex
    /// • 1: Half duplex
    /// Not available for ODIN-W2 software versions 2.0.0 or 2.0.1. Default PHY values will be used.
    #[at_arg(value = 3)]
    DuplexMode(EthernetDuplexMode),
    /// <param_val> Auto-negotiation (of speed and duplex mode)
    /// • 0: disabled
    /// • 1 (default): enabled
    /// Not available for ODIN-W2 software versions 2.0.0 or 2.0.1. Default PHY values will be used.
    #[at_arg(value = 4)]
    AutoNegotiation(OnOff),
    /// <param_val> is the Phy address. The factory default value is 0x3 (for ODIN) and 0x0
    /// (for NINA-W13 and NINA-W15).
    #[at_arg(value = 5)]
    PhyAddress(u32),
    /// IPv4 Mode - <param_val1> to set the way to retrieve an IP address
    /// • 1 (default): Static
    /// • 2: DHCP
    #[at_arg(value = 100)]
    IPv4Mode(IPv4Mode),
    /// <param_val> is the IPv4 address. The factory default value is 0.0.0.0
    #[at_arg(value = 101)]
    IPv4Address(#[at_arg(len = 16)] Ipv4Addr),
    /// <param_val> is the subnet mask. The factory default value is 0.0.0.0
    #[at_arg(value = 102)]
    SubnetMask(#[at_arg(len = 16)] Ipv4Addr),
    /// <param_val> is the default gateway. The factory default value is 0.0.0.0
    #[at_arg(value = 103)]
    DefaultGateway(#[at_arg(len = 16)] Ipv4Addr),
    /// <param_val> is the primary DNS server IP address. The factory default value is 0
    /// .0.0.0
    #[at_arg(value = 104)]
    PrimaryDNS(#[at_arg(len = 16)] Ipv4Addr),
    /// <param_val> is the secondary DNS server IP address. The factory default value is
    /// 0.0.0.0
    #[at_arg(value = 105)]
    SecondaryDNS(#[at_arg(len = 16)] Ipv4Addr),
    /// Address conflict detection. The factory default value is 0 (disabled). This tag is
    /// supported by ODIN-W2 from software version 6.0.0 onwards only.
    /// • 0: Disabled
    /// • 1: Enabled
    #[at_arg(value = 107)]
    AddressConflictDetection(OnOff),
}

#[derive(Clone, PartialEq, AtatEnum)]
pub enum EthernetConfigR {
    /// <param_val> decides if the network is active on start up.
    /// • 0 (default): Inactive
    /// • 1: active
    #[at_arg(value = 0)]
    ActiveOnStartup(OnOff),
    /// <param_val> Phy support mode
    /// • 0: disabled
    /// • 1 (default): enabled
    /// Not available for ODIN-W2 software versions 2.0.0 or 2.0.1. Default PHY values will be used.
    #[at_arg(value = 1)]
    PhySupport(OnOff),
    /// <param_val> Ethernet speed
    /// • 0 (default): 100 Mbit/s
    /// • 1: 10 Mbit/s
    /// Not available for ODIN-W2 software versions 2.0.0 or 2.0.1. Default PHY values will be used.
    #[at_arg(value = 2)]
    Speed(EthernetSpeed),
    /// <param_val> Ethernet Duplex mode
    /// • 0 (default): Full duplex
    /// • 1: Half duplex
    /// Not available for ODIN-W2 software versions 2.0.0 or 2.0.1. Default PHY values will be used.
    #[at_arg(value = 3)]
    DuplexMode(EthernetDuplexMode),
    /// <param_val> Auto-negotiation (of speed and duplex mode)
    /// • 0: disabled
    /// • 1 (default): enabled
    /// Not available for ODIN-W2 software versions 2.0.0 or 2.0.1. Default PHY values will be used.
    #[at_arg(value = 4)]
    AutoNegotiation(OnOff),
    /// <param_val> is the Phy address. The factory default value is 0x3 (for ODIN) and 0x0
    /// (for NINA-W13 and NINA-W15).
    #[at_arg(value = 5)]
    PhyAddress(u32),
    /// IPv4 Mode - <param_val1> to set the way to retrieve an IP address
    /// • 1 (default): Static
    /// • 2: DHCP
    #[at_arg(value = 100)]
    IPv4Mode(IPv4Mode),
    /// <param_val> is the IPv4 address. The factory default value is 0.0.0.0
    #[at_arg(value = 101)]
    IPv4Address(#[at_arg(len = 16)] Ipv4Addr),
    /// <param_val> is the subnet mask. The factory default value is 0.0.0.0
    #[at_arg(value = 102)]
    SubnetMask(#[at_arg(len = 16)] Ipv4Addr),
    /// <param_val> is the default gateway. The factory default value is 0.0.0.0
    #[at_arg(value = 103)]
    DefaultGateway(#[at_arg(len = 16)] Ipv4Addr),
    /// <param_val> is the primary DNS server IP address. The factory default value is 0
    /// .0.0.0
    #[at_arg(value = 104)]
    PrimaryDNS(#[at_arg(len = 16)] Ipv4Addr),
    /// <param_val> is the secondary DNS server IP address. The factory default value is
    /// 0.0.0.0
    #[at_arg(value = 105)]
    SecondaryDNS(#[at_arg(len = 16)] Ipv4Addr),
    /// Address conflict detection. The factory default value is 0 (disabled). This tag is
    /// supported by ODIN-W2 from software version 6.0.0 onwards only.
    /// • 0: Disabled
    /// • 1: Enabled
    #[at_arg(value = 107)]
    AddressConflictDetection(OnOff),
}

#[derive(Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum EthernetSpeed {
    Mbps10 = 1,
    Mbps100 = 0,
}

#[derive(Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum EthernetDuplexMode {
    FullDuplex = 0,
    HalfDuplex = 1,
}

#[derive(Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum IPv4Mode {
    Static = 1,
    DHCP = 2,
}

#[derive(Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum EthernetConfigAction {
    /// Reset; it clears the specified profile, resetting all the parameters to their factory
    /// programmed values
    Reset = 0,
    /// Store; it saves all the current parameters
    Store = 1,
    /// Load: it reads all the parameters
    Load = 2,
    /// Activate; it activates the Ethernet, using the current parameters.
    Activate = 3,
    /// Deactivate; it deactivates the Ethernet.
    Deactivate = 4,
}
