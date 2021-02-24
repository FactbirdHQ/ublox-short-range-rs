use crate::command::wifi::types::{OperationMode, ScanedWifiNetwork};
use crate::error::WifiError;
use crate::hex::from_hex;
use atat::serde_at::CharVec;
use heapless::{consts, String};

use core::convert::TryFrom;

#[derive(PartialEq, Debug)]
pub enum WifiMode {
    Station,
    AccessPoint,
}

#[derive(Debug)]
pub struct WifiNetwork {
    pub bssid: CharVec<consts::U20>,
    pub op_mode: OperationMode,
    pub ssid: String<consts::U64>,
    pub channel: u8,
    pub rssi: i32,
    pub authentication_suites: u8,
    pub unicast_ciphers: u8,
    pub group_ciphers: u8,
    pub mode: WifiMode,
}

impl TryFrom<ScanedWifiNetwork> for WifiNetwork {
    type Error = WifiError;

    fn try_from(r: ScanedWifiNetwork) -> Result<Self, Self::Error> {
        Ok(WifiNetwork {
            bssid: r.bssid,
            op_mode: r.op_mode,
            ssid: r.ssid,
            channel: r.channel,
            rssi: r.rssi,
            authentication_suites: from_hex(&mut [r.authentication_suites])
                .map_err(|_| Self::Error::HexError)?[0], //TODO: Better solution
            unicast_ciphers: from_hex(&mut [r.unicast_ciphers])
                .map_err(|_| Self::Error::HexError)?[0],
            group_ciphers: r.group_ciphers,
            mode: WifiMode::Station,
        })
    }
}
