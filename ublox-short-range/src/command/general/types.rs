//! Argument and parameter types used by General Commands and Responses

use atat::atat_derive::AtatEnum;

#[derive(Clone, PartialEq, AtatEnum)]
pub enum GreetingTextMode<'a> {
    /// Turn off the greeting text
    #[at_arg(value = 0)]
    Off,
    /// Turn on the greeting text
    #[at_arg(value = 1)]
    On(#[at_arg(len = 48)] Option<&'a str>),
}

/// Identification information command value
#[derive(Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum IdentificationInfoEnum {
    /// Type code
    TypeCode = 0,
    /// Complete software version information
    SoftwareVersion = 9,
    /// MCU ID
    MCUID = 10,
}
