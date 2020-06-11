use crate::command::{
    wifi::types::{
        OperationMode,
        Authentication,
        ScanedWifiNetwork}
    };
use crate::error::WifiError;
use crate::hex::from_hex;
use heapless::{String, consts};

use core::convert::TryFrom;

#[derive(PartialEq, Debug)]
pub enum WifiMode {
    Station,
    AccessPoint,
}

#[derive(Debug)]
pub struct WifiNetwork {
    pub bssid: String<consts::U64>,
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
        if let ScanedWifiNetwork {
            bssid,
            op_mode,
            ssid,
            channel,
            rssi,
            authentication_suites,
            unicast_ciphers,
            group_ciphers,
        } = r
        {
            Ok(WifiNetwork {
                bssid,
                op_mode,
                ssid,
                channel,
                rssi,
                authentication_suites: from_hex(&mut [authentication_suites]).map_err(|_| Self::Error::HexError)?[0], //TODO: Better solution
                unicast_ciphers: from_hex(&mut [unicast_ciphers]).map_err(|_| Self::Error::HexError)?[0],
                group_ciphers,
                mode: WifiMode::Station,
            })
        } else {
            Err(WifiError::UnexpectedResponse)
        }
    }
}
