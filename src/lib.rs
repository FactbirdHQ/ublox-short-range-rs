#![no_std]

extern crate heapless;

extern crate at_rs as at;
extern crate nb;
extern crate no_std_net;


#[cfg(test)]
#[macro_use]
mod test_helpers;

mod traits;

pub mod command;
pub mod error;
pub mod prelude;
pub mod socket;
pub mod wifi;

pub type ATClient<T> = at::client::ATClient<T, command::Command, command::ResponseType>;
