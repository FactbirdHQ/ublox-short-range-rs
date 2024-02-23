#![cfg_attr(not(test), no_std)]
#![allow(async_fn_in_trait)]

#[cfg(all(feature = "ppp", feature = "internal-network-stack"))]
compile_error!("You may not enable both `ppp` and `internal-network-stack` features.");

mod fmt;

pub mod asynch;

mod connection;
mod network;
mod peer_builder;

mod hex;

pub use atat;

pub mod command;
pub mod error;
// pub mod wifi;
pub use peer_builder::SecurityCredentials;

// TODO:
// - UDP stack
// - Secure sockets
// - Network scan
// - AP Mode (control)
// - TCP listener stack
// -
//
// FIXME:
// - PWR/Restart stuff doesn't fully work
// -
