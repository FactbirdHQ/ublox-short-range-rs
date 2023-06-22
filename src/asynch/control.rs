use core::future::poll_fn;
use core::task::Poll;

use atat::asynch::AtatClient;
use embassy_time::{with_timeout, Duration};

use crate::command::gpio::{
    types::{GPIOId, GPIOValue},
    WriteGPIO,
};
use crate::command::network::SetNetworkHostName;
use crate::command::wifi::types::{
    Authentication, StatusId, WifiStationAction, WifiStationConfig, WifiStatus, WifiStatusVal,
};
use crate::command::wifi::{ExecWifiStationAction, GetWifiStatus, SetWifiStationConfig};
use crate::command::OnOff;
use crate::error::Error;

use super::state::LinkState;
use super::{state, AtHandle};

const CONFIG_ID: u8 = 0;

pub struct Control<'a, AT: AtatClient> {
    state_ch: state::StateRunner<'a>,
    at: AtHandle<'a, AT>,
}

impl<'a, AT: AtatClient> Control<'a, AT> {
    pub(crate) fn new(state_ch: state::StateRunner<'a>, at: AtHandle<'a, AT>) -> Self {
        Self { state_ch, at }
    }

    pub(crate) async fn init(&mut self) -> Result<(), Error> {
        defmt::debug!("Initalizing ublox control");
        // read MAC addr.
        // let mut resp = self.at.send_edm(GetWifiMac).await?;
        // self.state_ch.set_ethernet_address(
        //     hex::from_hex(resp.mac_addr.as_mut_slice())
        //         .unwrap()
        //         .try_into()
        //         .unwrap(),
        // );

        // let country = countries::WORLD_WIDE_XX;
        // let country_info = CountryInfo {
        //     country_abbrev: [country.code[0], country.code[1], 0, 0],
        //     country_code: [country.code[0], country.code[1], 0, 0],
        //     rev: if country.rev == 0 {
        //         -1
        //     } else {
        //         country.rev as _
        //     },
        // };
        // self.set_iovar("country", &country_info.to_bytes()).await;

        // // set country takes some time, next ioctls fail if we don't wait.
        // Timer::after(Duration::from_millis(100)).await;

        // // Set antenna to chip antenna
        // self.ioctl_set_u32(IOCTL_CMD_ANTDIV, 0, 0).await;

        // self.set_iovar_u32("bus:txglom", 0).await;
        // Timer::after(Duration::from_millis(100)).await;
        // //self.set_iovar_u32("apsta", 1).await; // this crashes, also we already did it before...??
        // //Timer::after(Duration::from_millis(100)).await;
        // self.set_iovar_u32("ampdu_ba_wsize", 8).await;
        // Timer::after(Duration::from_millis(100)).await;
        // self.set_iovar_u32("ampdu_mpdu", 4).await;
        // Timer::after(Duration::from_millis(100)).await;
        // //self.set_iovar_u32("ampdu_rx_factor", 0).await; // this crashes

        // // set wifi up
        // self.ioctl(ControlType::Set, IOCTL_CMD_UP, 0, &mut []).await;

        // Timer::after(Duration::from_millis(100)).await;

        // self.ioctl_set_u32(110, 0, 1).await; // SET_GMODE = auto
        // self.ioctl_set_u32(142, 0, 0).await; // SET_BAND = any

        Ok(())
    }

    pub async fn set_hostname(&mut self, hostname: &str) -> Result<(), Error> {
        self.at
            .send_edm(SetNetworkHostName {
                host_name: hostname,
            })
            .await?;
        Ok(())
    }

    async fn get_wifi_status(&mut self) -> Result<WifiStatusVal, Error> {
        match self
            .at
            .send_edm(GetWifiStatus {
                status_id: StatusId::Status,
            })
            .await?
            .status_id
        {
            WifiStatus::Status(s) => Ok(s),
            _ => Err(Error::AT(atat::Error::InvalidResponse)),
        }
    }

    async fn get_connected_ssid(&mut self) -> Result<heapless::String<64>, Error> {
        match self
            .at
            .send_edm(GetWifiStatus {
                status_id: StatusId::SSID,
            })
            .await?
            .status_id
        {
            WifiStatus::SSID(s) => Ok(s),
            _ => Err(Error::AT(atat::Error::InvalidResponse)),
        }
    }

    pub async fn join_open(&mut self, ssid: &str) -> Result<(), Error> {
        if matches!(self.get_wifi_status().await?, WifiStatusVal::Connected) {
            // Wifi already connected. Check if the SSID is the same
            let current_ssid = self.get_connected_ssid().await?;
            if current_ssid.as_str() == ssid {
                return Ok(());
            } else {
                self.disconnect().await?;
            };
        }

        self.at
            .send_edm(SetWifiStationConfig {
                config_id: CONFIG_ID,
                config_param: WifiStationConfig::ActiveOnStartup(OnOff::Off),
            })
            .await?;

        self.at
            .send_edm(SetWifiStationConfig {
                config_id: CONFIG_ID,
                config_param: WifiStationConfig::SSID(heapless::String::from(ssid)),
            })
            .await?;

        self.at
            .send_edm(SetWifiStationConfig {
                config_id: CONFIG_ID,
                config_param: WifiStationConfig::Authentication(Authentication::Open),
            })
            .await?;

        self.at
            .send_edm(ExecWifiStationAction {
                config_id: CONFIG_ID,
                action: WifiStationAction::Activate,
            })
            .await?;

        with_timeout(Duration::from_secs(10), self.wait_for_join(ssid))
            .await
            .map_err(|_| Error::Timeout)??;

        Ok(())
    }

    pub async fn join_wpa2(&mut self, ssid: &str, passphrase: &str) -> Result<(), Error> {
        if matches!(self.get_wifi_status().await?, WifiStatusVal::Connected) {
            // Wifi already connected. Check if the SSID is the same
            let current_ssid = self.get_connected_ssid().await?;
            if current_ssid.as_str() == ssid {
                return Ok(());
            } else {
                self.disconnect().await?;
            };
        }

        self.at
            .send_edm(SetWifiStationConfig {
                config_id: CONFIG_ID,
                config_param: WifiStationConfig::ActiveOnStartup(OnOff::Off),
            })
            .await?;

        self.at
            .send_edm(SetWifiStationConfig {
                config_id: CONFIG_ID,
                config_param: WifiStationConfig::SSID(heapless::String::from(ssid)),
            })
            .await?;

        self.at
            .send_edm(SetWifiStationConfig {
                config_id: CONFIG_ID,
                config_param: WifiStationConfig::Authentication(Authentication::WpaWpa2Psk),
            })
            .await?;

        self.at
            .send_edm(SetWifiStationConfig {
                config_id: CONFIG_ID,
                config_param: WifiStationConfig::WpaPskOrPassphrase(heapless::String::from(
                    passphrase,
                )),
            })
            .await?;

        self.at
            .send_edm(ExecWifiStationAction {
                config_id: CONFIG_ID,
                action: WifiStationAction::Activate,
            })
            .await?;

        with_timeout(Duration::from_secs(10), self.wait_for_join(ssid))
            .await
            .map_err(|_| Error::Timeout)??;

        Ok(())
    }

    pub async fn disconnect(&mut self) -> Result<(), Error> {
        match self.get_wifi_status().await? {
            WifiStatusVal::Disabled => {}
            WifiStatusVal::Disconnected | WifiStatusVal::Connected => {
                self.at
                    .send_edm(ExecWifiStationAction {
                        config_id: CONFIG_ID,
                        action: WifiStationAction::Deactivate,
                    })
                    .await?;
            }
        }

        let wait_for_disconnect = poll_fn(|cx| match self.state_ch.link_state(cx) {
            LinkState::Up => Poll::Pending,
            LinkState::Down => Poll::Ready(()),
        });

        with_timeout(Duration::from_secs(10), wait_for_disconnect)
            .await
            .map_err(|_| Error::Timeout)?;

        Ok(())
    }

    async fn wait_for_join(&mut self, ssid: &str) -> Result<(), Error> {
        poll_fn(|cx| match self.state_ch.link_state(cx) {
            LinkState::Down => Poll::Pending,
            LinkState::Up => Poll::Ready(()),
        })
        .await;

        // Check that SSID matches
        let current_ssid = self.get_connected_ssid().await?;
        if ssid != current_ssid.as_str() {
            return Err(Error::Network);
        }

        Ok(())
    }

    pub async fn gpio_set(&mut self, id: GPIOId, value: GPIOValue) -> Result<(), Error> {
        self.at.send_edm(WriteGPIO { id, value }).await?;
        Ok(())
    }
}
