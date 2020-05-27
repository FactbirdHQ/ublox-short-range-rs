//! Argument and parameter types used by GPIO Commands and Responses

use serde_repr::{Deserialize_repr, Serialize_repr};
use ufmt::derive::uDebug;
use no_std_net::{IpAddr, Ipv4Addr, Ipv6Addr};
use heapless::{consts, String};

#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum GPIOId{
    /// LPO_CLK
    C16 = 14,
    /// UART_RTS
    A12 = 28,
    /// UART_CTS
    A10 = 27,
    /// RMII_MDC
    C14 = 12,
    /// RMII_MDIO
    C15 = 13,
    /// RMII_TXD0
    D1 = 20,
    /// RMII_TXD1
    D2 = 21,
    /// RMII_TX-EN
    D3 = 22,
    /// RMII_CRS-DV
    D4 = 23,
    /// RMII_RXD0
    D5 = 24,
    /// RMII_RXD1
    D6 = 25,
    /// RMII_REF-CLK
    D8 = 26,
    /// UART3 RX
    A14 = 7,
    /// UART3 TX
    A15 = 8,
    /// UART3 CTS
    A16 = 9,
    /// UART3 RTS
    A17 = 10,
    /// RSVD
    C13 = 11,
    /// RSVD
    C5 = 15,
    /// SPI MISO / SDIO D0
    C6 = 16,
    /// SPI/SCK/SDIO CLK
    C8 = 17,
    /// SPI MOSI / SDIO CMD
    C10 = 18,
    /// SPI SEL
    C11 = 19,
    /// SDIO CD
    C12 = 29,
}

#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
pub enum GPIOMode{
    #[at_arg(value = 0)]
    Output(GPIOOutputConfig),
    #[at_arg(value = 1)]
    Input(GPIOInputConfig),
    /// Default
    #[at_arg(value = 255)]
    Disabled,
}

#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum GPIOOutputConfig{
    /// Default
    Low = 0,
    High = 1,
}

#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum GPIOInputConfig{
    /// Default
    /// (default value) No resistor activated
    NoPull = 0,
    /// Pull up resistor active
    PullUp = 1,
    /// Pull down resistor active
    PullDown = 2,
}

#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum GPIOValue{
    Low = 0,
    High = 1,
}


