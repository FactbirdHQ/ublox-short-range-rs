#![no_std]

extern crate heapless;

#[macro_use]
extern crate nb;
extern crate no_std_net;
extern crate packed_struct;
#[macro_use]
extern crate packed_struct_codegen;

// pub type ATClient<T> = at::client::ATClient<
//     T,
//     command::RequestType,
//     heapless::consts::U5,
//     heapless::consts::U5,
// >;

#[cfg(test)]
#[macro_use]
mod test_helpers;

mod client;
mod hex;

pub use client::UbloxClient;

mod traits;

pub mod command;
pub mod error;
pub mod prelude;
pub mod socket;
pub mod wifi;
