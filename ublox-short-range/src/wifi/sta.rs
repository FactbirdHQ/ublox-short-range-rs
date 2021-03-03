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
use atat::serde_at::CharVec;
use atat::AtatClient;

// use core::convert::TryFrom;
use core::convert::TryFrom;
use heapless::{consts, ArrayLength, Vec};

/// Wireless network connectivity functionality.
pub trait WifiConnectivity {
    /// Makes an attempt to connect to a selected wireless network with password specified.
    fn connect(&self, options: ConnectionOptions) -> Result<(), WifiConnectionError>;

    fn scan(&self) -> Result<Vec<WifiNetwork, consts::U32>, WifiError>;

    fn is_connected(&self) -> bool;

    fn disconnect(&self) -> Result<(), WifiConnectionError>;
}

impl<T, N, L> WifiConnectivity for UbloxClient<T, N, L>
where
    T: AtatClient,
    N: ArrayLength<Option<crate::sockets::SocketSetItem<L>>>,
    L: ArrayLength<u8>,
{
    /// Attempts to connect to a wireless network with the given connection options.
    fn connect(&self, options: ConnectionOptions) -> Result<(), WifiConnectionError> {
        let mut config_id: u8 = 0;
        if let Some(config_id_option) = options.config_id {
            config_id = config_id_option;
        }

        // Network part
        // Deactivate network id 0
        self.send_internal(
            &EdmAtCmdWrapper(ExecWifiStationAction {
                config_id: config_id,
                action: WifiStationAction::Deactivate,
            }),
            true,
        )?;

        if let Some(ref con) = *self.wifi_connection.try_borrow()? {
            if con.wifi_state != WiFiState::Inactive {
                return Err(WifiConnectionError::WaitingForWifiDeactivation);
            }
        }

        // Disable DHCP Client (static IP address will be used)
        if options.ip.is_some() || options.subnet.is_some() || options.gateway.is_some() {
            self.send_internal(
                &EdmAtCmdWrapper(SetWifiStationConfig {
                    config_id: config_id,
                    config_param: WifiStationConfig::IPv4Mode(IPv4Mode::Static),
                }),
                true,
            )?;
        }

        // Network IP address
        if let Some(ip) = options.ip {
            self.send_internal(
                &EdmAtCmdWrapper(SetWifiStationConfig {
                    config_id: config_id,
                    config_param: WifiStationConfig::IPv4Address(ip),
                }),
                true,
            )?;
        }
        // Network Subnet mask
        if let Some(subnet) = options.subnet {
            self.send_internal(
                &EdmAtCmdWrapper(SetWifiStationConfig {
                    config_id: config_id,
                    config_param: WifiStationConfig::SubnetMask(subnet),
                }),
                true,
            )?;
        }
        // Network Default gateway
        if let Some(gateway) = options.gateway {
            self.send_internal(
                &EdmAtCmdWrapper(SetWifiStationConfig {
                    config_id: config_id,
                    config_param: WifiStationConfig::DefaultGateway(gateway),
                }),
                true,
            )?;
        }

        // Active on startup
        // self.send_internal(&SetWifiStationConfig{
        //     config_id: config_id,
        //     config_param: WifiStationConfig::ActiveOnStartup(OnOff::On),
        // }, true)?;

        // Wifi part
        // Set the Network SSID to connect to
        self.send_internal(
            &EdmAtCmdWrapper(SetWifiStationConfig {
                config_id: config_id,
                config_param: WifiStationConfig::SSID(&options.ssid),
            }),
            true,
        )?;

        if let Some(pass) = options.password {
            // Use WPA2 as authentication type
            self.send_internal(
                &EdmAtCmdWrapper(SetWifiStationConfig {
                    config_id: config_id,
                    config_param: WifiStationConfig::Authentication(Authentication::WpaWpa2Psk),
                }),
                true,
            )?;

            // Input passphrase
            self.send_internal(
                &EdmAtCmdWrapper(SetWifiStationConfig {
                    config_id: config_id,
                    config_param: WifiStationConfig::WpaPskOrPassphrase(&pass),
                }),
                true,
            )?;
        }

        *self.wifi_connection.try_borrow_mut()? = Some(WifiConnection::new(
            WifiNetwork {
                bssid: CharVec::new(),
                op_mode: wifi::types::OperationMode::AdHoc,
                ssid: options.ssid,
                channel: 0,
                rssi: 1,
                authentication_suites: 0,
                unicast_ciphers: 0,
                group_ciphers: 0,
                mode: WifiMode::AccessPoint,
            },
            WiFiState::NotConnected,
            config_id,
        ));
        self.send_internal(
            &EdmAtCmdWrapper(ExecWifiStationAction {
                config_id: config_id,
                action: WifiStationAction::Activate,
            }),
            true,
        )?;

        // TODO: Await connected event?
        // block!(wait_for_unsolicited!(self, UnsolicitedResponse::NetworkUp { .. })).unwrap();

        Ok(())
    }

    fn scan(&self) -> Result<Vec<WifiNetwork, consts::U32>, WifiError> {
        match self.send_internal(&EdmAtCmdWrapper(WifiScan { ssid: None }), true) {
            Ok(resp) => resp
                .network_list
                .into_iter()
                .map(WifiNetwork::try_from)
                .collect(),
            Err(_) => Err(WifiError::UnexpectedResponse),
        }
    }

    fn is_connected(&self) -> bool {
        if !self.initialized.get() {
            return false;
        }

        if let Ok(mut some) = self.wifi_connection.try_borrow_mut() {
            if let Some(ref mut con) = *some {
                if con.is_connected() {
                    return true;
                }
            }
        }
        false
    }

    fn disconnect(&self) -> Result<(), WifiConnectionError> {
        if let Some(ref mut con) = *self.wifi_connection.try_borrow_mut()? {
            match con.wifi_state {
                WiFiState::Connected | WiFiState::NotConnected => {
                    // con.wifi_state = WiFiState::Inactive;
                    self.send_internal(
                        &EdmAtCmdWrapper(ExecWifiStationAction {
                            config_id: 0,
                            action: WifiStationAction::Deactivate,
                        }),
                        true,
                    )?;
                }
                WiFiState::Inactive => {}
            }
        } else {
            return Err(WifiConnectionError::FailedToDisconnect);
        }
        Ok(())
    }
}

// #[cfg(test)]
// mod tests {
//
// }
