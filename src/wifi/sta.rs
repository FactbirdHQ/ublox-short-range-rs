use atat::AtatClient;
use crate::{
    client::UbloxClient,
    command::*,
    error::{WifiConnectionError, WifiError},
    prelude::*,
    wait_for_unsolicited,
    wifi::{
        connection::WifiConnection,
        network::{WifiMode, WifiNetwork},
        options::ConnectionOptions,
    },
};

// use core::convert::TryFrom;
use embedded_hal::timer::{Cancel, CountDown};
use heapless::{Vec, String, consts};
use log::info;

impl<T> WifiConnectivity<T> for UbloxClient<T>
where
    T: AtatClient,
{
    /// Attempts to connect to a wireless network with the given options.
    fn connect(
        mut self,
        options: ConnectionOptions,
    ) -> Result<WifiConnection<T>, WifiConnectionError> {
        // // Network part
        // // Deactivate network id 0
        // self.send_at(Command::ExecSTAAction {
        //     configuration_id: 0,
        //     action: STAAction::Deactivate,
        // })?;

        // // Disable DHCP Client (static IP address will be used)
        // if options.ip.is_some() || options.subnet.is_some() || options.gateway.is_some() {
        //     self.send_at(Command::STASetConfig {
        //         configuration_id: 0,
        //         param_tag: UWSCSetTag::Ipv4Mode(Ipv4Mode::Static),
        //     })?;
        // }

        // // Network IP address
        // if let Some(ip) = options.ip {
        //     self.send_at(Command::STASetConfig {
        //         configuration_id: 0,
        //         param_tag: UWSCSetTag::Ipv4Address(ip),
        //     })?;
        // }
        // // Network Subnet mask
        // if let Some(subnet) = options.subnet {
        //     self.send_at(Command::STASetConfig {
        //         configuration_id: 0,
        //         param_tag: UWSCSetTag::SubnetMask(subnet),
        //     })?;
        // }
        // // Network Default gateway
        // if let Some(gateway) = options.gateway {
        //     self.send_at(Command::STASetConfig {
        //         configuration_id: 0,
        //         param_tag: UWSCSetTag::DefaultGateway(gateway),
        //     })?;
        // }

        // // Active on startup
        // self.send_at(Command::STASetConfig {
        //     configuration_id: 0,
        //     param_tag: UWSCSetTag::ActiveOnStartup(true),
        // })?;

        // // Wifi part
        // // Set the Network SSID to connect to
        // self.send_at(Command::STASetConfig {
        //     configuration_id: 0,
        //     param_tag: UWSCSetTag::SSID(options.ssid.clone()),
        // })?;

        // if let Some(pass) = options.password {
        //     // Use WPA2 as authentication type
        //     self.send_at(Command::STASetConfig {
        //         configuration_id: 0,
        //         param_tag: UWSCSetTag::Authentication(AuthentificationType::WpaWpa2),
        //     })?;

        //     // Recommended to use a secured WPA2 password
        //     self.send_at(Command::STASetConfig {
        //         configuration_id: 0,
        //         param_tag: UWSCSetTag::Passphrase(pass),
        //     })?;
        // }

        // self.send_at(Command::ExecSTAAction {
        //     configuration_id: 0,
        //     action: STAAction::Activate,
        // })?;

        // // Await connected event?
        // let (bssid, channel) =
        //     if let UnsolicitedResponse::WifiLinkConnected { bssid, channel, .. } =
        //         block!(wait_for_unsolicited!(self, UnsolicitedResponse::WifiLinkConnected { .. }))
        //             .unwrap()
        //     {
        //         (bssid, channel)
        //     } else {
        //         unreachable!()
        //     };

        // block!(wait_for_unsolicited!(self, UnsolicitedResponse::NetworkUp { .. })).unwrap();

        // // Read ip, mac info
        // let network = WifiNetwork {
        //     bssid,
        //     op_mode: wifi::types::OperationMode::AdHoc,
        //     ssid: options.ssid,
        //     channel,
        //     rssi: 1,
        //     authentication_suites: 0,
        //     unicast_ciphers: 0,
        //     group_ciphers: 0,
        //     mode: WifiMode::Station,
        // };

        // Ok(WifiConnection::new(self, network))
        Ok(WifiConnection::new(self, WifiNetwork {
            bssid: String::new(),
            op_mode: wifi::types::OperationMode::AdHoc,
            ssid: String::new(),
            channel: 0,
            rssi: 1,
            authentication_suites: 0,
            unicast_ciphers: 0,
            group_ciphers: 0,
            mode: WifiMode::AccessPoint}))
    }

    fn scan(&mut self) -> Result<Vec<WifiNetwork, consts::U32>, WifiError> {
        // match self.send_at(Command::STAScan { ssid: None })? {
        //     // ResponseType::MultiSolicited(responses) => responses
        //     //     .iter()
        //     //     .cloned()
        //     //     .map(WifiNetwork::try_from)
        //     //     .collect(),
        //     _ => Err(WifiError::UnexpectedResponse),
        // }
        Ok(Vec::new())
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
