use core::cell::Cell;
use core::str::FromStr as _;

use atat::AtatCmd;
use atat::{asynch::AtatClient, response_slot::ResponseSlotGuard, UrcChannel};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Sender};
use embassy_time::{with_timeout, Duration, Timer};
use heapless::Vec;
use no_std_net::Ipv4Addr;

use crate::command::general::responses::SoftwareVersionResponse;
use crate::command::general::types::FirmwareVersion;
use crate::command::general::SoftwareVersion;
use crate::command::gpio::responses::ReadGPIOResponse;
use crate::command::gpio::types::GPIOMode;
use crate::command::gpio::ConfigureGPIO;
use crate::command::network::responses::NetworkStatusResponse;
use crate::command::network::types::{NetworkStatus, NetworkStatusParameter};
use crate::command::network::GetNetworkStatus;
use crate::command::ping::Ping;
use crate::command::system::responses::LocalAddressResponse;
use crate::command::system::types::InterfaceID;
use crate::command::system::GetLocalAddress;
use crate::command::wifi::types::{IPv4Mode, PasskeyR};
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
use crate::connection::{DnsServers, StaticConfigV4, WiFiState};
use crate::error::Error;
use crate::options::{ConnectionOptions, HotspotOptions, WifiAuthentication};

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
            trace!(
                "Sending command: {:?}",
                atat::helpers::LossyStr(&buf[..len])
            );
        } else {
            trace!("Sending command with long payload ({} bytes)", len);
        }

        if let Some(cooldown) = self.cooldown_timer.take() {
            cooldown.await
        }

        // TODO: Guard against race condition!
        with_timeout(
            Duration::from_secs(1),
            self.req_sender.send(Vec::try_from(&buf[..len]).unwrap()),
        )
        .await
        .map_err(|_| atat::Error::Timeout)?;

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
    urc_channel: &'a UrcChannel<UbloxUrc, URC_CAPACITY, { URC_SUBSCRIBERS }>,
}

impl<'a, const INGRESS_BUF_SIZE: usize, const URC_CAPACITY: usize>
    Control<'a, INGRESS_BUF_SIZE, URC_CAPACITY>
{
    pub(crate) fn new(
        state_ch: state::Runner<'a>,
        urc_channel: &'a UrcChannel<UbloxUrc, URC_CAPACITY, { URC_SUBSCRIBERS }>,
        req_sender: Sender<'a, NoopRawMutex, Vec<u8, MAX_CMD_LEN>, 1>,
        res_slot: &'a atat::ResponseSlot<INGRESS_BUF_SIZE>,
    ) -> Self {
        Self {
            state_ch,
            at_client: ProxyClient::new(req_sender, res_slot),
            urc_channel: urc_channel,
        }
    }

    /// Set the hostname of the device
    pub async fn set_hostname(&self, hostname: &str) -> Result<(), Error> {
        self.state_ch.wait_for_initialized().await;

        (&self.at_client)
            .send_retry(&SetNetworkHostName {
                host_name: hostname,
            })
            .await?;
        Ok(())
    }

    /// Gets the firmware version of the device
    pub async fn get_version(&self) -> Result<FirmwareVersion, Error> {
        self.state_ch.wait_for_initialized().await;

        let SoftwareVersionResponse { version } =
            (&self.at_client).send_retry(&SoftwareVersion).await?;
        Ok(version)
    }

    /// Gets the MAC address of the device
    pub async fn hardware_address(&mut self) -> Result<[u8; 6], Error> {
        self.state_ch.wait_for_initialized().await;

        let LocalAddressResponse { mac } = (&self.at_client)
            .send_retry(&GetLocalAddress {
                interface_id: InterfaceID::WiFi,
            })
            .await?;

        Ok(mac.to_be_bytes()[2..].try_into().unwrap())
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

    pub async fn wait_for_link_state(&self, link_state: LinkState) {
        self.state_ch.wait_for_link_state(link_state).await
    }

    pub async fn config_v4(&self) -> Result<Option<StaticConfigV4>, Error> {
        let NetworkStatusResponse {
            status: NetworkStatus::IPv4Address(ipv4),
            ..
        } = (&self.at_client)
            .send_retry(&GetNetworkStatus {
                interface_id: 0,
                status: NetworkStatusParameter::IPv4Address,
            })
            .await?
        else {
            return Err(Error::Network);
        };

        let ipv4_addr = core::str::from_utf8(ipv4.as_slice())
            .ok()
            .and_then(|s| Ipv4Addr::from_str(s).ok())
            .and_then(|ip| (!ip.is_unspecified()).then_some(ip));

        let NetworkStatusResponse {
            status: NetworkStatus::Gateway(gateway),
            ..
        } = (&self.at_client)
            .send_retry(&GetNetworkStatus {
                interface_id: 0,
                status: NetworkStatusParameter::Gateway,
            })
            .await?
        else {
            return Err(Error::Network);
        };

        let gateway_addr = core::str::from_utf8(gateway.as_slice())
            .ok()
            .and_then(|s| Ipv4Addr::from_str(s).ok())
            .and_then(|ip| (!ip.is_unspecified()).then_some(ip));

        let NetworkStatusResponse {
            status: NetworkStatus::PrimaryDNS(primary),
            ..
        } = (&self.at_client)
            .send_retry(&GetNetworkStatus {
                interface_id: 0,
                status: NetworkStatusParameter::PrimaryDNS,
            })
            .await?
        else {
            return Err(Error::Network);
        };

        let primary = core::str::from_utf8(primary.as_slice())
            .ok()
            .and_then(|s| Ipv4Addr::from_str(s).ok())
            .and_then(|ip| (!ip.is_unspecified()).then_some(ip));

        let NetworkStatusResponse {
            status: NetworkStatus::SecondaryDNS(secondary),
            ..
        } = (&self.at_client)
            .send_retry(&GetNetworkStatus {
                interface_id: 0,
                status: NetworkStatusParameter::SecondaryDNS,
            })
            .await?
        else {
            return Err(Error::Network);
        };

        let secondary = core::str::from_utf8(secondary.as_slice())
            .ok()
            .and_then(|s| Ipv4Addr::from_str(s).ok())
            .and_then(|ip| (!ip.is_unspecified()).then_some(ip));

        Ok(ipv4_addr.map(|address| StaticConfigV4 {
            address,
            gateway: gateway_addr,
            dns_servers: DnsServers { primary, secondary },
        }))
    }

    pub async fn get_connected_ssid(&self) -> Result<heapless::String<64>, Error> {
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

    pub async fn start_ap(
        &self,
        options: ConnectionOptions<'_>,
        configuration: HotspotOptions,
    ) -> Result<(), Error> {
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

        // Disable DHCP Server (static IP address will be used)
        if options.ip.is_some() || options.subnet.is_some() || options.gateway.is_some() {
            (&self.at_client)
                .send_retry(&SetWifiAPConfig {
                    ap_config_id: AccessPointId::Id0,
                    ap_config_param: AccessPointConfig::IPv4Mode(IPv4Mode::Static),
                })
                .await?;
        }

        // Network IP address
        if let Some(ip) = options.ip {
            (&self.at_client)
                .send_retry(&SetWifiAPConfig {
                    ap_config_id: AccessPointId::Id0,
                    ap_config_param: AccessPointConfig::IPv4Address(ip),
                })
                .await?;
        }
        // Network Subnet mask
        if let Some(subnet) = options.subnet {
            (&self.at_client)
                .send_retry(&SetWifiAPConfig {
                    ap_config_id: AccessPointId::Id0,
                    ap_config_param: AccessPointConfig::SubnetMask(subnet),
                })
                .await?;
        }
        // Network Default gateway
        if let Some(gateway) = options.gateway {
            (&self.at_client)
                .send_retry(&SetWifiAPConfig {
                    ap_config_id: AccessPointId::Id0,
                    ap_config_param: AccessPointConfig::DefaultGateway(gateway),
                })
                .await?;
        }
        // Network Primary DNS
        if let Some(dns) = options.dns {
            (&self.at_client)
                .send_retry(&SetWifiAPConfig {
                    ap_config_id: AccessPointId::Id0,
                    ap_config_param: AccessPointConfig::PrimaryDNS(dns),
                })
                .await?;
        }

        (&self.at_client)
            .send_retry(&SetWifiAPConfig {
                ap_config_id: AccessPointId::Id0,
                ap_config_param: AccessPointConfig::DHCPServer(configuration.dhcp_server.into()),
            })
            .await?;

        // Set the Network SSID to connect to
        (&self.at_client)
            .send_retry(&SetWifiAPConfig {
                ap_config_id: AccessPointId::Id0,
                ap_config_param: AccessPointConfig::SSID(options.ssid),
            })
            .await?;

        match options.auth {
            WifiAuthentication::None => {
                (&self.at_client)
                    .send_retry(&SetWifiAPConfig {
                        ap_config_id: AccessPointId::Id0,
                        ap_config_param: AccessPointConfig::SecurityMode(
                            SecurityMode::Open,
                            SecurityModePSK::Open,
                        ),
                    })
                    .await?;
            }
            WifiAuthentication::Wpa2Passphrase(passphrase) => {
                (&self.at_client)
                    .send_retry(&SetWifiAPConfig {
                        ap_config_id: AccessPointId::Id0,
                        ap_config_param: AccessPointConfig::SecurityMode(
                            SecurityMode::Wpa2AesCcmp,
                            SecurityModePSK::PSK,
                        ),
                    })
                    .await?;

                // Input passphrase
                (&self.at_client)
                    .send_retry(&SetWifiAPConfig {
                        ap_config_id: AccessPointId::Id0,
                        ap_config_param: AccessPointConfig::PSKPassphrase(PasskeyR::Passphrase(
                            // FIXME:
                            heapless::String::try_from(passphrase).unwrap(),
                        )),
                    })
                    .await?;
            } // WifiAuthentication::Wpa2Psk(_psk) => {
              //     unimplemented!()
              //     // (&self.at_client)
              //     //     .send_retry(&SetWifiStationConfig {
              //     //         config_id: CONFIG_ID,
              //     //         config_param: WifiStationConfig::Authentication(Authentication::WpaWpa2Psk),
              //     //     })
              //     //     .await?;

              //     // (&self.at_client)
              //     //     .send_retry(&SetWifiStationConfig {
              //     //         config_id: CONFIG_ID,
              //     //         config_param: WifiStationConfig::WpaPskOrPassphrase(todo!("hex values?!")),
              //     //     })
              //     //     .await?;
              // }
        }

        if let Some(channel) = configuration.channel {
            (&self.at_client)
                .send_retry(&SetWifiAPConfig {
                    ap_config_id: AccessPointId::Id0,
                    ap_config_param: AccessPointConfig::Channel(channel as u8),
                })
                .await?;
        }

        (&self.at_client)
            .send_retry(&WifiAPAction {
                ap_config_id: AccessPointId::Id0,
                ap_action: AccessPointAction::Activate,
            })
            .await?;

        self.state_ch.set_should_connect(true);

        Ok(())
    }

    /// Closes access point.
    pub async fn close_ap(&self) -> Result<(), Error> {
        self.state_ch.wait_for_initialized().await;
        self.state_ch.set_should_connect(false);

        (&self.at_client)
            .send_retry(&WifiAPAction {
                ap_config_id: AccessPointId::Id0,
                ap_action: AccessPointAction::Deactivate,
            })
            .await?;
        Ok(())
    }

    pub async fn peek_join_sta(&self, options: ConnectionOptions<'_>) -> Result<(), Error> {
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
                config_param: WifiStationConfig::SSID(options.ssid),
            })
            .await?;

        match options.auth {
            WifiAuthentication::None => {
                (&self.at_client)
                    .send_retry(&SetWifiStationConfig {
                        config_id: CONFIG_ID,
                        config_param: WifiStationConfig::Authentication(Authentication::Open),
                    })
                    .await?;
            }
            WifiAuthentication::Wpa2Passphrase(passphrase) => {
                (&self.at_client)
                    .send_retry(&SetWifiStationConfig {
                        config_id: CONFIG_ID,
                        config_param: WifiStationConfig::Authentication(Authentication::WpaWpa2Psk),
                    })
                    .await?;

                (&self.at_client)
                    .send_retry(&SetWifiStationConfig {
                        config_id: CONFIG_ID,
                        config_param: WifiStationConfig::WpaPskOrPassphrase(passphrase),
                    })
                    .await?;
            } // WifiAuthentication::Wpa2Psk(_psk) => {
              //     unimplemented!()
              //     // (&self.at_client)
              //     //     .send_retry(&SetWifiStationConfig {
              //     //         config_id: CONFIG_ID,
              //     //         config_param: WifiStationConfig::Authentication(Authentication::WpaWpa2Psk),
              //     //     })
              //     //     .await?;

              //     // (&self.at_client)
              //     //     .send_retry(&SetWifiStationConfig {
              //     //         config_id: CONFIG_ID,
              //     //         config_param: WifiStationConfig::WpaPskOrPassphrase(todo!("hex values?!")),
              //     //     })
              //     //     .await?;
              // }
        }

        if options.ip.is_some() || options.subnet.is_some() || options.gateway.is_some() {
            (&self.at_client)
                .send_retry(&SetWifiStationConfig {
                    config_id: CONFIG_ID,
                    config_param: WifiStationConfig::IPv4Mode(IPv4Mode::Static),
                })
                .await?;
        }

        // Network IP address
        if let Some(ip) = options.ip {
            (&self.at_client)
                .send_retry(&SetWifiStationConfig {
                    config_id: CONFIG_ID,
                    config_param: WifiStationConfig::IPv4Address(ip),
                })
                .await?;
        }
        // Network Subnet mask
        if let Some(subnet) = options.subnet {
            (&self.at_client)
                .send_retry(&SetWifiStationConfig {
                    config_id: CONFIG_ID,
                    config_param: WifiStationConfig::SubnetMask(subnet),
                })
                .await?;
        }
        // Network Default gateway
        if let Some(gateway) = options.gateway {
            (&self.at_client)
                .send_retry(&SetWifiStationConfig {
                    config_id: CONFIG_ID,
                    config_param: WifiStationConfig::DefaultGateway(gateway),
                })
                .await?;
        }

        (&self.at_client)
            .send_retry(&ExecWifiStationAction {
                config_id: CONFIG_ID,
                action: WifiStationAction::Activate,
            })
            .await?;

        self.wait_for_join(options.ssid, Duration::from_secs(20))
            .await?;

        Ok(())
    }

    pub async fn join_sta(&self, options: ConnectionOptions<'_>) -> Result<(), Error> {
        self.state_ch.wait_for_initialized().await;

        if matches!(self.get_wifi_status().await?, WifiStatusVal::Connected) {
            // Wifi already connected. Check if the SSID is the same
            let current_ssid = self.get_connected_ssid().await?;
            if current_ssid.as_str() == options.ssid {
                self.state_ch.set_should_connect(true);
                return Ok(());
            } else {
                self.leave().await?;
            };
        }

        self.peek_join_sta(options).await?;

        self.state_ch.set_should_connect(true);

        Ok(())
    }

    /// Leave the wifi, with which we are currently associated.
    pub async fn leave(&self) -> Result<(), Error> {
        self.state_ch.wait_for_initialized().await;
        self.state_ch.set_should_connect(false);

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

    pub async fn wait_for_join(&self, ssid: &str, timeout: Duration) -> Result<(), Error> {
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
                let _ = (&self.at_client)
                    .send_retry(&ExecWifiStationAction {
                        config_id: CONFIG_ID,
                        action: WifiStationAction::Deactivate,
                    })
                    .await;
                Err(Error::SecurityProblems)
            }
            Err(_) => Err(Error::Timeout),
        }
    }

    // /// Start a wifi scan
    // ///
    // /// Returns a `Stream` of networks found by the device
    // ///
    // /// # Note
    // /// Device events are currently implemented using a bounded queue.
    // /// To not miss any events, you should make sure to always await the stream.
    // pub async fn scan(&mut self, scan_opts: ScanOptions) -> Scanner<'_> {
    //     todo!()
    // }

    pub async fn send_at<Cmd: AtatCmd>(&self, cmd: &Cmd) -> Result<Cmd::Response, Error> {
        self.state_ch.wait_for_initialized().await;
        Ok((&self.at_client).send_retry(cmd).await?)
    }

    pub async fn gpio_configure(&self, id: GPIOId, mode: GPIOMode) -> Result<(), Error> {
        self.send_at(&ConfigureGPIO { id, mode }).await?;
        Ok(())
    }

    pub async fn gpio_set(&self, id: GPIOId, value: bool) -> Result<(), Error> {
        let value = if value {
            GPIOValue::High
        } else {
            GPIOValue::Low
        };

        self.send_at(&WriteGPIO { id, value }).await?;
        Ok(())
    }

    pub async fn gpio_get(&self, id: GPIOId) -> Result<bool, Error> {
        let ReadGPIOResponse { value, .. } = self.send_at(&ReadGPIO { id }).await?;
        Ok(value as u8 != 0)
    }

    #[cfg(feature = "ppp")]
    pub async fn ping(
        &self,
        hostname: &str,
    ) -> Result<crate::command::ping::urc::PingResponse, Error> {
        let mut urc_sub = self.urc_channel.subscribe().map_err(|_| Error::Overflow)?;

        self.send_at(&Ping {
            hostname,
            retry_num: 1,
        })
        .await?;

        let result_fut = async {
            loop {
                match urc_sub.next_message_pure().await {
                    crate::command::Urc::PingResponse(r) => return Ok(r),
                    crate::command::Urc::PingErrorResponse(e) => return Err(Error::Dns(e.error)),
                    _ => {}
                }
            }
        };

        with_timeout(Duration::from_secs(15), result_fut).await?
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
