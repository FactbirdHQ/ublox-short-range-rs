//! ### 11 - Security Commands
pub mod responses;
pub mod types;

use atat::atat_derive::AtatCmd;
use heapless::{consts, String};
use no_std_net::IpAddr;
use responses::*;
use types::*;

use super::NoResponse;

/// 11.1 SSL/TLS certificates and private keys manager +USECMNG
///
/// Manages the X.509 certificates and private keys with the following functionalities:
/// • Validation and import of certificates and private keys
/// • List and information retrieval of the imported certificates and private keys
/// • Removal of the certificates and private keys
/// • MD5 calculation of the imported certificate or private key
#[derive(Clone, AtatCmd)]
#[at_cmd("+USECMNG=0,", ImportResponse, value_sep = false, timeout_ms = 10000)]
pub struct PrepareSecurityDataImport<'a> {
    /// Type of the security data
    #[at_arg(position = 0)]
    pub data_type: SecurityDataType,
    /// Unique identifier of an imported certificate or private key. If an existing name is
    /// used, the data will be overridden. The maximum length is 32 characters.
    #[at_arg(position = 1, len = 32)]
    pub internal_name: &'a str,
    /// Size in bytes of a certificate or private key being imported. The maximum allowed
    /// size is 8192 bytes.
    #[at_arg(position = 2)]
    pub data_size: usize,
    /// Decryption password; applicable only for PKCS8 encrypted client private keys.
    /// The maximum length is 64 characters.
    #[at_arg(position = 3, len = 64)]
    pub password: Option<&'a str>,
}

#[derive(Clone, AtatCmd)]
#[at_cmd(
    "",
    SecurityDataImport,
    value_sep = false,
    timeout_ms = 10000,
    cmd_prefix = "",
    termination = "",
    force_receive_state = true
)]
pub struct SendSecurityDataImport<'a> {
    #[at_arg(position = 0,  len = 2048)]
    pub data: serde_at::ser::Bytes<'a>,
}

/// 11.1 SSL/TLS certificates and private keys manager +USECMNG
///
/// Manages the X.509 certificates and private keys with the following functionalities:
/// • Validation and import of certificates and private keys
/// • List and information retrieval of the imported certificates and private keys
/// • Removal of the certificates and private keys
/// • MD5 calculation of the imported certificate or private key
#[derive(Clone, AtatCmd)]
#[at_cmd("+USECMNG=2,", NoResponse, value_sep = false, timeout_ms = 10000)]
pub struct RemoveSecurityData<'a> {
    #[at_arg(position = 0)]
    pub types: SecurityDataType,
    #[at_arg(position = 1, len = 32)]
    pub name: &'a str,
}

/// TODO: Implement response for this
/// 11.1 SSL/TLS certificates and private keys manager +USECMNG
///
/// Manages the X.509 certificates and private keys with the following functionalities:
/// • Validation and import of certificates and private keys
/// • List and information retrieval of the imported certificates and private keys
/// • Removal of the certificates and private keys
/// • MD5 calculation of the imported certificate or private key
#[derive(Clone, AtatCmd)]
#[at_cmd("+USECMNG=3,", NoResponse, value_sep = false, timeout_ms = 10000)]
// #[at_cmd("+USECMNG=3,", ListSecurityDataResponse, value_sep = false, timeout_ms = 10000)]
pub struct ListSecurityData {
    // #[at_arg(position = 0)]
    // pub types: SecurityDataType,
}

/// 11.1 SSL/TLS certificates and private keys manager +USECMNG
///
/// Manages the X.509 certificates and private keys with the following functionalities:
/// • Validation and import of certificates and private keys
/// • List and information retrieval of the imported certificates and private keys
/// • Removal of the certificates and private keys
/// • MD5 calculation of the imported certificate or private key
#[derive(Clone, AtatCmd)]
#[at_cmd("+USECMNG=4,", SecurityDataMD5, value_sep = false, timeout_ms = 10000)]
pub struct GetSecurityDataMD5 {
    #[at_arg(position = 0)]
    pub types: SecurityDataType,
    #[at_arg(position = 1)]
    pub name: String<consts::U32>,
}
