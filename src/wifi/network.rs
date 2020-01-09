use crate::command::*;
use crate::error::WifiError;
use heapless::String;

use core::convert::TryFrom;

#[derive(PartialEq, Debug)]
pub enum WifiMode {
    Station,
    AccessPoint,
}

#[derive(Debug)]
pub struct WifiNetwork {
    pub bssid: String<at::MaxCommandLen>,
    pub op_mode: OPMode,
    pub ssid: String<at::MaxCommandLen>,
    pub channel: u8,
    pub rssi: i16,
    pub authentication_suites: u8,
    pub unicast_ciphers: u8,
    pub group_ciphers: u8,
    pub mode: WifiMode,
}

impl TryFrom<Response> for WifiNetwork {
    type Error = WifiError;

    fn try_from(r: Response) -> Result<Self, Self::Error> {
        if let Response::STAScan {
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
                authentication_suites,
                unicast_ciphers,
                group_ciphers,
                mode: WifiMode::Station,
            })
        } else {
            Err(WifiError::UnexpectedResponse)
        }
    }
}
