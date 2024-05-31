use core::future::poll_fn;
use core::task::Poll;

use atat::{
    asynch::{AtatClient, SimpleClient},
    UrcChannel, UrcSubscription,
};
use embassy_net::{
    udp::{PacketMetadata, UdpSocket},
    Ipv4Address,
};
use embassy_time::{with_timeout, Duration};

use crate::command::gpio::{
    types::{GPIOId, GPIOValue},
    WriteGPIO,
};
use crate::command::network::SetNetworkHostName;
use crate::command::system::{RebootDCE, ResetToFactoryDefaults};
use crate::command::wifi::types::{
    Authentication, StatusId, WifiStationAction, WifiStationConfig, WifiStatus, WifiStatusVal,
};
use crate::command::wifi::{ExecWifiStationAction, GetWifiStatus, SetWifiStationConfig};
use crate::command::OnOff;
use crate::error::Error;

use super::state::LinkState;
use super::{at_udp_socket::AtUdpSocket, runner::URC_SUBSCRIBERS};
use super::{state, UbloxUrc};

const CONFIG_ID: u8 = 0;

const MAX_COMMAND_LEN: usize = 128;

// TODO: Can this be made in a more intuitive way?
pub struct ControlResources {
    rx_meta: [PacketMetadata; 1],
    tx_meta: [PacketMetadata; 1],
    socket_rx_buf: [u8; 32],
    socket_tx_buf: [u8; 32],
    at_buf: [u8; MAX_COMMAND_LEN],
}

impl ControlResources {
    pub const fn new() -> Self {
        Self {
            rx_meta: [PacketMetadata::EMPTY; 1],
            tx_meta: [PacketMetadata::EMPTY; 1],
            socket_rx_buf: [0u8; 32],
            socket_tx_buf: [0u8; 32],
            at_buf: [0u8; MAX_COMMAND_LEN],
        }
    }
}

pub struct Control<'a, 'r, const URC_CAPACITY: usize> {
    state_ch: state::Runner<'a>,
    at_client: SimpleClient<'r, AtUdpSocket<'r>, atat::DefaultDigester<UbloxUrc>>,
    _urc_subscription: UrcSubscription<'a, UbloxUrc, URC_CAPACITY, URC_SUBSCRIBERS>,
}

impl<'a, 'r, const URC_CAPACITY: usize> Control<'a, 'r, URC_CAPACITY> {
    pub(crate) fn new<D: embassy_net::driver::Driver>(
        state_ch: state::Runner<'a>,
        urc_channel: &'a UrcChannel<UbloxUrc, URC_CAPACITY, URC_SUBSCRIBERS>,
        resources: &'r mut ControlResources,
        stack: &'r embassy_net::Stack<D>,
    ) -> Self {
        let mut socket = UdpSocket::new(
            stack,
            &mut resources.rx_meta,
            &mut resources.socket_rx_buf,
            &mut resources.tx_meta,
            &mut resources.socket_tx_buf,
        );

        info!("Socket bound!");
        socket
            .bind((Ipv4Address::new(172, 30, 0, 252), AtUdpSocket::PPP_AT_PORT))
            .unwrap();

        let at_client = SimpleClient::new(
            AtUdpSocket(socket),
            atat::AtDigester::<UbloxUrc>::new(),
            &mut resources.at_buf,
            atat::Config::default(),
        );

        Self {
            state_ch,
            at_client,
            _urc_subscription: urc_channel.subscribe().unwrap(),
        }
    }

    pub async fn set_hostname(&mut self, hostname: &str) -> Result<(), Error> {
        self.at_client
            .send(&SetNetworkHostName {
                host_name: hostname,
            })
            .await?;
        Ok(())
    }

    async fn get_wifi_status(&mut self) -> Result<WifiStatusVal, Error> {
        match self
            .at_client
            .send(&GetWifiStatus {
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
            .at_client
            .send(&GetWifiStatus {
                status_id: StatusId::SSID,
            })
            .await?
            .status_id
        {
            WifiStatus::SSID(s) => Ok(s),
            _ => Err(Error::AT(atat::Error::InvalidResponse)),
        }
    }

    pub async fn factory_reset(&mut self) -> Result<(), Error> {
        self.at_client.send(&ResetToFactoryDefaults).await?;
        self.at_client.send(&RebootDCE).await?;

        Ok(())
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

        self.at_client
            .send(&ExecWifiStationAction {
                config_id: CONFIG_ID,
                action: WifiStationAction::Reset,
            })
            .await?;

        self.at_client
            .send(&SetWifiStationConfig {
                config_id: CONFIG_ID,
                config_param: WifiStationConfig::ActiveOnStartup(OnOff::Off),
            })
            .await?;

        self.at_client
            .send(&SetWifiStationConfig {
                config_id: CONFIG_ID,
                config_param: WifiStationConfig::SSID(
                    heapless::String::try_from(ssid).map_err(|_| Error::Overflow)?,
                ),
            })
            .await?;

        self.at_client
            .send(&SetWifiStationConfig {
                config_id: CONFIG_ID,
                config_param: WifiStationConfig::Authentication(Authentication::Open),
            })
            .await?;

        self.at_client
            .send(&ExecWifiStationAction {
                config_id: CONFIG_ID,
                action: WifiStationAction::Activate,
            })
            .await?;

        with_timeout(Duration::from_secs(25), self.wait_for_join(ssid))
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

        self.at_client
            .send(&ExecWifiStationAction {
                config_id: CONFIG_ID,
                action: WifiStationAction::Reset,
            })
            .await?;

        self.at_client
            .send(&SetWifiStationConfig {
                config_id: CONFIG_ID,
                config_param: WifiStationConfig::ActiveOnStartup(OnOff::Off),
            })
            .await?;

        self.at_client
            .send(&SetWifiStationConfig {
                config_id: CONFIG_ID,
                config_param: WifiStationConfig::SSID(
                    heapless::String::try_from(ssid).map_err(|_| Error::Overflow)?,
                ),
            })
            .await?;

        self.at_client
            .send(&SetWifiStationConfig {
                config_id: CONFIG_ID,
                config_param: WifiStationConfig::Authentication(Authentication::WpaWpa2Psk),
            })
            .await?;

        self.at_client
            .send(&SetWifiStationConfig {
                config_id: CONFIG_ID,
                config_param: WifiStationConfig::WpaPskOrPassphrase(
                    heapless::String::try_from(passphrase).map_err(|_| Error::Overflow)?,
                ),
            })
            .await?;

        self.at_client
            .send(&ExecWifiStationAction {
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
                self.at_client
                    .send(&ExecWifiStationAction {
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
        self.at_client.send(&WriteGPIO { id, value }).await?;
        Ok(())
    }

    // FIXME: This could probably be improved
    #[cfg(feature = "internal-network-stack")]
    pub async fn import_credentials(
        &mut self,
        data_type: SecurityDataType,
        name: &str,
        data: &[u8],
        md5_sum: Option<&str>,
    ) -> Result<(), atat::Error> {
        assert!(name.len() < 16);

        info!("Importing {:?} bytes as {:?}", data.len(), name);

        self.at_client
            .send(&PrepareSecurityDataImport {
                data_type,
                data_size: data.len(),
                internal_name: name,
                password: None,
            })
            .await?;

        let import_data = self
            .at_client
            .send(&SendSecurityDataImport {
                data: atat::serde_bytes::Bytes::new(data),
            })
            .await?;

        if let Some(hash) = md5_sum {
            assert_eq!(import_data.md5_string.as_str(), hash);
        }

        Ok(())
    }
}
