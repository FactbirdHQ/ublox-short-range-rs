use core::convert::TryInto;

use crate::{
    client::UbloxClient,
    command::{
        edm::EdmAtCmdWrapper,
        wifi::{
            self,
            types::{
                AccessPointAction, AccessPointConfig, AccessPointId, IPv4Mode, OnOff, PasskeyR,
                SecurityMode, SecurityModePSK,
            },
            SetWifiAPConfig, WifiAPAction,
        },
    },
    error::WifiHotspotError,
    wifi::{
        network::{WifiMode, WifiNetwork},
        options::{ConnectionOptions, HotspotOptions},
    },
};
use atat::heapless_bytes::Bytes;
use atat::AtatClient;
use embedded_hal::digital::OutputPin;
use embedded_time::{
    duration::{Generic, Milliseconds},
    Clock,
};

use super::connection::{WiFiState, WifiConnection};

impl<C, CLK, RST, const N: usize, const L: usize> UbloxClient<C, CLK, RST, N, L>
where
    C: AtatClient,
    CLK: Clock,
    RST: OutputPin,
    Generic<CLK::T>: TryInto<Milliseconds>,
{
    /// Creates wireless hotspot service for host machine.
    pub fn create_hotspot(
        &mut self,
        options: ConnectionOptions,
        configuration: HotspotOptions,
    ) -> Result<(), WifiHotspotError> {
        let ap_config_id = AccessPointId::Id0;

        // Network part
        // Deactivate network id 0
        self.send_internal(
            &EdmAtCmdWrapper(WifiAPAction {
                ap_config_id,
                ap_action: AccessPointAction::Deactivate,
            }),
            true,
        )?;

        self.send_internal(
            &EdmAtCmdWrapper(WifiAPAction {
                ap_config_id,
                ap_action: AccessPointAction::Reset,
            }),
            true,
        )?;

        if let Some(ref con) = self.wifi_connection {
            if con.wifi_state != WiFiState::Inactive {
                return Err(WifiHotspotError::CreationFailed);
            }
        }

        // Disable DHCP Server (static IP address will be used)
        if options.ip.is_some() || options.subnet.is_some() || options.gateway.is_some() {
            self.send_internal(
                &EdmAtCmdWrapper(SetWifiAPConfig {
                    ap_config_id,
                    ap_config_param: AccessPointConfig::IPv4Mode(IPv4Mode::Static),
                }),
                true,
            )?;
        }

        // Network IP address
        if let Some(ip) = options.ip {
            self.send_internal(
                &EdmAtCmdWrapper(SetWifiAPConfig {
                    ap_config_id,
                    ap_config_param: AccessPointConfig::IPv4Address(ip),
                }),
                true,
            )?;
        }
        // Network Subnet mask
        if let Some(subnet) = options.subnet {
            self.send_internal(
                &EdmAtCmdWrapper(SetWifiAPConfig {
                    ap_config_id,
                    ap_config_param: AccessPointConfig::SubnetMask(subnet),
                }),
                true,
            )?;
        }
        // Network Default gateway
        if let Some(gateway) = options.gateway {
            self.send_internal(
                &EdmAtCmdWrapper(SetWifiAPConfig {
                    ap_config_id,
                    ap_config_param: AccessPointConfig::DefaultGateway(gateway),
                }),
                true,
            )?;
        }

        self.send_internal(
            &EdmAtCmdWrapper(SetWifiAPConfig {
                ap_config_id,
                ap_config_param: AccessPointConfig::DHCPServer(OnOff::On),
            }),
            true,
        )?;

        // Active on startup
        // self.send_internal(&SetWifiAPConfig{
        //     ap_config_id,
        //     ap_config_param: AccessPointConfig::ActiveOnStartup(OnOff::On),
        // }, true)?;

        // Wifi part
        // Set the Network SSID to connect to
        self.send_internal(
            &EdmAtCmdWrapper(SetWifiAPConfig {
                ap_config_id,
                ap_config_param: AccessPointConfig::SSID(options.ssid.clone()),
            }),
            true,
        )?;

        if let Some(pass) = options.password.clone() {
            // Use WPA2 as authentication type
            self.send_internal(
                &EdmAtCmdWrapper(SetWifiAPConfig {
                    ap_config_id,
                    ap_config_param: AccessPointConfig::SecurityMode(
                        SecurityMode::Wpa2AesCcmp,
                        SecurityModePSK::PSK,
                    ),
                }),
                true,
            )?;

            // Input passphrase
            self.send_internal(
                &EdmAtCmdWrapper(SetWifiAPConfig {
                    ap_config_id,
                    ap_config_param: AccessPointConfig::PSKPassphrase(PasskeyR::Passphrase(pass)),
                }),
                true,
            )?;
        } else {
            self.send_internal(
                &EdmAtCmdWrapper(SetWifiAPConfig {
                    ap_config_id,
                    ap_config_param: AccessPointConfig::SecurityMode(
                        SecurityMode::Open,
                        SecurityModePSK::Open,
                    ),
                }),
                true,
            )?;
        }

        if let Some(channel) = configuration.channel {
            self.send_internal(
                &EdmAtCmdWrapper(SetWifiAPConfig {
                    ap_config_id,
                    ap_config_param: AccessPointConfig::Channel(channel as u8),
                }),
                true,
            )?;
        }

        self.wifi_connection.replace(WifiConnection::new(
            WifiNetwork {
                bssid: Bytes::new(),
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
            ap_config_id as u8,
        ));
        self.send_internal(
            &EdmAtCmdWrapper(WifiAPAction {
                ap_config_id,
                ap_action: AccessPointAction::Activate,
            }),
            true,
        )?;

        Ok(())
    }

    /// Stop serving a wireless network.
    ///
    /// **NOTE: All users connected will automatically be disconnected.**
    pub fn stop_hotspot(&mut self) -> Result<(), WifiHotspotError> {
        let ap_config_id = AccessPointId::Id0;

        if let Some(ref con) = self.wifi_connection {
            match con.wifi_state {
                WiFiState::Connected | WiFiState::NotConnected => {
                    // con.wifi_state = WiFiState::Inactive;
                    self.send_internal(
                        &EdmAtCmdWrapper(WifiAPAction {
                            ap_config_id,
                            ap_action: AccessPointAction::Deactivate,
                        }),
                        true,
                    )?;
                }
                WiFiState::Inactive => {}
            }
        } else {
            return Err(WifiHotspotError::FailedToStop);
        }

        Ok(())
    }
}
