use crate::{
    command::*,
    error::{WifiConnectionError, WifiError},
    prelude::*,
    wifi::{
        connection::WifiConnection,
        network::{WifiMode, WifiNetwork},
        options::ConnectionOptions,
    },
    ATClient
};

use at::ATInterface;

use core::convert::TryFrom;
use embedded_hal::timer::CountDown;
use heapless::{String, Vec};

impl<T> WifiConnectivity<T> for ATClient<T>
where
    T: CountDown,
{
    /// Attempts to connect to a wireless network with the given options.
    fn connect(
        mut self,
        options: ConnectionOptions,
    ) -> Result<WifiConnection<T>, WifiConnectionError> {
        // if let Err(_e) = self.turn_on() {
        //     return Err(WifiConnectionError::Other {
        //         kind: WifiError::WifiDisabled,
        //     });
        // }

        self.send(Command::SetRS232Settings {
            baud_rate: BaudRate::Baud115200,
            flow_control: FlowControl::NotUsed,
            data_bits: 8,
            stop_bits: StopBits::StopBits1,
            parity: Parity::NoParity,
            change_after_confirm: ChangeAfterConfirm::NoChange,
        })?;

        self.send(Command::Store)?;
        // self.send(Command::PwrOff)?;

        // Network part
        // Deactivate network id 0
        self.send(Command::ExecSTAAction {
            configuration_id: 0,
            action: STAAction::Deactivate,
        })?;

        // Disable DHCP Client (static IP address will be used)
        if options.ip.is_some() || options.subnet.is_some() || options.gateway.is_some() {
            self.send(Command::STASetConfig {
                configuration_id: 0,
                param_tag: UWSCSetTag::Ipv4Mode(Ipv4Mode::Static),
            })?;
        }

        // Network IP address
        if let Some(ip) = options.ip {
            self.send(Command::STASetConfig {
                configuration_id: 0,
                param_tag: UWSCSetTag::Ipv4Address(ip),
            })?;
        }
        // Network Subnet mask
        if let Some(subnet) = options.subnet {
            self.send(Command::STASetConfig {
                configuration_id: 0,
                param_tag: UWSCSetTag::SubnetMask(subnet),
            })?;
        }
        // Network Default gateway
        if let Some(gateway) = options.gateway {
            self.send(Command::STASetConfig {
                configuration_id: 0,
                param_tag: UWSCSetTag::DefaultGateway(gateway),
            })?;
        }

        // Active on startup
        self.send(Command::STASetConfig {
            configuration_id: 0,
            param_tag: UWSCSetTag::ActiveOnStartup(true),
        })?;

        // Wifi part
        // Set the Network SSID to connect to
        self.send(Command::STASetConfig {
            configuration_id: 0,
            param_tag: UWSCSetTag::SSID(options.ssid.clone()),
        })?;

        if let Some(pass) = options.password {
            // Use WPA2 as authentication type
            self.send(Command::STASetConfig {
                configuration_id: 0,
                param_tag: UWSCSetTag::Authentication(AuthentificationType::WpaWpa2),
            })?;

            // Recommended to use a secured WPA2 password
            self.send(Command::STASetConfig {
                configuration_id: 0,
                param_tag: UWSCSetTag::Passphrase(pass),
            })?;
        }

        self.send(Command::ExecSTAAction {
            configuration_id: 0,
            action: STAAction::Activate,
        })?;

        // Store Wi-Fi configuration.
        self.send(Command::ExecSTAAction {
            configuration_id: 0,
            action: STAAction::Store,
        })?;

        // Await connected event?

        // Read ip, mac info
        let network = WifiNetwork {
            bssid: String::new(),
            op_mode: OPMode::AdHoc,
            ssid: options.ssid,
            channel: 5,
            rssi: 1,
            authentication_suites: Vec::new(),
            unicast_ciphers: Vec::new(),
            group_ciphers: Vec::new(),
            mode: WifiMode::Station,
        };

        Ok(WifiConnection::new(self, network))
    }

    fn scan(&mut self) -> Result<Vec<WifiNetwork, at::MaxResponseLines>, WifiError> {
        match self.send(Command::STAScan { ssid: None })? {
            ResponseType::MultiSolicited(responses) => responses
                .iter()
                .map(|r| WifiNetwork::try_from(r.clone()))
                .collect(),
            _ => Err(WifiError::UnexpectedResponse),
        }
    }
}

#[cfg(test)]
mod tests {
    setup_test_env!();

    #[test]
    fn test_connect() {
        let (wifi_client, (mut wifi_cmd_c, mut wifi_resp_p)) = setup_test_case!();

        // Load the response queue with expected responses
        wifi_resp_p.enqueue(Ok(ResponseType::None)).unwrap();
        wifi_resp_p.enqueue(Ok(ResponseType::None)).unwrap();
        wifi_resp_p.enqueue(Ok(ResponseType::None)).unwrap();
        wifi_resp_p
            .enqueue(Ok(ResponseType::SingleSolicited(Response::STASetConfig {
                configuration_id: 0,
                param_tag: UWSCSetTag::ActiveOnStartup(true),
            })))
            .unwrap();
        wifi_resp_p
            .enqueue(Ok(ResponseType::SingleSolicited(Response::STASetConfig {
                configuration_id: 0,
                param_tag: UWSCSetTag::SSID(String::from("WifiSSID")),
            })))
            .unwrap();
        wifi_resp_p
            .enqueue(Ok(ResponseType::SingleSolicited(Response::STASetConfig {
                configuration_id: 0,
                param_tag: UWSCSetTag::Authentication(AuthentificationType::WpaWpa2),
            })))
            .unwrap();
        wifi_resp_p
            .enqueue(Ok(ResponseType::SingleSolicited(Response::STASetConfig {
                configuration_id: 0,
                param_tag: UWSCSetTag::Passphrase(String::from("passphrase123098")),
            })))
            .unwrap();
        wifi_resp_p.enqueue(Ok(ResponseType::None)).unwrap();
        wifi_resp_p.enqueue(Ok(ResponseType::None)).unwrap();

        let options = wifi::options::ConnectionOptions::new()
            .ssid(String::from("WifiSSID"))
            .password(String::from("passphrase123098"));

        // Attempt to connect to a wifi
        let connection = wifi_client.connect(options);

        assert!(connection.is_ok());

        // assertions
        assert_eq!(
            wifi_cmd_c.dequeue().unwrap(),
            Command::SetRS232Settings {
                baud_rate: BaudRate::Baud115200,
                flow_control: FlowControl::NotUsed,
                data_bits: 8,
                stop_bits: StopBits::StopBits1,
                parity: Parity::NoParity,
                change_after_confirm: ChangeAfterConfirm::NoChange,
            }
        );

        assert_eq!(wifi_cmd_c.dequeue().unwrap(), Command::Store);

        assert_eq!(
            wifi_cmd_c.dequeue().unwrap(),
            Command::ExecSTAAction {
                configuration_id: 0,
                action: STAAction::Deactivate,
            }
        );

        assert_eq!(
            wifi_cmd_c.dequeue().unwrap(),
            Command::STASetConfig {
                configuration_id: 0,
                param_tag: UWSCSetTag::ActiveOnStartup(true),
            }
        );
        assert_eq!(
            wifi_cmd_c.dequeue().unwrap(),
            Command::STASetConfig {
                configuration_id: 0,
                param_tag: UWSCSetTag::SSID(String::from("WifiSSID")),
            }
        );
        assert_eq!(
            wifi_cmd_c.dequeue().unwrap(),
            Command::STASetConfig {
                configuration_id: 0,
                param_tag: UWSCSetTag::Authentication(AuthentificationType::WpaWpa2),
            }
        );
        assert_eq!(
            wifi_cmd_c.dequeue().unwrap(),
            Command::STASetConfig {
                configuration_id: 0,
                param_tag: UWSCSetTag::Passphrase(String::from("passphrase123098")),
            }
        );

        assert_eq!(
            wifi_cmd_c.dequeue().unwrap(),
            Command::ExecSTAAction {
                configuration_id: 0,
                action: STAAction::Activate,
            }
        );

        assert_eq!(
            wifi_cmd_c.dequeue().unwrap(),
            Command::ExecSTAAction {
                configuration_id: 0,
                action: STAAction::Store,
            }
        );

        cleanup_test_case!(connection, wifi_cmd_c);
    }
}
