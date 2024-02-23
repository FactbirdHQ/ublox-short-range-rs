use core::future::poll_fn;
use core::task::Poll;

use atat::asynch::AtatClient;
use embassy_time::{with_timeout, Duration};

use crate::command::network::SetNetworkHostName;
use crate::command::security::types::SecurityDataType;
use crate::command::security::SendSecurityDataImport;
use crate::command::wifi::types::{
    Authentication, StatusId, WifiStationAction, WifiStationConfig, WifiStatus, WifiStatusVal,
};
use crate::command::wifi::{ExecWifiStationAction, GetWifiStatus, SetWifiStationConfig};
use crate::command::OnOff;
use crate::command::{
    gpio::{
        types::{GPIOId, GPIOValue},
        WriteGPIO,
    },
    security::PrepareSecurityDataImport,
};
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

    pub async fn set_hostname(&mut self, hostname: &str) -> Result<(), Error> {
        self.at
            .send(SetNetworkHostName {
                host_name: hostname,
            })
            .await?;
        Ok(())
    }

    async fn get_wifi_status(&mut self) -> Result<WifiStatusVal, Error> {
        match self
            .at
            .send(GetWifiStatus {
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
            .send(GetWifiStatus {
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
            .send(SetWifiStationConfig {
                config_id: CONFIG_ID,
                config_param: WifiStationConfig::ActiveOnStartup(OnOff::Off),
            })
            .await?;

        self.at
            .send(SetWifiStationConfig {
                config_id: CONFIG_ID,
                config_param: WifiStationConfig::SSID(
                    heapless::String::try_from(ssid).map_err(|_| Error::Overflow)?,
                ),
            })
            .await?;

        self.at
            .send(SetWifiStationConfig {
                config_id: CONFIG_ID,
                config_param: WifiStationConfig::Authentication(Authentication::Open),
            })
            .await?;

        self.at
            .send(ExecWifiStationAction {
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
            .send(ExecWifiStationAction {
                config_id: CONFIG_ID,
                action: WifiStationAction::Reset,
            })
            .await?;

        self.at
            .send(SetWifiStationConfig {
                config_id: CONFIG_ID,
                config_param: WifiStationConfig::ActiveOnStartup(OnOff::Off),
            })
            .await?;

        self.at
            .send(SetWifiStationConfig {
                config_id: CONFIG_ID,
                config_param: WifiStationConfig::SSID(
                    heapless::String::try_from(ssid).map_err(|_| Error::Overflow)?,
                ),
            })
            .await?;

        self.at
            .send(SetWifiStationConfig {
                config_id: CONFIG_ID,
                config_param: WifiStationConfig::Authentication(Authentication::WpaWpa2Psk),
            })
            .await?;

        self.at
            .send(SetWifiStationConfig {
                config_id: CONFIG_ID,
                config_param: WifiStationConfig::WpaPskOrPassphrase(
                    heapless::String::try_from(passphrase).map_err(|_| Error::Overflow)?,
                ),
            })
            .await?;

        self.at
            .send(ExecWifiStationAction {
                config_id: CONFIG_ID,
                action: WifiStationAction::Activate,
            })
            .await?;

        with_timeout(Duration::from_secs(20), self.wait_for_join(ssid))
            .await
            .map_err(|_| Error::Timeout)??;

        Ok(())
    }

    pub async fn disconnect(&mut self) -> Result<(), Error> {
        match self.get_wifi_status().await? {
            WifiStatusVal::Disabled => {}
            WifiStatusVal::Disconnected | WifiStatusVal::Connected => {
                self.at
                    .send(ExecWifiStationAction {
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
        self.at.send(WriteGPIO { id, value }).await?;
        Ok(())
    }

    // FIXME: This could probably be improved
    pub async fn import_credentials(
        &mut self,
        data_type: SecurityDataType,
        name: &str,
        data: &[u8],
        md5_sum: Option<&str>,
    ) -> Result<(), atat::Error> {
        assert!(name.len() < 16);

        info!("Importing {:?} bytes as {:?}", data.len(), name);

        self.at
            .send(PrepareSecurityDataImport {
                data_type,
                data_size: data.len(),
                internal_name: name,
                password: None,
            })
            .await?;

        let import_data = self
            .at
            .send(SendSecurityDataImport {
                data: atat::serde_bytes::Bytes::new(data),
            })
            .await?;

        if let Some(hash) = md5_sum {
            assert_eq!(import_data.md5_string.as_str(), hash);
        }

        Ok(())
    }
}
