//! Argument and parameter types used by General Commands and Responses

use core::fmt::Write;

use atat::atat_derive::AtatEnum;
use serde::{Deserialize, Deserializer, Serialize};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirmwareVersion {
    major: u8,
    minor: u8,
    patch: u8,
    meta: Option<heapless::String<5>>,
}

impl FirmwareVersion {
    pub fn new(major: u8, minor: u8, patch: u8) -> Self {
        Self {
            major,
            minor,
            patch,
            meta: None,
        }
    }
}

impl PartialOrd for FirmwareVersion {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        match self.major.partial_cmp(&other.major) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        match self.minor.partial_cmp(&other.minor) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        match self.patch.partial_cmp(&other.patch) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.meta.partial_cmp(&other.meta)
    }
}

pub struct DeserializeError;

impl core::fmt::Display for DeserializeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Failed to deserialize version")
    }
}

impl core::str::FromStr for FirmwareVersion {
    type Err = DeserializeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut iter = s.splitn(3, '.');
        let major = iter
            .next()
            .and_then(|s| s.parse().ok())
            .ok_or(DeserializeError)?;
        let minor = iter
            .next()
            .and_then(|s| s.parse().ok())
            .ok_or(DeserializeError)?;
        let patch_meta = iter.next().ok_or(DeserializeError)?;

        let (patch, meta) = match patch_meta.split_once('-') {
            Some((patch_str, meta)) => (
                patch_str.parse().map_err(|_| DeserializeError)?,
                Some(heapless::String::from(meta)),
            ),
            None => (patch_meta.parse().map_err(|_| DeserializeError)?, None),
        };

        Ok(Self {
            major,
            minor,
            patch,
            meta,
        })
    }
}

impl defmt::Format for FirmwareVersion {
    fn format(&self, fmt: defmt::Formatter) {
        if let Some(meta) = &self.meta {
            defmt::write!(fmt, "{}.{}.{}-{}", self.major, self.minor, self.patch, meta)
        } else {
            defmt::write!(fmt, "{}.{}.{}", self.major, self.minor, self.patch,)
        }
    }
}

impl Serialize for FirmwareVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut str = heapless::String::<64>::new();
        if let Some(meta) = &self.meta {
            str.write_fmt(format_args!(
                "{}.{}.{}-{}",
                self.major, self.minor, self.patch, meta
            ))
            .map_err(serde::ser::Error::custom)?;
        } else {
            str.write_fmt(format_args!("{}.{}.{}", self.major, self.minor, self.patch))
                .map_err(serde::ser::Error::custom)?;
        }
        serializer.serialize_str(&str)
    }
}

impl<'de> Deserialize<'de> for FirmwareVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = heapless::String::<64>::deserialize(deserializer)?;
        core::str::FromStr::from_str(&s).map_err(serde::de::Error::custom)
    }
}
