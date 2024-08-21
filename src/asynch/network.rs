use core::str::FromStr as _;

use atat::{asynch::AtatClient, UrcChannel, UrcSubscription};
use embassy_time::{with_timeout, Duration, Timer};
use embedded_hal::digital::OutputPin as _;
use no_std_net::{Ipv4Addr, Ipv6Addr};

use crate::{
    command::{
        network::{
            responses::NetworkStatusResponse,
            types::{InterfaceType, NetworkStatus, NetworkStatusParameter},
            urc::{NetworkDown, NetworkUp},
            GetNetworkStatus,
        },
        system::{RebootDCE, StoreCurrentConfig},
        wifi::{
            types::DisconnectReason,
            urc::{WifiLinkConnected, WifiLinkDisconnected},
        },
        Urc,
    },
    connection::WiFiState,
    error::Error,
    network::WifiNetwork,
    WifiConfig,
};

use super::{runner::URC_SUBSCRIBERS, state, UbloxUrc};

pub(crate) struct NetDevice<'a, 'b, C, A, const URC_CAPACITY: usize> {
    ch: &'b state::Runner<'a>,
    config: &'b mut C,
    at_client: A,
    urc_subscription: UrcSubscription<'a, UbloxUrc, URC_CAPACITY, { URC_SUBSCRIBERS }>,
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
        urc_channel: &'a UrcChannel<UbloxUrc, URC_CAPACITY, { URC_SUBSCRIBERS }>,
    ) -> Self {
        Self {
            ch,
            config,
            at_client,
            urc_subscription: urc_channel.subscribe().unwrap(),
        }
    }

    pub async fn run(&mut self) -> Result<(), Error> {
        loop {
            match embassy_futures::select::select(
                self.urc_subscription.next_message_pure(),
                self.ch.wait_for_wifi_state_change(),
            )
            .await
            {
                embassy_futures::select::Either::First(event) => {
                    #[cfg(feature = "edm")]
                    let Some(event) = event.extract_urc() else {
                        continue;
                    };

                    self.handle_urc(event).await?;
                }
                _ => {}
            }

            if self.ch.wifi_state(None) == WiFiState::Inactive && self.ch.connection_down(None) {
                return Ok(());
            }
        }
    }

    async fn handle_urc(&mut self, event: Urc) -> Result<(), Error> {
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
            Urc::WifiAPUp(_) => self.ch.update_connection_with(|con| {
                con.wifi_state = WiFiState::Connected;
                con.network.replace(WifiNetwork::new_ap());
            }),
            Urc::WifiAPDown(_) => self.ch.update_connection_with(|con| {
                con.network.take();
                con.wifi_state = WiFiState::Inactive;
            }),
            Urc::WifiAPStationConnected(_) => warn!("Not yet implemented [WifiAPStationConnected]"),
            Urc::WifiAPStationDisconnected(_) => {
                warn!("Not yet implemented [WifiAPStationDisconnected]")
            }
            Urc::EthernetLinkUp(_) => warn!("Not yet implemented [EthernetLinkUp]"),
            Urc::EthernetLinkDown(_) => warn!("Not yet implemented [EthernetLinkDown]"),
            Urc::NetworkUp(NetworkUp { interface_id }) => {
                //self.network_status_callback(interface_id).await?;
                self.ch.update_connection_with(|con| {
                    con.ipv6_link_local_up = true;
                    con.ipv4_up = true;

                    #[cfg(feature = "ipv6")]
                    {
                        con.ipv6_up = ipv6_up
                    }
                });
            }
            Urc::NetworkDown(NetworkDown { interface_id }) => {
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
                NetworkStatus::InterfaceType(
                    InterfaceType::WifiStation
                    | InterfaceType::Unknown
                    | InterfaceType::WifiAccessPoint,
                ),
            ..
        } = self
            .at_client
            .send_retry(&GetNetworkStatus {
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
            .send_retry(&GetNetworkStatus {
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

        #[cfg(feature = "ipv6")]
        let ipv6_up = {
            let NetworkStatusResponse {
                status: NetworkStatus::IPv6Address1(ipv6),
                ..
            } = self
                .at_client
                .send_retry(&GetNetworkStatus {
                    interface_id,
                    status: NetworkStatusParameter::IPv6Address1,
                })
                .await?
            else {
                return Err(Error::Network);
            };

            core::str::from_utf8(ipv6.as_slice())
                .ok()
                .and_then(|s| Ipv6Addr::from_str(s).ok())
                .map(|ip| !ip.is_unspecified())
                .unwrap_or_default()
        };

        let NetworkStatusResponse {
            status: NetworkStatus::IPv6LinkLocalAddress(ipv6_link_local),
            ..
        } = self
            .at_client
            .send_retry(&GetNetworkStatus {
                interface_id,
                status: NetworkStatusParameter::IPv6LinkLocalAddress,
            })
            .await?
        else {
            return Err(Error::Network);
        };

        let ipv6_link_local_up = core::str::from_utf8(ipv6_link_local.as_slice())
            .ok()
            .and_then(|s| Ipv6Addr::from_str(s).ok())
            .map(|ip| !ip.is_unspecified())
            .unwrap_or_default();

        // Use `ipv4_addr` & `ipv6_addr` to determine link state
        self.ch.update_connection_with(|con| {
            con.ipv6_link_local_up = ipv6_link_local_up;
            con.ipv4_up = ipv4_up;

            #[cfg(feature = "ipv6")]
            {
                con.ipv6_up = ipv6_up
            }
        });

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

                if let Urc::StartUp = event {
                    return;
                }
            }
        };

        with_timeout(timeout, fut).await.map_err(|_| Error::Timeout)
    }

    pub async fn reset(&mut self) -> Result<(), Error> {
        if let Some(reset_pin) = self.config.reset_pin() {
            warn!("Reset pin found! Hard resetting Ublox Short Range");
            reset_pin.set_low().ok();
            Timer::after(Duration::from_millis(100)).await;
            reset_pin.set_high().ok();
        } else {
            warn!("No reset pin found! Soft resetting Ublox Short Range");
            self.at_client.send_retry(&RebootDCE).await?;
        }

        self.ch.mark_uninitialized();

        self.wait_startup(Duration::from_secs(5)).await?;

        #[cfg(feature = "edm")]
        self.enter_edm(Duration::from_secs(4)).await?;

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn restart(&mut self, store: bool) -> Result<(), Error> {
        warn!("Soft resetting Ublox Short Range");
        if store {
            self.at_client.send_retry(&StoreCurrentConfig).await?;
        }

        self.at_client.send_retry(&RebootDCE).await?;

        self.ch.mark_uninitialized();

        self.wait_startup(Duration::from_secs(5)).await?;

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
                    .send_retry(&crate::command::edm::SwitchToEdmCommand)
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
