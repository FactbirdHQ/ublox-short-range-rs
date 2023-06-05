//! Responses for Security Commands
use super::types::*;
use atat::atat_derive::AtatResp;
use heapless::String;

/// 11.1 SSL/TLS certificates import produces: '>'
#[derive(Clone, PartialEq, AtatResp)]
pub struct ImportResponse {
    /// Type of operation
    #[at_arg(position = 0)]
    pub ch: char,
}

/// 11.1 SSL/TLS certificates and private keys manager +USECMNG
#[derive(Clone, PartialEq, AtatResp)]
pub struct SecurityDataImport {
    /// Type of operation
    #[at_arg(position = 0)]
    pub op_code: SecurityOperation,
    /// Type of the security data
    #[at_arg(position = 1)]
    pub data_type: SecurityDataType,
    /// Unique identifier of an imported certificate or private key. If an existing name is
    /// used, the data will be overridden. The maximum length is 32 characters.
    #[at_arg(position = 2)]
    pub internal_name: String<32>,
    /// MD5 formatted string.
    #[at_arg(position = 3)]
    pub md5_string: String<128>,
}

/// 10.2 Network status +UNSTAT
#[derive(Clone, AtatResp)]
pub struct SecurityDataMD5 {
    /// Type of the security data
    #[at_arg(position = 0)]
    pub data_type: SecurityDataType,
    /// Unique identifier of an imported certificate or private key. If an existing name is
    /// used, the data will be overridden. The maximum length is 32 characters.
    #[at_arg(position = 1)]
    pub internal_name: String<32>,
    /// MD5 formatted string.
    #[at_arg(position = 2)]
    pub md5_string: String<128>,
}
