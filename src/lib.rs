#![cfg_attr(all(not(test), not(feature = "std")), no_std)]
#![allow(async_fn_in_trait)]

mod fmt;

pub mod asynch;

pub use embedded_nal_async;

pub use ublox_sockets;

mod connection;
mod network;
mod peer_builder;

// mod blocking;
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
// - (Blocking client?)
// -
//
// FIXME:
// - PWR/Restart stuff doesn't fully work
// -
