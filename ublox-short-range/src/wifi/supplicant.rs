use embedded_nal::nb;
use heapless::Vec;

use crate::{
    command::{
        edm::EdmAtCmdWrapper,
        wifi::{
            responses::GetWifiStationConfigResponse,
            types::{
                Authentication, IPv4Mode, WifiStationAction, WifiStationConfig,
                WifiStationConfigParameter, WifiStationConfigR,
            },
            ExecWifiStationAction, GetWifiStationConfig, SetWifiStationConfig, WifiScan,
        },
    },
    error::{Error, WifiConnectionError, WifiError},
};

use super::{
    connection::{WiFiState, WifiConnection},
    network::WifiNetwork,
    options::ConnectionOptions,
};

pub struct Supplicant<'a, C, const N: usize> {
    pub(crate) client: &'a mut C,
    pub(crate) wifi_connection: &'a mut Option<WifiConnection>,
}

impl<'a, C, const N: usize> Supplicant<'a, C, N>
where
    C: atat::AtatClient,
{
    fn send_at<A: atat::AtatCmd<LEN>, const LEN: usize>(
        &mut self,
        req: &A,
    ) -> Result<A::Response, Error> {
        self.client.send(req).map_err(|e| match e {
            nb::Error::Other(ate) => {
                defmt::error!("{:?}: {=[u8]:a}", ate, req.as_bytes());
                ate.into()
            }
            nb::Error::WouldBlock => Error::_Unknown,
        })
    }

    pub fn load(&mut self) -> Result<(), Error> {
        for config_id in 0..N as u8 {
            self.client
                .send(&EdmAtCmdWrapper(ExecWifiStationAction {
                    config_id,
                    action: WifiStationAction::Load,
                }))
                .ok();
        }
        Ok(())
    }

    pub fn get_connection(&mut self, config_id: u8) -> Result<Option<ConnectionOptions>, Error> {
        let GetWifiStationConfigResponse {
            parameter: ip_mode, ..
        } = self.send_at(&EdmAtCmdWrapper(GetWifiStationConfig {
            config_id,
            parameter: Some(WifiStationConfigParameter::IPv4Mode),
        }))?;

        let mut options = ConnectionOptions {
            ssid: heapless::String::new(),
            password: None,
            ip: None,
            subnet: None,
            gateway: None,
        };

        let GetWifiStationConfigResponse { parameter, .. } =
            self.send_at(&EdmAtCmdWrapper(GetWifiStationConfig {
                config_id,
                parameter: Some(WifiStationConfigParameter::SSID),
            }))?;

        if let WifiStationConfigR::SSID(ssid) = parameter {
            if ssid.is_empty() {
                return Ok(None);
            }
            options.ssid = ssid;
        }

        let GetWifiStationConfigResponse { parameter, .. } =
            self.send_at(&EdmAtCmdWrapper(GetWifiStationConfig {
                config_id,
                parameter: Some(WifiStationConfigParameter::Authentication),
            }))?;

        if let WifiStationConfigR::Authentication(auth) = parameter {
            if !matches!(auth, Authentication::Open) {
                options.password = Some(heapless::String::from("***"));
            }
        }

        if let WifiStationConfigR::IPv4Mode(IPv4Mode::Static) = ip_mode {
            let GetWifiStationConfigResponse { parameter, .. } =
                self.send_at(&EdmAtCmdWrapper(GetWifiStationConfig {
                    config_id,
                    parameter: Some(WifiStationConfigParameter::IPv4Address),
                }))?;

            if let WifiStationConfigR::IPv4Address(ip) = parameter {
                options.ip = Some(ip);
            }

            let GetWifiStationConfigResponse { parameter, .. } =
                self.send_at(&EdmAtCmdWrapper(GetWifiStationConfig {
                    config_id,
                    parameter: Some(WifiStationConfigParameter::SubnetMask),
                }))?;

            if let WifiStationConfigR::SubnetMask(subnet) = parameter {
                options.subnet = Some(subnet);
            }

            let GetWifiStationConfigResponse { parameter, .. } =
                self.send_at(&EdmAtCmdWrapper(GetWifiStationConfig {
                    config_id,
                    parameter: Some(WifiStationConfigParameter::DefaultGateway),
                }))?;

            if let WifiStationConfigR::DefaultGateway(gateway) = parameter {
                options.gateway = Some(gateway);
            }
        }

        Ok(Some(options))
    }

    pub fn list_connections(&mut self) -> Result<Vec<(u8, ConnectionOptions), N>, Error> {
        Ok((0..N as u8)
            .filter_map(|config_id| {
                self.get_connection(config_id)
                    .unwrap()
                    .map(|c| (config_id, c))
            })
            .collect())
    }

    pub fn remove_connection(&mut self, config_id: u8) -> Result<(), WifiConnectionError> {
        self.deactivate(config_id)?;

        if let Some(ref con) = self.wifi_connection {
            if con.config_id == config_id && con.wifi_state != WiFiState::Inactive {
                return Err(WifiConnectionError::WaitingForWifiDeactivation);
            }
        }

        self.send_at(&EdmAtCmdWrapper(ExecWifiStationAction {
            config_id,
            action: WifiStationAction::Reset,
        }))?;

        self.send_at(&EdmAtCmdWrapper(ExecWifiStationAction {
            config_id,
            action: WifiStationAction::Store,
        }))?;

        Ok(())
    }

    /// Attempts to connect to a wireless network with the given connection options.
    pub fn upsert_connection(
        &mut self,
        id: u8,
        options: &ConnectionOptions,
    ) -> Result<(), WifiConnectionError> {
        // Network part
        // Deactivate & reset network config slot
        self.remove_connection(id)?;

        // Disable DHCP Client (static IP address will be used)
        if options.ip.is_some() || options.subnet.is_some() || options.gateway.is_some() {
            self.send_at(&EdmAtCmdWrapper(SetWifiStationConfig {
                config_id: id,
                config_param: WifiStationConfig::IPv4Mode(IPv4Mode::Static),
            }))?;
        }

        // Network IP address
        if let Some(ip) = options.ip {
            self.send_at(&EdmAtCmdWrapper(SetWifiStationConfig {
                config_id: id,
                config_param: WifiStationConfig::IPv4Address(ip),
            }))?;
        }
        // Network Subnet mask
        if let Some(subnet) = options.subnet {
            self.send_at(&EdmAtCmdWrapper(SetWifiStationConfig {
                config_id: id,
                config_param: WifiStationConfig::SubnetMask(subnet),
            }))?;
        }
        // Network Default gateway
        if let Some(gateway) = options.gateway {
            self.send_at(&EdmAtCmdWrapper(SetWifiStationConfig {
                config_id: id,
                config_param: WifiStationConfig::DefaultGateway(gateway),
            }))?;
        }

        // Wifi part
        // Set the Network SSID to connect to
        self.send_at(&EdmAtCmdWrapper(SetWifiStationConfig {
            config_id: id,
            config_param: WifiStationConfig::SSID(options.ssid.clone()),
        }))?;

        if let Some(pass) = options.password.clone() {
            // Use WPA2 as authentication type
            self.send_at(&EdmAtCmdWrapper(SetWifiStationConfig {
                config_id: id,
                config_param: WifiStationConfig::Authentication(Authentication::WpaWpa2Psk),
            }))?;

            // Input passphrase
            self.send_at(&EdmAtCmdWrapper(SetWifiStationConfig {
                config_id: id,
                config_param: WifiStationConfig::WpaPskOrPassphrase(pass),
            }))?;
        } else {
            self.send_at(&EdmAtCmdWrapper(SetWifiStationConfig {
                config_id: id,
                config_param: WifiStationConfig::Authentication(Authentication::Open),
            }))?;
        }

        self.send_at(&EdmAtCmdWrapper(ExecWifiStationAction {
            config_id: id,
            action: WifiStationAction::Store,
        }))?;

        Ok(())
    }

    pub fn activate(&mut self, config_id: u8) -> Result<(), WifiConnectionError> {
        self.send_at(&EdmAtCmdWrapper(ExecWifiStationAction {
            config_id,
            action: WifiStationAction::Activate,
        }))?;

        self.wifi_connection.replace(WifiConnection::new(
            WifiNetwork {
                bssid: atat::heapless_bytes::Bytes::new(),
                op_mode: crate::command::wifi::types::OperationMode::Infrastructure,
                ssid: heapless::String::new(),
                channel: 0,
                rssi: 1,
                authentication_suites: 0,
                unicast_ciphers: 0,
                group_ciphers: 0,
                mode: super::network::WifiMode::Station,
            },
            WiFiState::NotConnected,
            config_id,
        ));

        Ok(())
    }

    pub fn deactivate(&mut self, config_id: u8) -> Result<(), WifiConnectionError> {
        self.send_at(&EdmAtCmdWrapper(ExecWifiStationAction {
            config_id,
            action: WifiStationAction::Deactivate,
        }))?;

        self.wifi_connection.take();

        Ok(())
    }

    pub fn scan(&mut self) -> Result<Vec<WifiNetwork, 32>, WifiError> {
        match self.send_at(&EdmAtCmdWrapper(WifiScan { ssid: None })) {
            Ok(resp) => resp
                .network_list
                .into_iter()
                .map(WifiNetwork::try_from)
                .collect(),
            Err(_) => Err(WifiError::UnexpectedResponse),
        }
    }

    pub fn is_connected(&self) -> bool {
        self.wifi_connection
            .as_ref()
            .map(WifiConnection::is_connected)
            .unwrap_or_default()
    }
}
