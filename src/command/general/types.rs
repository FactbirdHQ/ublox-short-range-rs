//! Argument and parameter types used by GPIO Commands and Responses

use serde_repr::{Deserialize_repr, Serialize_repr};
use ufmt::derive::uDebug;

/// Identification information command value
#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum IdentificationInfoEnum {
    /// Type code
    TypeCode = 0,
    /// Complete software version information
    SoftwareVersion = 9,
    /// MCU ID
    MCUID = 10,
}
