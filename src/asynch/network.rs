use core::str::FromStr as _;

use atat::{asynch::AtatClient, UrcChannel, UrcSubscription};
use embassy_time::{with_timeout, Duration, Timer};
use embedded_hal::digital::OutputPin as _;
use no_std_net::{Ipv4Addr, Ipv6Addr};

use crate::{
    command::{
        general::SoftwareVersion,
        network::{
            responses::NetworkStatusResponse,
            types::{InterfaceType, NetworkStatus, NetworkStatusParameter},
            urc::{NetworkDown, NetworkUp},
            GetNetworkStatus,
        },
        system::{types::EchoOn, RebootDCE, SetEcho, StoreCurrentConfig},
        wifi::{
            types::{DisconnectReason, PowerSaveMode, WifiConfig as WifiConfigParam},
            urc::{WifiLinkConnected, WifiLinkDisconnected},
            SetWifiConfig,
        },
        OnOff, Urc,
    },
    connection::WiFiState,
    error::Error,
    network::WifiNetwork,
    WifiConfig,
};

use super::{runner::URC_SUBSCRIBERS, state, LinkState, UbloxUrc};

pub(crate) struct NetDevice<'a, 'b, C, A, const URC_CAPACITY: usize> {
    ch: &'b state::Runner<'a>,
    config: &'b mut C,
    at_client: A,
    urc_subscription: UrcSubscription<'a, UbloxUrc, URC_CAPACITY, URC_SUBSCRIBERS>,
}

impl<'a, 'b, C, A, const URC_CAPACITY: usize> NetDevice<'a, 'b, C, A, URC_CAPACITY>
where
    C: WifiConfig<'a>,
    A: AtatClient,
{
    pub fn new(
        ch: &'b state::Runner<'a>,
        config: &'b mut C,
        at_client: A,
        urc_channel: &'a UrcChannel<UbloxUrc, URC_CAPACITY, URC_SUBSCRIBERS>,
    ) -> Self {
        Self {
            ch,
            config,
            at_client,
            urc_subscription: urc_channel.subscribe().unwrap(),
        }
    }

    pub(crate) async fn init(&mut self) -> Result<(), Error> {
        // Initialize a new ublox device to a known state (set RS232 settings)
        debug!("Initializing module");
        // Hard reset module
        self.reset().await?;

        self.at_client.send(&SoftwareVersion).await?;
        self.at_client.send(&SetEcho { on: EchoOn::Off }).await?;
        self.at_client
            .send(&SetWifiConfig {
                config_param: WifiConfigParam::DropNetworkOnLinkLoss(OnOff::On),
            })
            .await?;

        // Disable all power savings for now
        self.at_client
            .send(&SetWifiConfig {
                config_param: WifiConfigParam::PowerSaveMode(PowerSaveMode::ActiveMode),
            })
            .await?;

        #[cfg(feature = "internal-network-stack")]
        if let Some(size) = C::TLS_IN_BUFFER_SIZE {
            self.at_client
                .send(&crate::command::data_mode::SetPeerConfiguration {
                    parameter: crate::command::data_mode::types::PeerConfigParameter::TlsInBuffer(
                        size,
                    ),
                })
                .await?;
        }

        #[cfg(feature = "internal-network-stack")]
        if let Some(size) = C::TLS_OUT_BUFFER_SIZE {
            self.at_client
                .send(&crate::command::data_mode::SetPeerConfiguration {
                    parameter: crate::command::data_mode::types::PeerConfigParameter::TlsOutBuffer(
                        size,
                    ),
                })
                .await?;
        }

        self.ch.mark_initialized();

        Ok(())
    }

    pub async fn run(&mut self) -> Result<(), Error> {
        if self.ch.link_state(None) == LinkState::Uninitialized {
            self.init().await?;
        }

        loop {
            let event = self.urc_subscription.next_message_pure().await;

            #[cfg(feature = "edm")]
            let Some(event) = event.extract_urc() else {
                continue;
            };

            self.handle_urc(event).await?;
        }
    }

    async fn handle_urc(&mut self, event: Urc) -> Result<(), Error> {
        debug!("GOT URC event");
        match event {
            Urc::StartUp => {
                error!("AT startup event?! Device restarted unintentionally!");
            }
            Urc::WifiLinkConnected(WifiLinkConnected {
                connection_id: _,
                bssid,
                channel,
            }) => self.ch.update_connection_with(|con| {
                con.wifi_state = WiFiState::Connected;
                con.network
                    .replace(WifiNetwork::new_station(bssid, channel));
            }),
            Urc::WifiLinkDisconnected(WifiLinkDisconnected { reason, .. }) => {
                self.ch.update_connection_with(|con| {
                    con.wifi_state = match reason {
                        DisconnectReason::NetworkDisabled => {
                            con.network.take();
                            warn!("Wifi network disabled!");
                            WiFiState::Inactive
                        }
                        DisconnectReason::SecurityProblems => {
                            error!("Wifi Security Problems");
                            WiFiState::SecurityProblems
                        }
                        _ => WiFiState::NotConnected,
                    }
                })
            }
            Urc::WifiAPUp(_) => warn!("Not yet implemented [WifiAPUp]"),
            Urc::WifiAPDown(_) => warn!("Not yet implemented [WifiAPDown]"),
            Urc::WifiAPStationConnected(_) => warn!("Not yet implemented [WifiAPStationConnected]"),
            Urc::WifiAPStationDisconnected(_) => {
                warn!("Not yet implemented [WifiAPStationDisconnected]")
            }
            Urc::EthernetLinkUp(_) => warn!("Not yet implemented [EthernetLinkUp]"),
            Urc::EthernetLinkDown(_) => warn!("Not yet implemented [EthernetLinkDown]"),
            Urc::NetworkUp(NetworkUp { interface_id }) => {
                drop(event);
                self.network_status_callback(interface_id).await?;
            }
            Urc::NetworkDown(NetworkDown { interface_id }) => {
                drop(event);
                self.network_status_callback(interface_id).await?;
            }
            Urc::NetworkError(_) => warn!("Not yet implemented [NetworkError]"),
            _ => {}
        }

        Ok(())
    }

    async fn network_status_callback(&mut self, interface_id: u8) -> Result<(), Error> {
        // Normally a check for this interface type being
        // `InterfaceType::WifiStation`` should be made but there is a bug in
        // uConnect which gives the type `InterfaceType::Unknown` when the
        // credentials have been restored from persistent memory. This although
        // the wifi station has been started. So we assume that this type is
        // also ok.
        let NetworkStatusResponse {
            status:
                NetworkStatus::InterfaceType(InterfaceType::WifiStation | InterfaceType::Unknown),
            ..
        } = self
            .at_client
            .send(&GetNetworkStatus {
                interface_id,
                status: NetworkStatusParameter::InterfaceType,
            })
            .await?
        else {
            return Err(Error::Network);
        };

        let NetworkStatusResponse {
            status: NetworkStatus::IPv4Address(ipv4),
            ..
        } = self
            .at_client
            .send(&GetNetworkStatus {
                interface_id,
                status: NetworkStatusParameter::IPv4Address,
            })
            .await?
        else {
            return Err(Error::Network);
        };

        let ipv4_up = core::str::from_utf8(ipv4.as_slice())
            .ok()
            .and_then(|s| Ipv4Addr::from_str(s).ok())
            .map(|ip| !ip.is_unspecified())
            .unwrap_or_default();

        let NetworkStatusResponse {
            status: NetworkStatus::IPv6LinkLocalAddress(ipv6),
            ..
        } = self
            .at_client
            .send(&GetNetworkStatus {
                interface_id,
                status: NetworkStatusParameter::IPv6LinkLocalAddress,
            })
            .await?
        else {
            return Err(Error::Network);
        };

        let ipv6_up = core::str::from_utf8(ipv6.as_slice())
            .ok()
            .and_then(|s| Ipv6Addr::from_str(s).ok())
            .map(|ip| !ip.is_unspecified())
            .unwrap_or_default();

        // Use `ipv4_up` & `ipv6_up` to determine link state
        self.ch
            .update_connection_with(|con| con.network_up = ipv4_up && ipv6_up);

        Ok(())
    }

    async fn wait_startup(&mut self, timeout: Duration) -> Result<(), Error> {
        let fut = async {
            loop {
                let event = self.urc_subscription.next_message_pure().await;

                #[cfg(feature = "edm")]
                let Some(event) = event.extract_urc() else {
                    continue;
                };

                match event {
                    Urc::StartUp => return,
                    _ => {}
                }
            }
        };

        with_timeout(timeout, fut).await.map_err(|_| Error::Timeout)
    }

    pub async fn reset(&mut self) -> Result<(), Error> {
        warn!("Hard resetting Ublox Short Range");
        self.config.reset_pin().unwrap().set_low().ok();
        Timer::after(Duration::from_millis(100)).await;
        self.config.reset_pin().unwrap().set_high().ok();

        self.wait_startup(Duration::from_secs(4)).await?;

        #[cfg(feature = "edm")]
        self.enter_edm(Duration::from_secs(4)).await?;

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn restart(&mut self, store: bool) -> Result<(), Error> {
        warn!("Soft resetting Ublox Short Range");
        if store {
            self.at_client.send(&StoreCurrentConfig).await?;
        }

        self.at_client.send(&RebootDCE).await?;

        self.wait_startup(Duration::from_secs(10)).await?;

        info!("Module started again");
        #[cfg(feature = "edm")]
        self.enter_edm(Duration::from_secs(4)).await?;

        Ok(())
    }

    #[cfg(feature = "edm")]
    pub async fn enter_edm(&mut self, timeout: Duration) -> Result<(), Error> {
        info!("Entering EDM mode");

        // Switch to EDM on Init. If in EDM, fail and check with autosense
        let fut = async {
            loop {
                // Ignore AT results until we are successful in EDM mode
                if let Ok(_) = self
                    .at_client
                    .send(&crate::command::edm::SwitchToEdmCommand)
                    .await
                {
                    // After executing the data mode command or the extended data
                    // mode command, a delay of 50 ms is required before start of
                    // data transmission.
                    Timer::after(Duration::from_millis(50)).await;
                    break;
                }
                Timer::after(Duration::from_millis(10)).await;
            }
        };

        with_timeout(timeout, fut)
            .await
            .map_err(|_| Error::Timeout)?;

        Ok(())
    }
}
