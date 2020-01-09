#![no_std]

extern crate heapless;

extern crate at_rs as at;
#[macro_use]
extern crate nb;
extern crate no_std_net;
// extern crate packed_struct;
// #[macro_use]
// extern crate packed_struct_codegen;

pub type ATClient<T> = at::client::ATClient<
    T,
    command::Command,
    command::ResponseType,
    heapless::consts::U10,
    heapless::consts::U10,
>;

#[cfg(test)]
#[macro_use]
mod test_helpers;

mod client;

pub use client::UbloxClient;

mod traits;

pub mod command;
pub mod error;
pub mod prelude;
pub mod socket;
pub mod wifi;
