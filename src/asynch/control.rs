use core::cell::Cell;

use atat::{asynch::AtatClient, response_slot::ResponseSlotGuard, UrcChannel};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Sender};
use embassy_time::{with_timeout, Duration, Timer};
use heapless::Vec;

use crate::command::gpio::responses::ReadGPIOResponse;
use crate::command::gpio::types::GPIOMode;
use crate::command::gpio::ConfigureGPIO;
use crate::command::wifi::{ExecWifiStationAction, GetWifiStatus, SetWifiStationConfig};
use crate::command::OnOff;
use crate::command::{
    gpio::ReadGPIO,
    wifi::{
        types::{
            AccessPointAction, Authentication, SecurityMode, SecurityModePSK, StatusId,
            WifiStationAction, WifiStationConfig, WifiStatus, WifiStatusVal,
        },
        WifiAPAction,
    },
};
use crate::command::{
    gpio::{
        types::{GPIOId, GPIOValue},
        WriteGPIO,
    },
    wifi::SetWifiAPConfig,
};
use crate::command::{network::SetNetworkHostName, wifi::types::AccessPointConfig};
use crate::command::{
    system::{RebootDCE, ResetToFactoryDefaults},
    wifi::types::AccessPointId,
};
use crate::connection::WiFiState;
use crate::error::Error;

use super::runner::{MAX_CMD_LEN, URC_SUBSCRIBERS};
use super::state::LinkState;
use super::{state, UbloxUrc};

const CONFIG_ID: u8 = 0;

pub(crate) struct ProxyClient<'a, const INGRESS_BUF_SIZE: usize> {
    pub(crate) req_sender: Sender<'a, NoopRawMutex, Vec<u8, MAX_CMD_LEN>, 1>,
    pub(crate) res_slot: &'a atat::ResponseSlot<INGRESS_BUF_SIZE>,
    cooldown_timer: Cell<Option<Timer>>,
}

impl<'a, const INGRESS_BUF_SIZE: usize> ProxyClient<'a, INGRESS_BUF_SIZE> {
    pub fn new(
        req_sender: Sender<'a, NoopRawMutex, Vec<u8, MAX_CMD_LEN>, 1>,
        res_slot: &'a atat::ResponseSlot<INGRESS_BUF_SIZE>,
    ) -> Self {
        Self {
            req_sender,
            res_slot,
            cooldown_timer: Cell::new(None),
        }
    }

    async fn wait_response(
        &self,
        timeout: Duration,
    ) -> Result<ResponseSlotGuard<'_, INGRESS_BUF_SIZE>, atat::Error> {
        with_timeout(timeout, self.res_slot.get())
            .await
            .map_err(|_| atat::Error::Timeout)
    }
}

impl<'a, const INGRESS_BUF_SIZE: usize> atat::asynch::AtatClient
    for &ProxyClient<'a, INGRESS_BUF_SIZE>
{
    async fn send<Cmd: atat::AtatCmd>(&mut self, cmd: &Cmd) -> Result<Cmd::Response, atat::Error> {
        let mut buf = [0u8; MAX_CMD_LEN];
        let len = cmd.write(&mut buf);

        if len < 50 {
            debug!(
                "Sending command: {:?}",
                atat::helpers::LossyStr(&buf[..len])
            );
        } else {
            debug!("Sending command with long payload ({} bytes)", len);
        }

        if let Some(cooldown) = self.cooldown_timer.take() {
            cooldown.await
        }

        // TODO: Guard against race condition!
        self.req_sender
            .send(Vec::try_from(&buf[..len]).unwrap())
            .await;

        self.cooldown_timer.set(Some(Timer::after_millis(20)));

        if !Cmd::EXPECTS_RESPONSE_CODE {
            cmd.parse(Ok(&[]))
        } else {
            let response = self
                .wait_response(Duration::from_millis(Cmd::MAX_TIMEOUT_MS.into()))
                .await?;
            let response: &atat::Response<INGRESS_BUF_SIZE> = &response.borrow();
            cmd.parse(response.into())
        }
    }
}

pub struct Control<'a, const INGRESS_BUF_SIZE: usize, const URC_CAPACITY: usize> {
    state_ch: state::Runner<'a>,
    at_client: ProxyClient<'a, INGRESS_BUF_SIZE>,
    _urc_channel: &'a UrcChannel<UbloxUrc, URC_CAPACITY, URC_SUBSCRIBERS>,
}

impl<'a, const INGRESS_BUF_SIZE: usize, const URC_CAPACITY: usize>
    Control<'a, INGRESS_BUF_SIZE, URC_CAPACITY>
{
    pub(crate) fn new(
        state_ch: state::Runner<'a>,
        urc_channel: &'a UrcChannel<UbloxUrc, URC_CAPACITY, URC_SUBSCRIBERS>,
        req_sender: Sender<'a, NoopRawMutex, Vec<u8, MAX_CMD_LEN>, 1>,
        res_slot: &'a atat::ResponseSlot<INGRESS_BUF_SIZE>,
    ) -> Self {
        Self {
            state_ch,
            at_client: ProxyClient::new(req_sender, res_slot),
            _urc_channel: urc_channel,
        }
    }

    pub async fn set_hostname(&self, hostname: &str) -> Result<(), Error> {
        self.state_ch.wait_for_initialized().await;

        (&(&self).at_client)
            .send_retry(&SetNetworkHostName {
                host_name: hostname,
            })
            .await?;
        Ok(())
    }

    async fn get_wifi_status(&self) -> Result<WifiStatusVal, Error> {
        match (&self.at_client)
            .send_retry(&GetWifiStatus {
                status_id: StatusId::Status,
            })
            .await?
            .status_id
        {
            WifiStatus::Status(s) => Ok(s),
            _ => Err(Error::AT(atat::Error::InvalidResponse)),
        }
    }

    async fn get_connected_ssid(&self) -> Result<heapless::String<64>, Error> {
        match (&self.at_client)
            .send_retry(&GetWifiStatus {
                status_id: StatusId::SSID,
            })
            .await?
            .status_id
        {
            WifiStatus::SSID(s) => Ok(s),
            _ => Err(Error::AT(atat::Error::InvalidResponse)),
        }
    }

    pub async fn factory_reset(&self) -> Result<(), Error> {
        self.state_ch.wait_for_initialized().await;

        (&self.at_client)
            .send_retry(&ResetToFactoryDefaults)
            .await?;
        (&self.at_client).send_retry(&RebootDCE).await?;

        Ok(())
    }

    pub async fn start_ap(&self, ssid: &str) -> Result<(), Error> {
        self.state_ch.wait_for_initialized().await;

        // Deactivate network id 0
        (&self.at_client)
            .send_retry(&WifiAPAction {
                ap_config_id: AccessPointId::Id0,
                ap_action: AccessPointAction::Deactivate,
            })
            .await?;

        (&self.at_client)
            .send_retry(&WifiAPAction {
                ap_config_id: AccessPointId::Id0,
                ap_action: AccessPointAction::Reset,
            })
            .await?;

        // // Disable DHCP Server (static IP address will be used)
        // if options.ip.is_some() || options.subnet.is_some() || options.gateway.is_some() {
        //     (&self.at_client)
        //         .send_retry(&SetWifiAPConfig {
        //             ap_config_id: AccessPointId::Id0,
        //             ap_config_param: AccessPointConfig::IPv4Mode(IPv4Mode::Static),
        //         })
        //         .await?;
        // }

        // // Network IP address
        // if let Some(ip) = options.ip {
        //     (&self.at_client)
        //         .send_retry(&SetWifiAPConfig {
        //             ap_config_id: AccessPointId::Id0,
        //             ap_config_param: AccessPointConfig::IPv4Address(ip),
        //         })
        //         .await?;
        // }
        // // Network Subnet mask
        // if let Some(subnet) = options.subnet {
        //     (&self.at_client)
        //         .send_retry(&SetWifiAPConfig {
        //             ap_config_id: AccessPointId::Id0,
        //             ap_config_param: AccessPointConfig::SubnetMask(subnet),
        //         })
        //         .await?;
        // }
        // // Network Default gateway
        // if let Some(gateway) = options.gateway {
        //     (&self.at_client)
        //         .send_retry(&SetWifiAPConfig {
        //             ap_config_id: AccessPointId::Id0,
        //             ap_config_param: AccessPointConfig::DefaultGateway(gateway),
        //         })
        //         .await?;
        // }

        // (&self.at_client)
        //     .send_retry(&SetWifiAPConfig {
        //         ap_config_id: AccessPointId::Id0,
        //         ap_config_param: AccessPointConfig::DHCPServer(true.into()),
        //     })
        //     .await?;

        // Wifi part
        // Set the Network SSID to connect to
        (&self.at_client)
            .send_retry(&SetWifiAPConfig {
                ap_config_id: AccessPointId::Id0,
                ap_config_param: AccessPointConfig::SSID(
                    heapless::String::try_from(ssid).map_err(|_| Error::Overflow)?,
                ),
            })
            .await?;

        // if let Some(pass) = options.password.clone() {
        //     // Use WPA2 as authentication type
        //     (&self.at_client)
        //         .send_retry(&SetWifiAPConfig {
        //             ap_config_id: AccessPointId::Id0,
        //             ap_config_param: AccessPointConfig::SecurityMode(
        //                 SecurityMode::Wpa2AesCcmp,
        //                 SecurityModePSK::PSK,
        //             ),
        //         })
        //         .await?;

        //     // Input passphrase
        //     (&self.at_client)
        //         .send_retry(&SetWifiAPConfig {
        //             ap_config_id: AccessPointId::Id0,
        //             ap_config_param: AccessPointConfig::PSKPassphrase(PasskeyR::Passphrase(pass)),
        //         })
        //         .await?;
        // } else {
        (&self.at_client)
            .send_retry(&SetWifiAPConfig {
                ap_config_id: AccessPointId::Id0,
                ap_config_param: AccessPointConfig::SecurityMode(
                    SecurityMode::Open,
                    SecurityModePSK::Open,
                ),
            })
            .await?;
        // }

        // if let Some(channel) = configuration.channel {
        //     (&self.at_client)
        //         .send_retry(&SetWifiAPConfig {
        //             ap_config_id: AccessPointId::Id0,
        //             ap_config_param: AccessPointConfig::Channel(channel as u8),
        //         })
        //         .await?;
        // }

        (&self.at_client)
            .send_retry(&WifiAPAction {
                ap_config_id: AccessPointId::Id0,
                ap_action: AccessPointAction::Activate,
            })
            .await?;

        Ok(())
    }

    pub async fn join_open(&self, ssid: &str) -> Result<(), Error> {
        self.state_ch.wait_for_initialized().await;

        if matches!(self.get_wifi_status().await?, WifiStatusVal::Connected) {
            // Wifi already connected. Check if the SSID is the same
            let current_ssid = self.get_connected_ssid().await?;
            if current_ssid.as_str() == ssid {
                return Ok(());
            } else {
                self.disconnect().await?;
            };
        }

        (&self.at_client)
            .send_retry(&ExecWifiStationAction {
                config_id: CONFIG_ID,
                action: WifiStationAction::Reset,
            })
            .await?;

        (&self.at_client)
            .send_retry(&SetWifiStationConfig {
                config_id: CONFIG_ID,
                config_param: WifiStationConfig::ActiveOnStartup(OnOff::Off),
            })
            .await?;

        (&self.at_client)
            .send_retry(&SetWifiStationConfig {
                config_id: CONFIG_ID,
                config_param: WifiStationConfig::SSID(
                    heapless::String::try_from(ssid).map_err(|_| Error::Overflow)?,
                ),
            })
            .await?;

        (&self.at_client)
            .send_retry(&SetWifiStationConfig {
                config_id: CONFIG_ID,
                config_param: WifiStationConfig::Authentication(Authentication::Open),
            })
            .await?;

        (&self.at_client)
            .send_retry(&ExecWifiStationAction {
                config_id: CONFIG_ID,
                action: WifiStationAction::Activate,
            })
            .await?;

        self.wait_for_join(ssid, Duration::from_secs(20)).await
    }

    pub async fn join_wpa2(&self, ssid: &str, passphrase: &str) -> Result<(), Error> {
        self.state_ch.wait_for_initialized().await;

        if matches!(self.get_wifi_status().await?, WifiStatusVal::Connected) {
            // Wifi already connected. Check if the SSID is the same
            let current_ssid = self.get_connected_ssid().await?;
            if current_ssid.as_str() == ssid {
                return Ok(());
            } else {
                self.disconnect().await?;
            };
        }

        (&self.at_client)
            .send_retry(&ExecWifiStationAction {
                config_id: CONFIG_ID,
                action: WifiStationAction::Reset,
            })
            .await?;

        (&self.at_client)
            .send_retry(&SetWifiStationConfig {
                config_id: CONFIG_ID,
                config_param: WifiStationConfig::ActiveOnStartup(OnOff::Off),
            })
            .await?;

        (&self.at_client)
            .send_retry(&SetWifiStationConfig {
                config_id: CONFIG_ID,
                config_param: WifiStationConfig::SSID(
                    heapless::String::try_from(ssid).map_err(|_| Error::Overflow)?,
                ),
            })
            .await?;

        (&self.at_client)
            .send_retry(&SetWifiStationConfig {
                config_id: CONFIG_ID,
                config_param: WifiStationConfig::Authentication(Authentication::WpaWpa2Psk),
            })
            .await?;

        (&self.at_client)
            .send_retry(&SetWifiStationConfig {
                config_id: CONFIG_ID,
                config_param: WifiStationConfig::WpaPskOrPassphrase(
                    heapless::String::try_from(passphrase).map_err(|_| Error::Overflow)?,
                ),
            })
            .await?;

        (&self.at_client)
            .send_retry(&ExecWifiStationAction {
                config_id: CONFIG_ID,
                action: WifiStationAction::Activate,
            })
            .await?;

        self.wait_for_join(ssid, Duration::from_secs(20)).await
    }

    pub async fn disconnect(&self) -> Result<(), Error> {
        self.state_ch.wait_for_initialized().await;

        match self.get_wifi_status().await? {
            WifiStatusVal::Disabled => {}
            WifiStatusVal::Disconnected | WifiStatusVal::Connected => {
                (&self.at_client)
                    .send_retry(&ExecWifiStationAction {
                        config_id: CONFIG_ID,
                        action: WifiStationAction::Deactivate,
                    })
                    .await?;
            }
        }

        with_timeout(
            Duration::from_secs(10),
            self.state_ch.wait_connection_down(),
        )
        .await
        .map_err(|_| Error::Timeout)?;

        Ok(())
    }

    async fn wait_for_join(&self, ssid: &str, timeout: Duration) -> Result<(), Error> {
        match with_timeout(timeout, self.state_ch.wait_for_link_state(LinkState::Up)).await {
            Ok(_) => {
                // Check that SSID matches
                let current_ssid = self.get_connected_ssid().await?;
                if ssid != current_ssid.as_str() {
                    return Err(Error::Network);
                }

                Ok(())
            }
            Err(_) if self.state_ch.wifi_state(None) == WiFiState::SecurityProblems => {
                Err(Error::SecurityProblems)
            }
            Err(_) => Err(Error::Timeout),
        }
    }

    pub async fn gpio_configure(&self, id: GPIOId, mode: GPIOMode) -> Result<(), Error> {
        self.state_ch.wait_for_initialized().await;
        (&self.at_client)
            .send_retry(&ConfigureGPIO { id, mode })
            .await?;
        Ok(())
    }

    pub async fn gpio_set(&self, id: GPIOId, value: bool) -> Result<(), Error> {
        self.state_ch.wait_for_initialized().await;

        let value = if value {
            GPIOValue::High
        } else {
            GPIOValue::Low
        };

        (&self.at_client)
            .send_retry(&WriteGPIO { id, value })
            .await?;
        Ok(())
    }

    pub async fn gpio_get(&self, id: GPIOId) -> Result<bool, Error> {
        self.state_ch.wait_for_initialized().await;

        let ReadGPIOResponse { value, .. } = (&self.at_client).send_retry(&ReadGPIO { id }).await?;
        Ok(value as u8 != 0)
    }

    // FIXME: This could probably be improved
    // #[cfg(feature = "internal-network-stack")]
    // pub async fn import_credentials(
    //     &mut self,
    //     data_type: SecurityDataType,
    //     name: &str,
    //     data: &[u8],
    //     md5_sum: Option<&str>,
    // ) -> Result<(), atat::Error> {
    //     assert!(name.len() < 16);

    //     info!("Importing {:?} bytes as {:?}", data.len(), name);

    //     (&self.at_client)
    //         .send_retry(&PrepareSecurityDataImport {
    //             data_type,
    //             data_size: data.len(),
    //             internal_name: name,
    //             password: None,
    //         })
    //         .await?;

    //     let import_data = self
    //         .at_client
    //         .send_retry(&SendSecurityDataImport {
    //             data: atat::serde_bytes::Bytes::new(data),
    //         })
    //         .await?;

    //     if let Some(hash) = md5_sum {
    //         assert_eq!(import_data.md5_string.as_str(), hash);
    //     }

    //     Ok(())
    // }
}
