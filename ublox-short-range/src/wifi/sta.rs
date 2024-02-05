use crate::{
    client::UbloxClient,
    command::{
        edm::EdmAtCmdWrapper,
        wifi::{types::*, *},
        *,
    },
    error::{WifiConnectionError, WifiError},
    wifi::{
        connection::{WiFiState, WifiConnection},
        network::{WifiMode, WifiNetwork},
        options::ConnectionOptions,
    },
};

use atat::{blocking::AtatClient, heapless_bytes::Bytes};
use core::convert::TryFrom;
use embedded_hal::digital::OutputPin;
use heapless::Vec;

const CONFIG_ID: u8 = 0;

impl<'buf, 'sub, AtCl, AtUrcCh, RST, const N: usize, const L: usize>
    UbloxClient<'buf, 'sub, AtCl, AtUrcCh, RST, N, L>
where
    'buf: 'sub,
    AtCl: AtatClient,
    RST: OutputPin,
{
    /// Attempts to connect to a wireless network with the given connection options.
    pub fn connect(&mut self, options: &ConnectionOptions) -> Result<(), WifiConnectionError> {
        defmt::info!("Connecting to {:?}", options);
        // Network part

        if let Some(ref con) = self.wifi_connection {
            defmt::warn!("WaitingForWifiDeactivation {:#?}", con);
            if con.wifi_state != WiFiState::Inactive {
                return Err(WifiConnectionError::WaitingForWifiDeactivation);
            }
        }

        // Disable DHCP Client (static IP address will be used)
        if options.ip.is_some() || options.subnet.is_some() || options.gateway.is_some() {
            self.send_internal(
                &EdmAtCmdWrapper(SetWifiStationConfig {
                    config_id: CONFIG_ID,
                    config_param: WifiStationConfig::IPv4Mode(IPv4Mode::Static),
                }),
                true,
            )?;
        }

        // Network IP address
        if let Some(ip) = options.ip {
            self.send_internal(
                &EdmAtCmdWrapper(SetWifiStationConfig {
                    config_id: CONFIG_ID,
                    config_param: WifiStationConfig::IPv4Address(ip),
                }),
                true,
            )?;
        }
        // Network Subnet mask
        if let Some(subnet) = options.subnet {
            self.send_internal(
                &EdmAtCmdWrapper(SetWifiStationConfig {
                    config_id: CONFIG_ID,
                    config_param: WifiStationConfig::SubnetMask(subnet),
                }),
                true,
            )?;
        }
        // Network Default gateway
        if let Some(gateway) = options.gateway {
            self.send_internal(
                &EdmAtCmdWrapper(SetWifiStationConfig {
                    config_id: CONFIG_ID,
                    config_param: WifiStationConfig::DefaultGateway(gateway),
                }),
                true,
            )?;
        }

        // Wifi part
        // Set the Network SSID to connect to
        self.send_internal(
            &EdmAtCmdWrapper(SetWifiStationConfig {
                config_id: CONFIG_ID,
                config_param: WifiStationConfig::SSID(&options.ssid),
            }),
            true,
        )?;

        if let Some(ref pass) = options.password {
            // Use WPA2 as authentication type
            self.send_internal(
                &EdmAtCmdWrapper(SetWifiStationConfig {
                    config_id: CONFIG_ID,
                    config_param: WifiStationConfig::Authentication(Authentication::WpaWpa2Psk),
                }),
                true,
            )?;

            // Input passphrase
            self.send_internal(
                &EdmAtCmdWrapper(SetWifiStationConfig {
                    config_id: CONFIG_ID,
                    config_param: WifiStationConfig::WpaPskOrPassphrase(&pass),
                }),
                true,
            )?;
        }

        self.wifi_connection.replace(WifiConnection::new(
            WifiNetwork {
                bssid: Bytes::new(),
                op_mode: wifi::types::OperationMode::Infrastructure,
                ssid: options.ssid.clone(),
                channel: 0,
                rssi: 1,
                authentication_suites: 0,
                unicast_ciphers: 0,
                group_ciphers: 0,
                mode: WifiMode::Station,
            },
            WiFiState::NotConnected,
        ));
        self.send_internal(
            &EdmAtCmdWrapper(ExecWifiStationAction {
                config_id: CONFIG_ID,
                action: WifiStationAction::Activate,
            }),
            true,
        )?;

        // TODO: Await connected event?

        Ok(())
    }

    pub fn activate(&mut self) -> Result<(), WifiConnectionError> {
        self.send_internal(
            &EdmAtCmdWrapper(ExecWifiStationAction {
                config_id: CONFIG_ID,
                action: WifiStationAction::Activate,
            }),
            true,
        )?;
        return Ok(());
    }

    pub fn scan(&mut self) -> Result<Vec<WifiNetwork, 32>, WifiError> {
        match self.send_internal(&EdmAtCmdWrapper(WifiScan { ssid: None }), true) {
            Ok(resp) => resp
                .network_list
                .into_iter()
                .map(WifiNetwork::try_from)
                .collect(),
            Err(_) => Err(WifiError::UnexpectedResponse),
        }
    }

    pub fn is_connected(&self) -> bool {
        if !self.initialized {
            return false;
        }

        self.wifi_connection
            .as_ref()
            .map(|c| c.is_connected())
            .unwrap_or_default()
    }

    pub fn is_active_on_startup(&mut self) -> Result<bool, WifiConnectionError> {
        if let Ok(resp) = self.send_internal(
            &EdmAtCmdWrapper(GetWifiStationConfig {
                config_id: CONFIG_ID,
                parameter: Some(WifiStationConfigParameter::ActiveOnStartup),
            }),
            false,
        ) {
            if let WifiStationConfigR::ActiveOnStartup(active) = resp.parameter {
                return Ok(active == OnOff::On);
            }
        }
        Err(WifiConnectionError::Illegal)
    }

    pub fn get_ssid(&mut self) -> Result<heapless::String<64>, WifiConnectionError> {
        if let Ok(resp) = self.send_internal(
            &EdmAtCmdWrapper(GetWifiStationConfig {
                config_id: CONFIG_ID,
                parameter: Some(WifiStationConfigParameter::SSID),
            }),
            false,
        ) {
            if let WifiStationConfigR::SSID(ssid) = resp.parameter {
                return Ok(ssid);
            }
        };
        return Err(WifiConnectionError::Illegal);
    }

    pub fn reset_config_profile(&mut self) -> Result<(), WifiConnectionError> {
        self.send_at(EdmAtCmdWrapper(ExecWifiStationAction {
            config_id: CONFIG_ID,
            action: WifiStationAction::Reset,
        }))?;
        Ok(())
    }

    pub fn disconnect(&mut self) -> Result<(), WifiConnectionError> {
        defmt::debug!("Disconnecting");
        self.send_at(EdmAtCmdWrapper(ExecWifiStationAction {
            config_id: CONFIG_ID,
            action: WifiStationAction::Deactivate,
        }))?;
        Ok(())
    }
}
