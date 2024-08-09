#![cfg_attr(not(test), no_std)]
#![allow(async_fn_in_trait)]

#[cfg(all(feature = "ppp", feature = "internal-network-stack"))]
compile_error!("You may not enable both `ppp` and `internal-network-stack` features.");

#[cfg(not(any(
    feature = "odin-w2xx",
    feature = "nina-w1xx",
    feature = "nina-b1xx",
    feature = "anna-b1xx",
    feature = "nina-b2xx",
    feature = "nina-b3xx"
)))]
compile_error!("No module feature activated. You must activate exactly one of the following features: odin-w2xx, nina-w1xx, nina-b1xx, anna-b1xx, nina-b2xx, nina-b3xx");

mod fmt;

pub mod asynch;
pub mod options;

mod config;
mod connection;
mod network;

mod hex;

pub use atat;

pub mod command;
pub mod error;
pub use config::{Transport, WifiConfig};

use command::system::types::BaudRate;
pub const DEFAULT_BAUD_RATE: BaudRate = BaudRate::B115200;
