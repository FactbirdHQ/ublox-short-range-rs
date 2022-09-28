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

use defmt::{debug, trace};

/// Supplicant is used to
///
///
/// ```
/// // Add, activate and remove network
/// let network = ConnectionOptions::new().ssid("my-ssid").password("hunter2");
/// let config_id: u8 = 0;
/// let mut supplicant = ublox.supplicant::<MAX_NETWORKS>().unwrap();
///
/// supplicant.upsert_connection(config_id, network).unwrap();
/// supplicant.activate(config_id).unwrap();
/// while ublox.connected_to_network().is_err() {
///     ublox.spin().ok();
/// }
/// // Now connected to a wifi network.
///
/// let mut supplicant = ublox.supplicant::<MAX_NETWORKS>().unwrap();
/// supplicant.deactivate(0).unwrap();
/// // Connection has to be down before removal
/// while ublox.supplicant::<MAX_NETWORKS>().unwrap().get_active_config().is_some() {
///     ublox.spin().ok();
/// }
/// let mut supplicant = ublox.supplicant::<MAX_NETWORKS>().unwrap();
/// supplicant.remove_connection(0)
///
///
pub struct Supplicant<'a, C, const N: usize> {
    pub(crate) client: &'a mut C,
    pub(crate) wifi_connection: &'a mut Option<WifiConnection>,
    pub(crate) active_on_startup: &'a mut Option<u8>,
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

    // pub fn load(&mut self) {
    //     for config_id in 0..N as u8 {
    //         self.client
    //             .send(&EdmAtCmdWrapper(ExecWifiStationAction {
    //                 config_id,
    //                 action: WifiStationAction::Load,
    //             }))
    //             .ok();
    //     }
    // }
    pub(crate) fn init(&mut self) -> Result<(), Error> {
        debug!("[SUP] init");
        for config_id in 0..N as u8 {
            let load = self.client.send(&EdmAtCmdWrapper(ExecWifiStationAction {
                config_id,
                action: WifiStationAction::Load,
            }));

            let GetWifiStationConfigResponse { parameter, .. } =
                self.send_at(&EdmAtCmdWrapper(GetWifiStationConfig {
                    config_id,
                    parameter: Some(WifiStationConfigParameter::ActiveOnStartup),
                }))?;

            if parameter == WifiStationConfigR::ActiveOnStartup(true.into()) {
                debug!("[SUP] Config {:?} is active on startup", config_id);
                if *self.active_on_startup == None || *self.active_on_startup == Some(config_id) {
                    *self.active_on_startup = Some(config_id);
                    // Update wifi connection
                    if self.wifi_connection.is_none() {
                        let con = self.get_connection(config_id)?.unwrap_or_default();

                        self.wifi_connection.replace(
                            WifiConnection::new(
                                WifiNetwork {
                                    bssid: atat::heapless_bytes::Bytes::new(),
                                    op_mode:
                                        crate::command::wifi::types::OperationMode::Infrastructure,
                                    ssid: con.ssid,
                                    channel: 0,
                                    rssi: 1,
                                    authentication_suites: 0,
                                    unicast_ciphers: 0,
                                    group_ciphers: 0,
                                    mode: super::network::WifiMode::Station,
                                },
                                WiFiState::NotConnected,
                                config_id,
                            )
                            .activate(),
                        );
                    } else if let Some(ref mut con) = self.wifi_connection {
                        if con.config_id == 255 {
                            con.config_id = config_id;
                        }
                    }
                    // One could argue that an excisting connection should be verified,
                    // but should this be the case, the module is already having unexpected behaviour
                } else {
                    // This causes unexpected behaviour
                    defmt::panic!("Two configs are active on startup!")
                }
            } else if load.is_err() {
                //Handle shadow store bug
                //TODO: Check if the ssid is set, if so the credential has to be cleared, as it is not there actually.

                let GetWifiStationConfigResponse { parameter, .. } =
                    self.send_at(&EdmAtCmdWrapper(GetWifiStationConfig {
                        config_id,
                        parameter: Some(WifiStationConfigParameter::SSID),
                    }))?;

                if let WifiStationConfigR::SSID(ssid) = parameter {
                    if !ssid.is_empty() {
                        defmt::error!("Shadow store bug!");
                        // defmt::panic!("Shadow store bug!");
                        // self.client
                        //     .send(&EdmAtCmdWrapper(ExecWifiStationAction {
                        //         config_id,
                        //         action: WifiStationAction::Reset,
                        //     }))
                        //     .ok();
                        self.remove_connection(config_id)
                            .map_err(|_| Error::Supplicant)?;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn get_connection(&mut self, config_id: u8) -> Result<Option<ConnectionOptions>, Error> {
        debug!("[SUP] Get connection: {:?}", config_id);
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

    /// Get id of active config
    pub fn get_active_config_id(&self) -> Option<u8> {
        debug!("[SUP] Get active config id");
        if let Some(ref wifi) = self.wifi_connection {
            if wifi.wifi_state != WiFiState::Inactive {
                debug!("[SUP] Active: {:?}", wifi.config_id);
                return Some(wifi.config_id);
            }
        }
        None
    }

    /// Get id of active config
    pub fn get_active_config_id_quiet(&self) -> Option<u8> {
        // debug!("[SUP] Get active config id");
        if let Some(ref wifi) = self.wifi_connection {
            if wifi.wifi_state != WiFiState::Inactive {
                // debug!("[SUP] Active: {:?}", wifi.config_id);
                return Some(wifi.config_id);
            }
        }
        None
    }

    /// List connections stored in module
    ///
    /// Sorted by config ID
    pub fn list_connections(&mut self) -> Result<Vec<(u8, ConnectionOptions), N>, Error> {
        debug!("[SUP] list connections");
        Ok((0..N as u8)
            .filter_map(|config_id| {
                self.get_connection(config_id)
                    .unwrap()
                    .map(|c| (config_id, c))
            })
            .collect())
    }

    /// Attempts to remove a stored wireless network
    ///
    /// Removing the active connection is not possible. Deactivate the network first.
    pub fn remove_connection(&mut self, config_id: u8) -> Result<(), WifiConnectionError> {
        // self.deactivate(config_id)?;
        debug!("[SUP] Remove connection: {:?}", config_id);
        // check for active
        if self.is_config_in_use(config_id) {
            defmt::error!("Config id is active!");
            return Err(WifiConnectionError::Illigal);
        }

        self.send_at(&EdmAtCmdWrapper(ExecWifiStationAction {
            config_id,
            action: WifiStationAction::Reset,
        }))?;

        self.send_at(&EdmAtCmdWrapper(ExecWifiStationAction {
            config_id,
            action: WifiStationAction::Store,
        }))?;

        if Some(config_id) == self.get_active_on_startup() {
            self.unset_active_on_startup()?;
        }
        // debug!("[SUP] Remove config: {:?}", config_id);

        Ok(())
    }

    /// Attempts to store a wireless network with the given connection options.
    ///
    /// Replacing the currently active network is not possible.
    pub fn insert_connection(
        &mut self,
        config_id: u8,
        options: &ConnectionOptions,
    ) -> Result<(), WifiConnectionError> {
        debug!("[SUP] Insert config: {:?}", config_id);
        // Network part
        // Reset network config slot

        // check for active
        if self.is_config_in_use(config_id) {
            defmt::error!("Config id is active!");
            return Err(WifiConnectionError::Illigal);
        }

        self.send_at(&EdmAtCmdWrapper(ExecWifiStationAction {
            config_id,
            action: WifiStationAction::Reset,
        }))?;
        if Some(config_id) == self.get_active_on_startup() {
            self.unset_active_on_startup()?;
        }

        // Disable DHCP Client (static IP address will be used)
        if options.ip.is_some() || options.subnet.is_some() || options.gateway.is_some() {
            self.send_at(&EdmAtCmdWrapper(SetWifiStationConfig {
                config_id,
                config_param: WifiStationConfig::IPv4Mode(IPv4Mode::Static),
            }))?;
        }

        // Network IP address
        if let Some(ip) = options.ip {
            self.send_at(&EdmAtCmdWrapper(SetWifiStationConfig {
                config_id,
                config_param: WifiStationConfig::IPv4Address(ip),
            }))?;
        }
        // Network Subnet mask
        if let Some(subnet) = options.subnet {
            self.send_at(&EdmAtCmdWrapper(SetWifiStationConfig {
                config_id,
                config_param: WifiStationConfig::SubnetMask(subnet),
            }))?;
        }
        // Network Default gateway
        if let Some(gateway) = options.gateway {
            self.send_at(&EdmAtCmdWrapper(SetWifiStationConfig {
                config_id,
                config_param: WifiStationConfig::DefaultGateway(gateway),
            }))?;
        }

        // Wifi part
        // Set the Network SSID to connect to
        self.send_at(&EdmAtCmdWrapper(SetWifiStationConfig {
            config_id,
            config_param: WifiStationConfig::SSID(options.ssid.clone()),
        }))?;

        if let Some(pass) = options.password.clone() {
            // Use WPA2 as authentication type
            self.send_at(&EdmAtCmdWrapper(SetWifiStationConfig {
                config_id,
                config_param: WifiStationConfig::Authentication(Authentication::WpaWpa2Psk),
            }))?;

            // Input passphrase
            self.send_at(&EdmAtCmdWrapper(SetWifiStationConfig {
                config_id,
                config_param: WifiStationConfig::WpaPskOrPassphrase(pass),
            }))?;
        } else {
            self.send_at(&EdmAtCmdWrapper(SetWifiStationConfig {
                config_id,
                config_param: WifiStationConfig::Authentication(Authentication::Open),
            }))?;
        }

        // Store config
        self.send_at(&EdmAtCmdWrapper(ExecWifiStationAction {
            config_id,
            action: WifiStationAction::Store,
        }))?;

        Ok(())
    }

    /// Activate a given network config
    /// Only one config can be active at any time.
    ///
    /// The driver has two modes of active for ID's. Active in driver and active on module.
    /// These are differentiated by the driver mode being called activated and modules
    /// mode called active. The driver activates a config, and then the driver reacts
    /// asyncronous to the request and sets a config as active.
    /// Driver activation is seen as `wificonnection.active()`.
    /// Module is seen in `wificonnection.state`, where `inactive` is inactive and all others
    /// are activated.
    ///
    /// The activation flow is as follows:
    ///
    /// driver.activate()     driver.deactivate()
    ///       ┼─────────────────────────┼
    ///              ┼─────────────────────────┼
    ///        module is active          module inactive
    pub fn activate(&mut self, config_id: u8) -> Result<(), WifiConnectionError> {
        debug!("[SUP] Activate connection: {:?}", config_id);
        if let Some(w) = self.wifi_connection {
            if w.activated {
                return Err(WifiConnectionError::Illegal);
            }
        }

        let con = self.get_connection(config_id)?.unwrap_or_default();

        self.send_at(&EdmAtCmdWrapper(ExecWifiStationAction {
            config_id,
            action: WifiStationAction::Activate,
        }))?;

        self.wifi_connection.replace(
            WifiConnection::new(
                WifiNetwork {
                    bssid: atat::heapless_bytes::Bytes::new(),
                    op_mode: crate::command::wifi::types::OperationMode::Infrastructure,
                    ssid: con.ssid,
                    channel: 0,
                    rssi: 1,
                    authentication_suites: 0,
                    unicast_ciphers: 0,
                    group_ciphers: 0,
                    mode: super::network::WifiMode::Station,
                },
                WiFiState::Inactive,
                config_id,
            )
            .activate(),
        );
        debug!("[SUP] Activated: {:?}", config_id);

        Ok(())
    }

    /// Deactivates a given network config
    ///
    /// Operation not done until network conneciton is lost
    pub fn deactivate(&mut self, config_id: u8) -> Result<(), WifiConnectionError> {
        debug!("[SUP] Deactivate connection: {:?}", config_id);
        let mut active = false;

        if let Some(con) = self.wifi_connection {
            if con.activated && con.config_id == config_id {
                active = true;
            }
        }

        if active {
            self.send_at(&EdmAtCmdWrapper(ExecWifiStationAction {
                config_id,
                action: WifiStationAction::Deactivate,
            }))?;

            if let Some(ref mut con) = self.wifi_connection {
                con.deactivate();
            }
            debug!("[SUP] Deactivated: {:?}", config_id);
        }
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

    pub fn flush(&mut self) -> Result<(), WifiConnectionError> {
        todo!()
    }

    /// Returns Active on startup config ID if any
    pub fn get_active_on_startup(&self) -> Option<u8> {
        debug!(
            "[SUP] Get active on startup: {:?}",
            self.active_on_startup.clone()
        );
        return self.active_on_startup.clone();
    }

    /// Sets a config as active on startup, replacing the current.
    ///
    /// This is not possible if any of the two are currently active.
    pub fn set_active_on_startup(&mut self, config_id: u8) -> Result<(), WifiConnectionError> {
        debug!("[SUP] Set active on startup connection: {:?}", config_id);
        // check end condition true
        if let Some(active_on_startup) = *self.active_on_startup {
            if active_on_startup == config_id {
                return Ok(());
            }
            // check for active connection
            if self.is_config_in_use(active_on_startup) {
                defmt::error!("Active on startup is active!");
                return Err(WifiConnectionError::Illigal);
            }
        }
        if self.is_config_in_use(config_id) {
            defmt::error!("Config id is active!");
            return Err(WifiConnectionError::Illigal);
        }

        // disable current active on startup
        if let Some(active_on_startup) = *self.active_on_startup {
            // if any active on startup remove this parameter.
            self.send_at(&EdmAtCmdWrapper(SetWifiStationConfig {
                config_id: active_on_startup,
                config_param: WifiStationConfig::ActiveOnStartup(false.into()),
            }))?;

            self.send_at(&EdmAtCmdWrapper(ExecWifiStationAction {
                config_id: active_on_startup,
                action: WifiStationAction::Store,
            }))?;
        }

        // Insert the new one as active on startup.
        self.send_at(&EdmAtCmdWrapper(SetWifiStationConfig {
            config_id,
            config_param: WifiStationConfig::ActiveOnStartup(true.into()),
        }))?;

        self.send_at(&EdmAtCmdWrapper(ExecWifiStationAction {
            config_id,
            action: WifiStationAction::Store,
        }))?;

        *self.active_on_startup = Some(config_id);

        Ok(())
    }

    /// Unsets a config as active on startup, replacing the current.
    ///
    /// This is not possible if any of the two are currently active.
    pub fn unset_active_on_startup(&mut self) -> Result<(), WifiConnectionError> {
        debug!("[SUP] Unset active on startup connection");
        // check for any of them as active
        if let Some(active_on_startup) = self.active_on_startup.clone() {
            // check for active connection
            if self.is_config_in_use(active_on_startup) {
                defmt::error!("Active on startup is active!");
                return Err(WifiConnectionError::Illigal);
            }
            // if any active remove this asset.
            self.send_at(&EdmAtCmdWrapper(SetWifiStationConfig {
                config_id: active_on_startup,
                config_param: WifiStationConfig::ActiveOnStartup(false.into()),
            }))?;

            self.send_at(&EdmAtCmdWrapper(ExecWifiStationAction {
                config_id: active_on_startup,
                action: WifiStationAction::Store,
            }))?;
            *self.active_on_startup = None;
        }
        Ok(())
    }

    /// Checks for active and activated.
    /// See self.activate for explanation.
    fn is_config_in_use(&self, config_id: u8) -> bool {
        if let Some(active_id) = self.get_active_config_id() {
            if active_id == config_id {
                return true;
            }
        } else if let Some(ref con) = self.wifi_connection {
            if con.activated && con.config_id == config_id {
                defmt::error!("One of the IDs being changed is activated!");
                return true;
            }
        }
        false
    }
}
