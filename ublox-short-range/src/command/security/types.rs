//! Argument and parameter types used by Security Commands and Responses

use atat::atat_derive::AtatEnum;
use heapless::{consts, String, Vec};
use no_std_net::{IpAddr, Ipv4Addr, Ipv6Addr};

#[derive(Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum SecurityOperation {
    /// Import a certificate or a private key (data provided by the stream of byte)
    Import = 0,
    /// Remove an imported certificate or private key
    Remove = 2,
    /// List the imported certificates or private keys
    List = 3,
    /// Retrieve the MD5 of an imported certificate or private key
    MD5 = 4,
}

#[derive(Clone, PartialEq, AtatEnum)]
#[repr(u8)]
pub enum SecurityDataType {
    // This is undocumented..
    TrustedRootCA = 0,
    ClientCertificate = 1,
    ClientPrivateKey = 2,
}
