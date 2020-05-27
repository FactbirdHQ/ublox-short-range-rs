//! Argument and parameter types used by GPIO Commands and Responses

use serde_repr::{Deserialize_repr, Serialize_repr};
use ufmt::derive::uDebug;
use no_std_net::{IpAddr, Ipv4Addr, Ipv6Addr};
use heapless::{consts, String};


#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum SecurityOperation{
    /// Import a certificate or a private key (data provided by the stream of byte)
    Import = 0,
    /// Remove an imported certificate or private key
    Remove = 2,
    /// List the imported certificates or private keys
    List = 3,
    /// Retrieve the MD5 of an imported certificate or private key
    MD5 = 4,
}

#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum SecurityDataType{
    ClientCertificate = 1,
    ClientPrivateKey = 2,
}

