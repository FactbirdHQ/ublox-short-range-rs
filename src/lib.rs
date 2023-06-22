#![cfg_attr(all(not(test), not(feature = "std")), no_std)]
#![cfg_attr(feature = "async", allow(incomplete_features))]
#![cfg_attr(feature = "async", feature(generic_const_exprs))]
#![cfg_attr(feature = "async", feature(async_fn_in_trait))]
#![cfg_attr(feature = "async", feature(impl_trait_projections))]
#![cfg_attr(feature = "async", feature(type_alias_impl_trait))]

#[cfg(feature = "async")]
pub mod asynch;

#[cfg(feature = "async")]
pub use embedded_nal_async;

mod connection;
mod network;
mod peer_builder;

// mod blocking;
mod hex;

pub use atat;

pub mod command;
pub mod error;
// pub mod wifi;
