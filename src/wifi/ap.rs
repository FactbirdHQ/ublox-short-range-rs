use crate::{
    blocking::UbloxClient,
    command::{
        edm::EdmAtCmdWrapper,
        wifi::{
            self,
            types::{
                AccessPointAction, AccessPointConfig, AccessPointId, IPv4Mode, PasskeyR,
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
use atat::blocking::AtatClient;
use atat::heapless_bytes::Bytes;
use embedded_hal::digital::OutputPin;

use super::connection::{WiFiState, WifiConnection};

impl<C, RST, const N: usize, const L: usize> UbloxClient<C, RST, N, L>
where
    C: AtatClient,
    RST: OutputPin,
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
            if con.activated {
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
                ap_config_param: AccessPointConfig::DHCPServer(true.into()),
            }),
            true,
        )?;

        // Active on startup
        // self.send_internal(&SetWifiAPConfig{
        //     ap_config_id,
        //     ap_config_param: AccessPointConfig::ActiveOnStartup(true),
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

        self.send_internal(
            &EdmAtCmdWrapper(WifiAPAction {
                ap_config_id,
                ap_action: AccessPointAction::Activate,
            }),
            true,
        )?;

        self.wifi_connection.replace(
            WifiConnection::new(
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
            )
            .activate(),
        );
        Ok(())
    }

    /// Stop serving a wireless network.
    ///
    /// **NOTE: All users connected will automatically be disconnected.**
    pub fn stop_hotspot(&mut self) -> Result<(), WifiHotspotError> {
        let ap_config_id = AccessPointId::Id0;

        if let Some(ref con) = self.wifi_connection {
            if con.activated {
                self.send_internal(
                    &EdmAtCmdWrapper(WifiAPAction {
                        ap_config_id,
                        ap_action: AccessPointAction::Deactivate,
                    }),
                    true,
                )?;
            }
        } else {
            return Err(WifiHotspotError::FailedToStop);
        }
        if let Some(ref mut con) = self.wifi_connection {
            con.deactivate()
        }

        Ok(())
    }
}
