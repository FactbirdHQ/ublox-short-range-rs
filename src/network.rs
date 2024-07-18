#![allow(dead_code)]

use crate::command::wifi::types::{OperationMode, ScannedWifiNetwork};
use crate::error::WifiError;
use crate::hex::from_hex;
use atat::heapless_bytes::Bytes;
use heapless::String;

use core::convert::TryFrom;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum WifiMode {
    Station,
    AccessPoint,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct WifiNetwork {
    #[cfg_attr(feature = "defmt", defmt(Debug2Format))]
    pub bssid: Bytes<20>,
    pub op_mode: OperationMode,
    pub ssid: String<64>,
    pub channel: u8,
    pub rssi: i32,
    pub authentication_suites: u8,
    pub unicast_ciphers: u8,
    pub group_ciphers: u8,
    pub mode: WifiMode,
}

impl WifiNetwork {
    pub fn new_station(bssid: Bytes<20>, channel: u8) -> Self {
        Self {
            bssid,
            op_mode: OperationMode::Infrastructure,
            ssid: String::new(),
            channel,
            rssi: 1,
            authentication_suites: 0,
            unicast_ciphers: 0,
            group_ciphers: 0,
            mode: WifiMode::Station,
        }
    }
}

impl TryFrom<ScannedWifiNetwork> for WifiNetwork {
    type Error = WifiError;

    fn try_from(r: ScannedWifiNetwork) -> Result<Self, Self::Error> {
        Ok(WifiNetwork {
            bssid: r.bssid,
            op_mode: r.op_mode,
            ssid: r.ssid,
            channel: r.channel,
            rssi: r.rssi,
            authentication_suites: from_hex(&mut [r.authentication_suites])
                .map_err(|_| Self::Error::HexError)?[0], // TODO: Better solution
            unicast_ciphers: from_hex(&mut [r.unicast_ciphers])
                .map_err(|_| Self::Error::HexError)?[0],
            group_ciphers: r.group_ciphers,
            mode: WifiMode::Station,
        })
    }
}
