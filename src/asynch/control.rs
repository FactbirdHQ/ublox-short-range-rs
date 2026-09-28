use core::cell::Cell;
use core::net::Ipv4Addr;
use core::str::FromStr as _;

use atat::AtatCmd;
use atat::{asynch::AtatClient, response_slot::ResponseSlotGuard, UrcChannel};
use embassy_futures::select::{select, Either};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Sender};
use embassy_time::{with_timeout, Duration, Timer};
use embedded_io_async::Write as _;
use heapless::Vec;

use crate::command::general::responses::SoftwareVersionResponse;
use crate::command::general::types::FirmwareVersion;
use crate::command::general::SoftwareVersion;
use crate::command::gpio::responses::ReadGPIOResponse;
use crate::command::gpio::types::GPIOMode;
use crate::command::gpio::ConfigureGPIO;
use crate::command::network::responses::NetworkStatusResponse;
use crate::command::network::types::{NetworkStatus, NetworkStatusParameter};
use crate::command::network::GetNetworkStatus;
#[cfg(feature = "ppp")]
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

    async fn wait_cooldown(&self) {
        if let Some(cooldown) = self.cooldown_timer.take() {
            cooldown.await
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

    async fn parse_response<Cmd: AtatCmd>(&self, cmd: &Cmd) -> Result<Cmd::Response, atat::Error> {
        self.cooldown_timer.set(Some(Timer::after_millis(20)));

        if !Cmd::EXPECTS_RESPONSE_CODE {
            return cmd.parse(Ok(&[]));
        }

        let response = self
            .wait_response(Duration::from_millis(Cmd::MAX_TIMEOUT_MS.into()))
            .await?;
        cmd.parse((&*response).into())
    }
}

impl<const INGRESS_BUF_SIZE: usize> embedded_io_async::ErrorType
    for &ProxyClient<'_, INGRESS_BUF_SIZE>
{
    type Error = atat::Error;
}

/// Every write is forwarded to the bridge task as one request channel message,
/// so payloads larger than the channel item size are delivered in chunks.
impl<const INGRESS_BUF_SIZE: usize> embedded_io_async::Write
    for &ProxyClient<'_, INGRESS_BUF_SIZE>
{
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        if buf.is_empty() {
            return Ok(0);
        }

        let chunk = &buf[..buf.len().min(MAX_CMD_LEN)];

        // TODO: Guard against race condition!
        with_timeout(
            Duration::from_secs(1),
            self.req_sender.send(Vec::from_slice(chunk).unwrap()),
        )
        .await
        .map_err(|_| atat::Error::Timeout)?;

        Ok(chunk.len())
    }

    /// The transport is driven by the bridge task, so there is nothing to
    /// flush here beyond handing the message over.
    async fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<'a, const INGRESS_BUF_SIZE: usize> atat::asynch::AtatClient
    for &ProxyClient<'a, INGRESS_BUF_SIZE>
{
    type Writer = Self;

    fn inner(&mut self) -> &mut Self::Writer {
        self
    }

    async fn send_with<Cmd: AtatCmd>(
        &mut self,
        cmd: &Cmd,
        write: impl AsyncFnOnce(&mut Self::Writer) -> Result<(), atat::Error>,
    ) -> Result<Cmd::Response, atat::Error> {
        self.wait_cooldown().await;
        write(self).await?;
        self.parse_response(cmd).await
    }

    async fn send<Cmd: AtatCmd>(&mut self, cmd: &Cmd) -> Result<Cmd::Response, atat::Error> {
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

        self.send_with(cmd, async |writer| writer.write_all(&buf[..len]).await)
            .await
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
            urc_channel,
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

    pub async fn get_wifi_status(&self) -> Result<WifiStatusVal, Error> {
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

    pub async fn get_wifi_channel(&self) -> Result<u8, Error> {
        match (&self.at_client)
            .send_retry(&GetWifiStatus {
                status_id: StatusId::Channel,
            })
            .await?
            .status_id
        {
            WifiStatus::Channel(c) => Ok(c),
            _ => Err(Error::AT(atat::Error::InvalidResponse)),
        }
    }

    pub async fn get_signal_strength(&self) -> Result<i8, Error> {
        match (&self.at_client)
            .send_retry(&GetWifiStatus {
                status_id: StatusId::Rssi,
            })
            .await?
            .status_id
        {
            WifiStatus::Rssi(-32768) => Err(Error::NotConnected),
            WifiStatus::Rssi(s) => s
                .try_into()
                .map_err(|_| Error::AT(atat::Error::InvalidResponse)),
            _ => Err(Error::AT(atat::Error::InvalidResponse)),
        }
    }

    pub async fn wait_for_link_state(&self, link_state: LinkState) {
        self.state_ch.wait_for_link_state(link_state).await
    }
    pub fn is_connected(&self) -> bool {
        self.state_ch.link_state(None) == LinkState::Up
    }

    /// Last observed IPv4 address. `None` when no address is assigned.
    pub fn ipv4_addr(&self) -> Option<Ipv4Addr> {
        self.state_ch.ipv4_addr(None)
    }

    /// Wait until the cached IPv4 address differs from `prev`, then return the
    /// new value. Pass the result back as `prev` on the next call to wait for
    /// the following change.
    ///
    /// Fires when DHCP hands out an address, when the IP changes (e.g. a
    /// static IP is reconfigured upstream and the device picks up a new
    /// lease), and on disconnects that clear the cached address.
    pub async fn wait_for_ipv4_change(&self, prev: Option<Ipv4Addr>) -> Option<Ipv4Addr> {
        self.state_ch.wait_for_ipv4_change(prev).await
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
            status: NetworkStatus::SubnetMask(subnet),
            ..
        } = (&self.at_client)
            .send_retry(&GetNetworkStatus {
                interface_id: 0,
                status: NetworkStatusParameter::SubnetMask,
            })
            .await?
        else {
            return Err(Error::Network);
        };

        let subnet_mask = core::str::from_utf8(subnet.as_slice())
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
            subnet_mask,
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
    pub async fn reboot(&self) -> Result<(), Error> {
        self.state_ch.wait_for_initialized().await;

        // Setting wifi state to inactive will trigger network runner to reboot device.
        self.state_ch
            .update_connection_with(|con| con.wifi_state = WiFiState::Inactive);

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

            (&self.at_client)
                .send_retry(&SetWifiAPConfig {
                    ap_config_id: AccessPointId::Id0,
                    ap_config_param: AccessPointConfig::IPv4Address(
                        options.ip.unwrap_or(Ipv4Addr::new(192, 168, 2, 1)),
                    ),
                })
                .await?;

            (&self.at_client)
                .send_retry(&SetWifiAPConfig {
                    ap_config_id: AccessPointId::Id0,
                    ap_config_param: AccessPointConfig::SubnetMask(
                        options.subnet.unwrap_or(Ipv4Addr::new(255, 255, 255, 0)),
                    ),
                })
                .await?;

            (&self.at_client)
                .send_retry(&SetWifiAPConfig {
                    ap_config_id: AccessPointId::Id0,
                    ap_config_param: AccessPointConfig::DefaultGateway(
                        options.gateway.unwrap_or(Ipv4Addr::new(192, 168, 2, 1)),
                    ),
                })
                .await?;
        }

        // Network Primary + Secondary DNS
        let primary = match options.dns.as_slice() {
            &[primary] => Some(primary),
            &[primary, secondary] => {
                (&self.at_client)
                    .send_retry(&SetWifiAPConfig {
                        ap_config_id: AccessPointId::Id0,
                        ap_config_param: AccessPointConfig::SecondaryDNS(secondary),
                    })
                    .await?;

                Some(primary)
            }
            _ => None,
        };

        if let Some(primary) = primary {
            (&self.at_client)
                .send_retry(&SetWifiAPConfig {
                    ap_config_id: AccessPointId::Id0,
                    ap_config_param: AccessPointConfig::PrimaryDNS(primary),
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
            WifiAuthentication::WpaPsk(passphrase) => {
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
        self.state_ch.wait_for_initialized().await;

        // Deactivate first. Reset and the subsequent +UWSC writes are illegal
        // on an active station config and return ERROR; the module also
        // documents Deactivate as a no-op when already inactive, so this is
        // safe regardless of caller state.
        let _ = (&self.at_client)
            .send_retry(&ExecWifiStationAction {
                config_id: CONFIG_ID,
                action: WifiStationAction::Deactivate,
            })
            .await;

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
            WifiAuthentication::WpaPsk(passphrase) => {
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

        let status = self.get_wifi_status().await?;

        match status {
            WifiStatusVal::Disabled => {}
            WifiStatusVal::Disconnected => {
                // Wifi is disabled. Enable it
                (&self.at_client)
                    .send_retry(&ExecWifiStationAction {
                        config_id: CONFIG_ID,
                        action: WifiStationAction::Deactivate,
                    })
                    .await?;
            }
            WifiStatusVal::Connected => {
                // Wifi already connected. Check if the SSID is the same
                let current_ssid = self.get_connected_ssid().await?;
                if current_ssid.as_str() == options.ssid {
                    self.state_ch.set_should_connect(true);
                    return Ok(());
                } else {
                    self.wait_leave().await?;
                };
            }
        }

        self.peek_join_sta(options).await?;

        self.state_ch.set_should_connect(true);
        Ok(())
    }

    /// Leave the wifi and wait, with which we are currently associated.
    ///
    /// Sends `+UWSCA=<id>,Deactivate` so the module's hardware state matches
    /// our intent; without this the module stays associated and any later
    /// `+UWSCA=<id>,Reset` / `+UWSC` write in `peek_join_sta` returns ERROR.
    /// Deactivate is documented to be a no-op when already inactive, so it is
    /// always safe.
    pub async fn wait_leave(&self) -> Result<(), Error> {
        self.state_ch.set_should_connect(false);

        let _ = (&self.at_client)
            .send_retry(&ExecWifiStationAction {
                config_id: CONFIG_ID,
                action: WifiStationAction::Deactivate,
            })
            .await;

        self.state_ch.update_connection_with(|con| {
            con.reset();
        });

        with_timeout(
            Duration::from_secs(10),
            self.state_ch.wait_connection_down(),
        )
        .await
        .map_err(|_| Error::Timeout)?;

        Ok(())
    }
    /// Leave the wifi, with which we are currently associated.
    pub fn leave(&self) {
        self.state_ch.set_should_connect(false);
        self.state_ch.update_connection_with(|con| con.reset());
    }

    pub async fn wait_for_join(&self, ssid: &str, timeout: Duration) -> Result<(), Error> {
        // Race link-up against security problems detection.
        // SecurityProblems wifi_state can be overwritten by subsequent disconnect URCs
        // (e.g. OutOfRange), so we must detect it as soon as it appears rather than
        // only checking after timeout.
        let wait_for_security_error = async {
            // Only watch for state *changes* - don't check initial state,
            // as it could be stale SecurityProblems from a previous attempt.
            loop {
                let new_state = self.state_ch.wait_for_wifi_state_change().await;
                if new_state == WiFiState::SecurityProblems {
                    return;
                }
            }
        };

        match with_timeout(
            timeout,
            select(
                self.state_ch.wait_for_link_state(LinkState::Up),
                wait_for_security_error,
            ),
        )
        .await
        {
            Ok(Either::First(_)) => {
                // Link is up - check that SSID matches
                let current_ssid = self.get_connected_ssid().await?;
                if ssid != current_ssid.as_str() {
                    return Err(Error::Network);
                }
                Ok(())
            }
            Ok(Either::Second(_)) => {
                // SecurityProblems detected early - deactivate and report
                let _ = (&self.at_client)
                    .send_retry(&ExecWifiStationAction {
                        config_id: CONFIG_ID,
                        action: WifiStationAction::Deactivate,
                    })
                    .await;
                Err(Error::SecurityProblems)
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
