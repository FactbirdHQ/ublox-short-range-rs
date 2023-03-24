#![cfg_attr(all(not(test), not(feature = "std")), no_std)]
#![cfg_attr(feature = "async", allow(incomplete_features))]
#![cfg_attr(feature = "async", feature(generic_const_exprs))]
#![cfg_attr(feature = "async", feature(async_fn_in_trait))]

#[cfg(feature = "async")]
mod asynch;

mod blocking;
mod hex;

pub use atat;

pub mod command;
pub mod config;
pub mod error;
pub mod wifi;

#[cfg(test)]
mod test_helper;

#[cfg(any(feature = "socket-udp", feature = "socket-tcp"))]
pub use blocking::tls::TLS;
