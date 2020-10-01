use atat::AtatClient;
use crate::{
    client::{UbloxClient, State},
    command::{*, 
        wifi::{types::*, *, responses::*}},
    error::{WifiConnectionError, WifiError},
    prelude::*,
    // wait_for_unsolicited,
    wifi::{
        connection::{WifiConnection, WiFiState},
        network::{WifiMode, WifiNetwork},
        options::ConnectionOptions,
    },
};

// use core::convert::TryFrom;
use embedded_hal::timer::{Cancel, CountDown};
use heapless::{Vec, String, consts, ArrayLength};
use core::convert::TryFrom;

#[cfg(feature = "logging")]
use log::info;

impl<T, N, L> WifiConnectivity for UbloxClient<T, N, L>
where
    T: AtatClient,
    N: ArrayLength<Option<crate::sockets::SocketSetItem<L>>>,
    L: ArrayLength<u8>,
{
    /// Attempts to connect to a wireless network with the given options.
    fn connect(
        mut self,
        options: ConnectionOptions,
    ) -> Result<(), WifiConnectionError> {
        let mut config_id: u8 = 0;
        if let Some(config_id_option) = options.config_id{
            config_id =  config_id_option;
        }

        // Network part
        // Deactivate network id 0
        self.send_internal(&ExecWifiStationAction{
            config_id: config_id,
            action: WifiStationAction::Deactivate,
        }, true)?;

        // Disable DHCP Client (static IP address will be used)
        if options.ip.is_some() || options.subnet.is_some() || options.gateway.is_some() {
            self.send_internal(&SetWifiStationConfig{
                config_id: config_id,
                config_param: WifiStationConfig::IPv4Mode(IPv4Mode::Static)
            }, true)?;
        }

        // Network IP address
        if let Some(ip) = options.ip {
            self.send_internal(&SetWifiStationConfig{
                config_id: config_id,
                config_param: WifiStationConfig::IPv4Address(ip),
            }, true)?;
        }
        // Network Subnet mask
        if let Some(subnet) = options.subnet{
            self.send_internal(&SetWifiStationConfig{
                config_id: config_id,
                config_param: WifiStationConfig::SubnetMask(subnet),
            }, true)?;
        }
        // Network Default gateway
        if let Some(gateway) = options.gateway{
            self.send_internal(&SetWifiStationConfig{
                config_id: config_id,
                config_param: WifiStationConfig::DefaultGateway(gateway),
            }, true)?;
        }

        // Active on startup
        // self.send_internal(&SetWifiStationConfig{
        //     config_id: config_id,
        //     config_param: WifiStationConfig::ActiveOnStartup(OnOff::On),
        // }, true)?;

        // Wifi part
        // Set the Network SSID to connect to
        self.send_internal(&SetWifiStationConfig{
            config_id: config_id,
            config_param: WifiStationConfig::SSID(&options.ssid),
        }, true)?;

        if let Some(pass) = options.password{
            // Use WPA2 as authentication type
            self.send_internal(&SetWifiStationConfig{
                config_id: config_id,
                config_param: WifiStationConfig::Authentication(Authentication::WPA_WAP2_PSK)
            }, true)?;

            // Input passphrase
            self.send_internal(&SetWifiStationConfig{
                config_id: config_id,
                config_param: WifiStationConfig::WPA_PSKOrPassphrase(&pass),
            }, true)?;
        }

        *self.wifi_connection.try_borrow_mut()? = Some(
            WifiConnection::new(
                WifiNetwork {
                    bssid: String::new(),
                    op_mode: wifi::types::OperationMode::AdHoc,
                    ssid: options.ssid,
                    channel: 0,
                    rssi: 1,
                    authentication_suites: 0,
                    unicast_ciphers: 0,
                    group_ciphers: 0,
                    mode: WifiMode::AccessPoint
                },
                WiFiState::Connecting,
                config_id,
            )
        );
        self.send_internal(&ExecWifiStationAction{
            config_id: config_id,
            action: WifiStationAction::Activate,
        }, true)?;

        // TODO: Await connected event?
        // block!(wait_for_unsolicited!(self, UnsolicitedResponse::NetworkUp { .. })).unwrap();

        Ok(())
    }

    fn scan(&mut self) -> Result<Vec<WifiNetwork, consts::U32>, WifiError> {
        match self.send_internal(&WifiScan{
            ssid: None,
        }, true){
            Ok(resp) => resp.network_list
                .into_iter()
                .map(WifiNetwork::try_from)
                .collect(),
            Err(_) => Err(WifiError::UnexpectedResponse),
        }
    }

    fn disconnect(&mut self) -> Result<(), WifiConnectionError> {
        if let Some (ref mut con) = *self.wifi_connection.try_borrow_mut()? {
            match con.wifi_state {
                WiFiState::Connected | WiFiState::Connecting => {
                    con.wifi_state = WiFiState::Disconnecting;
                    self.send_internal(&ExecWifiStationAction{
                        config_id: 0,
                        action: WifiStationAction::Deactivate,
                    }, true)?;
                }
                _ => {}
            }
        } else {
            return Err(WifiConnectionError::FailedToDisconnect);
        }
        Ok(())
    }
}

// #[cfg(test)]
// mod tests {
//     setup_test_env!();

//     #[test]
//     fn test_connect() {
//         let (ublox, (mut wifi_req_c, mut wifi_res_p)) = setup_test_case!();

//         // Load the response queue with expected responses
//         wifi_res_p.enqueue(Ok(ResponseType::None)).unwrap();
//         wifi_res_p.enqueue(Ok(ResponseType::None)).unwrap();
//         wifi_res_p.enqueue(Ok(ResponseType::None)).unwrap();
//         wifi_res_p
//             .enqueue(Ok(ResponseType::SingleSolicited(Response::STASetConfig {
//                 configuration_id: 0,
//                 param_tag: UWSCSetTag::ActiveOnStartup(true),
//             })))
//             .unwrap();
//         wifi_res_p
//             .enqueue(Ok(ResponseType::SingleSolicited(Response::STASetConfig {
//                 configuration_id: 0,
//                 param_tag: UWSCSetTag::SSID(String::from("WifiSSID")),
//             })))
//             .unwrap();
//         wifi_res_p
//             .enqueue(Ok(ResponseType::SingleSolicited(Response::STASetConfig {
//                 configuration_id: 0,
//                 param_tag: UWSCSetTag::Authentication(AuthentificationType::WpaWpa2),
//             })))
//             .unwrap();
//         wifi_res_p
//             .enqueue(Ok(ResponseType::SingleSolicited(Response::STASetConfig {
//                 configuration_id: 0,
//                 param_tag: UWSCSetTag::Passphrase(String::from("passphrase123098")),
//             })))
//             .unwrap();
//         wifi_res_p.enqueue(Ok(ResponseType::None)).unwrap();
//         // wifi_res_p.enqueue(Ok(ResponseType::None)).unwrap();

//         let options = wifi::options::ConnectionOptions::new()
//             .ssid(String::from("WifiSSID"))
//             .password(String::from("passphrase123098"));

//         // Attempt to connect to a wifi
//         let connection = ublox.connect(options);

//         assert!(connection.is_ok());

//         // assertions
//         // assert_eq!(
//         //     wifi_req_c.dequeue().unwrap(),
//         //     Command::SetRS232Settings {
//         //         baud_rate: BaudRate::Baud115200,
//         //         flow_control: FlowControl::NotUsed,
//         //         data_bits: 8,
//         //         stop_bits: StopBits::StopBits1,
//         //         parity: Parity::NoParity,
//         //         change_after_confirm: ChangeAfterConfirm::NoChange,
//         //     }
//         // );

//         // assert_eq!(wifi_req_c.dequeue().unwrap(), Command::Store);

//         assert_eq!(
//             wifi_req_c.dequeue().unwrap().try_get_cmd().unwrap(),
//             Command::ExecSTAAction {
//                 configuration_id: 0,
//                 action: STAAction::Deactivate,
//             }
//         );

//         assert_eq!(
//             wifi_req_c.dequeue().unwrap().try_get_cmd().unwrap(),
//             Command::STASetConfig {
//                 configuration_id: 0,
//                 param_tag: UWSCSetTag::ActiveOnStartup(true),
//             }
//         );
//         assert_eq!(
//             wifi_req_c.dequeue().unwrap().try_get_cmd().unwrap(),
//             Command::STASetConfig {
//                 configuration_id: 0,
//                 param_tag: UWSCSetTag::SSID(String::from("WifiSSID")),
//             }
//         );
//         assert_eq!(
//             wifi_req_c.dequeue().unwrap().try_get_cmd().unwrap(),
//             Command::STASetConfig {
//                 configuration_id: 0,
//                 param_tag: UWSCSetTag::Authentication(AuthentificationType::WpaWpa2),
//             }
//         );
//         assert_eq!(
//             wifi_req_c.dequeue().unwrap().try_get_cmd().unwrap(),
//             Command::STASetConfig {
//                 configuration_id: 0,
//                 param_tag: UWSCSetTag::Passphrase(String::from("passphrase123098")),
//             }
//         );

//         assert_eq!(
//             wifi_req_c.dequeue().unwrap().try_get_cmd().unwrap(),
//             Command::ExecSTAAction {
//                 configuration_id: 0,
//                 action: STAAction::Activate,
//             }
//         );

//         // assert_eq!(
//         //     wifi_req_c.dequeue().unwrap().get_cmd().unwrap(),
//         //     Command::ExecSTAAction {
//         //         configuration_id: 0,
//         //         action: STAAction::Store,
//         //     }
//         // );

//         cleanup_test_case!(connection, wifi_req_c);
//     }
// }
