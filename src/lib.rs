#![cfg_attr(not(test), no_std)]
#![allow(async_fn_in_trait)]

#[cfg(all(feature = "ppp", feature = "internal-network-stack"))]
compile_error!("You may not enable both `ppp` and `internal-network-stack` features.");

#[cfg(not(any(
    feature = "odin_w2xx",
    feature = "nina_w1xx",
    feature = "nina_b1xx",
    feature = "anna_b1xx",
    feature = "nina_b2xx",
    feature = "nina_b3xx"
)))]
compile_error!("No chip feature activated. You must activate exactly one of the following features: odin_w2xx, nina_w1xx, nina_b1xx, anna_b1xx, nina_b2xx, nina_b3xx");

mod fmt;

pub mod asynch;

mod config;
mod connection;
mod network;

mod hex;

pub use atat;

pub mod command;
pub mod error;
pub use config::WifiConfig;
