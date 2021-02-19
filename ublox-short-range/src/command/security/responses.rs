//! Responses for Security Commands
use super::types::*;
use atat::atat_derive::AtatResp;
use heapless::{consts, String};

/// 11.1 SSL/TLS certificates import produces: '>'
#[derive(Clone, PartialEq, AtatResp)]
pub struct ImportResponse {
    /// Type of operation
    #[at_arg(position = 0)]
    ch: char,
}

/// 11.1 SSL/TLS certificates and private keys manager +USECMNG
#[derive(Clone, PartialEq, AtatResp)]
pub struct SecurityDataImport {
    /// Type of operation
    #[at_arg(position = 0)]
    op_code: SecurityOperation,
    /// Type of the security data
    #[at_arg(position = 1)]
    data_type: SecurityDataType,
    /// Unique identifier of an imported certificate or private key. If an existing name is
    /// used, the data will be overridden. The maximum length is 32 characters.
    #[at_arg(position = 2)]
    internal_name: String<consts::U32>,
    /// MD5 formatted string.
    #[at_arg(position = 3)]
    md5_string: String<consts::U128>,
}

/// 10.2 Network status +UNSTAT
#[derive(Clone, AtatResp)]
pub struct SecurityDataMD5 {
    /// Type of the security data
    #[at_arg(position = 0)]
    data_type: SecurityDataType,
    /// Unique identifier of an imported certificate or private key. If an existing name is
    /// used, the data will be overridden. The maximum length is 32 characters.
    #[at_arg(position = 1)]
    internal_name: String<consts::U32>,
    /// MD5 formatted string.
    #[at_arg(position = 2)]
    md5_string: String<consts::U128>,
}
