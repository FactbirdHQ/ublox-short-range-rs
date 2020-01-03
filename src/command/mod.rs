//! AT Commands for ODIN-W2 module\
//! Following https://www.u-blox.com/sites/default/files/u-connect-ATCommands-Manual_(UBX-14044127).pdf

use core::fmt::Write;
use heapless::{String, Vec};

use at::ATCommandInterface;

mod response;
mod cmd;
mod types;

pub use response::{Response, UnsolicitedResponse};
pub use cmd::Command;
pub use types::*;

#[derive(Debug, Clone)]
pub enum ResponseType {
    SingleSolicited(Response),
    MultiSolicited(Vec<Response, heapless::consts::U4>),
    Unsolicited(UnsolicitedResponse),
    None,
}


impl ATCommandInterface<ResponseType> for Command {
    fn get_cmd(&self) -> String<at::MaxCommandLen> {
        let mut buffer: String<at::MaxCommandLen> = String::new();
        match self {
            Command::AT => String::from("AT"),
            Command::GetManufacturerId => String::from("AT+CGMI"),
            Command::GetModelId => String::from("AT+CGMM"),
            Command::GetFWVersion => String::from("AT+CGMR"),
            Command::GetSerialNum => String::from("AT+CGSN"),
            Command::GetId => String::from("ATI9"),
            Command::SetGreetingText {
                ref enable,
                ref text,
            } => {
                if *enable {
                    if text.len() > 49 {
                        // TODO: Error!
                    }
                    write!(buffer, "AT+CSGT={},{}", *enable as u8, text).unwrap();
                } else {
                    write!(buffer, "AT+CSGT={}", *enable as u8).unwrap();
                }
                buffer
            }
            Command::GetGreetingText => String::from("AT+CSGT?"),
            Command::Store => String::from("AT&W"),
            Command::ResetDefault => String::from("ATZ0"),
            Command::ResetFactory => String::from("AT+UFACTORY"),
            Command::SetDTR { ref value } => {
                write!(buffer, "AT&D{}", *value as u8).unwrap();
                buffer
            }
            Command::SetDSR { ref value } => {
                write!(buffer, "AT&S{}", *value as u8).unwrap();
                buffer
            }
            Command::SetEcho { ref enable } => {
                write!(buffer, "ATE{}", *enable as u8).unwrap();
                buffer
            }
            Command::GetEcho => String::from("ATE?"),
            Command::SetEscape { ref esc_char } => {
                write!(buffer, "ATS2={}", esc_char).unwrap();
                buffer
            }
            Command::GetEscape => String::from("ATS2?"),
            Command::SetTermination { ref line_term } => {
                write!(buffer, "ATS3={}", line_term).unwrap();
                buffer
            }
            Command::GetTermination => String::from("ATS3?"),
            Command::SetFormatting { ref term } => {
                write!(buffer, "ATS4={}", term).unwrap();
                buffer
            }
            Command::GetFormatting => String::from("ATS4?"),
            Command::SetBackspace { ref backspace } => {
                write!(buffer, "ATS5={}", backspace).unwrap();
                buffer
            }
            Command::GetBackspace => String::from("ATS5?"),
            Command::FWUpdate {
                ref filetype,
                ref baud_rate,
            } => {
                write!(buffer, "AT+UFWUPD={},{}", filetype, *baud_rate as u16).unwrap();
                buffer
            }
            Command::PwrOff => String::from("AT+CPWROFF"),
            Command::SetStartMode { ref start_mode } => {
                write!(buffer, "AT+UMSM={}", *start_mode as u16).unwrap();
                buffer
            }
            Command::GetStartMode => String::from("AT+UMSM?"),
            Command::GetLocalAddr { ref interface_id } => {
                write!(buffer, "AT+UMLA={}", *interface_id as u8).unwrap();
                buffer
            }
            Command::GetSystemStatus => String::from("AT+UMSTAT"),
            Command::GetRS232Settings => String::from("AT+UMRS?"),
            Command::SetRS232Settings {
                ref baud_rate,
                ref flow_control,
                ref data_bits,
                ref stop_bits,
                ref parity,
                ref change_after_confirm,
            } => {
                write!(
                    buffer,
                    "AT+UMRS={},{},{},{},{},{}",
                    *baud_rate as u32,
                    *flow_control as u8,
                    data_bits,
                    *stop_bits as u8,
                    *parity as u8,
                    *change_after_confirm as u8
                )
                .unwrap();
                buffer
            }
            Command::SetMode { ref mode } => {
                write!(buffer, "ATO{}", *mode as u8).unwrap();
                buffer
            }
            Command::ConnectPeer { ref url } => {
                write!(buffer, "AT+UDCP={}", url).unwrap();
                buffer
            }
            Command::ClosePeerConnection { ref peer_handle } => {
                write!(buffer, "AT+UDCPC={}", peer_handle).unwrap();
                buffer
            }
            Command::GetDefaultPeer { ref peer_id } => {
                write!(buffer, "AT+UDDRP={}", peer_id).unwrap();
                buffer
            }
            Command::SetDefaultPeer {
                ref peer_id,
                ref url,
                ref connect_scheme,
            } => {
                write!(
                    buffer,
                    "AT+UDDRP={},{},{}",
                    peer_id, url, *connect_scheme as u8
                )
                .unwrap();
                buffer
            }
            // Command::SetServerCfg(u8, u8) => "",
            // Command::GetServerCfg(u8) => "",
            Command::GetWatchdogSettings { ref wd_type } => {
                if *wd_type == WatchDogType::All {
                    write!(buffer, "AT+UDWS").unwrap();
                } else {
                    write!(buffer, "AT+UDWS={}", *wd_type as u8).unwrap();
                }
                buffer
            }

            Command::SetWatchdogSettings {
                ref wd_type,
                ref timeout,
            } => {
                write!(buffer, "AT+UDWS={},{}", *wd_type as u8, timeout).unwrap();
                buffer
            }
            Command::GetPeerConfig { ref param } => {
                if *param == PeerConfigGet::All {
                    write!(buffer, "AT+UDCFG").unwrap();
                } else {
                    write!(buffer, "AT+UDCFG={}", *param as u8).unwrap();
                }
                buffer
            }
            Command::SetPeerConfig { ref param } => match param {
                PeerConfigSet::KeepPeerInCmdMode(ref keep_connections) => {
                    write!(buffer, "AT+UDCFG=0,{}", *keep_connections as u8).unwrap();
                    buffer
                }
            },

            Command::GetDiscoverable => String::from("AT+UBTDM?"),
            Command::SetDiscoverable {
                ref discoverability_mode,
            } => {
                write!(buffer, "AT+UBTDM={}", *discoverability_mode as u8).unwrap();
                buffer
            }
            Command::GetConnectability => String::from("AT+UBTCM?"),
            Command::SetConnectability {
                ref connectability_mode,
            } => {
                write!(buffer, "AT+UBTCM={}", *connectability_mode as u8).unwrap();
                buffer
            }
            Command::GetParingMode => String::from("AT+UBTPM?"),
            Command::SetParingMode { ref pairing_mode } => {
                write!(buffer, "AT+UBTPM={}", *pairing_mode as u8).unwrap();
                buffer
            }
            Command::GetSecurityMode => String::from("AT+UBTSM?"),
            Command::SetSecurityMode {
                ref security_mode,
                ref security_mode_bt2_0,
                fixed_pin,
            } => {
                write!(
                    buffer,
                    "AT+UBTSM={},{},{}",
                    *security_mode as u8, *security_mode_bt2_0 as u8, fixed_pin
                )
                .unwrap();
                buffer
            }
            Command::UserConfirmation {
                bd_addr,
                ref yes_no,
            } => {
                write!(buffer, "AT+UBTUC={},{}", bd_addr, *yes_no as u8).unwrap();
                buffer
            }
            Command::UserPasskey {
                bd_addr,
                ref ok_cancel,
                ref passkey,
            } => {
                if *ok_cancel {
                    write!(
                        buffer,
                        "AT+UBTUPE={},{},{}",
                        bd_addr, *ok_cancel as u8, passkey
                    )
                    .unwrap();
                } else {
                    write!(buffer, "AT+UBTUPE={},{}", bd_addr, *ok_cancel as u8).unwrap();
                }
                buffer
            }

            Command::NameDiscovery {
                device_name,
                ref mode,
                ref timeout,
            } => {
                if *mode == BTMode::ClassicAndLowEnergy {
                    // Error! Not supported here!
                }
                write!(
                    buffer,
                    "AT+UBTND={},{},{}",
                    device_name, *mode as u8, timeout
                )
                .unwrap();
                buffer
            }
            Command::Inquiry {
                ref inquiry_type,
                ref inquiry_length,
            } => {
                write!(buffer, "AT+UBTI={},{}", *inquiry_type as u8, inquiry_length).unwrap();
                buffer
            }
            Command::Discovery {
                ref discovery_type,
                ref mode,
                ref inquiry_length,
            } => {
                write!(
                    buffer,
                    "AT+UBTD={},{},{}",
                    *discovery_type as u8, *mode as u8, inquiry_length
                )
                .unwrap();
                buffer
            }
            Command::Bond { bd_addr, ref mode } => {
                if *mode == BTMode::ClassicAndLowEnergy {
                    // Error! Not supported here!
                }
                write!(buffer, "AT+UBTB={},{}", bd_addr, *mode as u8).unwrap();
                buffer
            }
            Command::UnBond { ref bd_addr } => {
                write!(buffer, "AT+UBTUB={}", bd_addr).unwrap();
                buffer
            }
            Command::GetBonds { ref mode } => {
                write!(buffer, "AT+UBTBD={}", *mode as u8).unwrap();
                buffer
            }
            Command::GetLocalName => String::from("AT+UBTLN?"),
            Command::SetLocalName { ref device_name } => {
                write!(buffer, "AT+UBTLN={}", device_name).unwrap();
                buffer
            }
            Command::GetLocalCOD => String::from("AT+UBTLC?"),
            Command::SetLocalCOD { .. } => String::from(""),
            Command::GetMasterSlaveRole { ref bd_addr } => {
                write!(buffer, "AT+UBTMSR={}", bd_addr).unwrap();
                buffer
            }
            Command::GetRolePolicy => String::from("AT+UBTMSP?"),
            Command::SetRolePolicy { ref role_policy } => {
                write!(buffer, "AT+UBTMSP={}", *role_policy as u8).unwrap();
                buffer
            }
            Command::GetRSSI { bd_addr } => {
                write!(buffer, "AT+UBTRSS={}", bd_addr).unwrap();
                buffer
            }
            Command::GetLinkQuality { bd_addr } => {
                write!(buffer, "AT++UBTLQ={}", bd_addr).unwrap();
                buffer
            }
            Command::GetRoleConfiguration => String::from("AT+UBTLE?"),
            Command::SetRoleConfiguration { ref role } => {
                write!(buffer, "AT+UBTLE={}", *role as u8).unwrap();
                buffer
            }
            Command::GetLEAdvertiseData => String::from("AT+UBTAD?"),
            Command::SetLEAdvertiseData { data } => {
                // TODO: Correct write! of data array?
                write!(buffer, "AT+UBTAD={:?}", data).unwrap();
                buffer
            }
            Command::GetLEScanResponseData => String::from("AT+UBTSD?"),
            Command::SetLEScanResponseData { data } => {
                // TODO: Correct write! of data array?
                write!(buffer, "AT+UBTSD={:?}", data).unwrap();
                buffer
            }
            Command::ServiceSearch {
                bd_addr,
                ref service_type,
                uuid: _,
            } => {
                // TODO: uuid?
                write!(buffer, "AT+UBTSS={},{}", bd_addr, *service_type as u8).unwrap();
                buffer
            }
            // GetWatchdogParameter(u8),
            // SetWatchdogParameter(u8, u8),
            // GetBTConfig(u8),
            // SetBTConfig(u8, u8),
            // GetBTLEConfig(u8),
            // SetBTLEConfig(u8, u8),
            Command::STAGetConfig {
                ref configuration_id,
                ref param_tag,
            } => {
                if *param_tag == UWSCGetTag::All {
                    write!(buffer, "AT+UWSC={}", *configuration_id).unwrap();
                } else {
                    write!(buffer, "AT+UWSC={},{}", *configuration_id, *param_tag as u8).unwrap();
                }
                buffer
            }
            Command::STASetConfig {
                ref configuration_id,
                ref param_tag,
            } => {
                write!(buffer, "AT+UWSC={}", *configuration_id).unwrap();
                match param_tag {
                    UWSCSetTag::ActiveOnStartup(ref enable) => {
                        write!(buffer, ",0,{}", *enable as u8).unwrap();
                    }
                    UWSCSetTag::SSID(ssid) => {
                        write!(buffer, ",2,{}", ssid).unwrap();
                    }
                    UWSCSetTag::BSSID(ref bssid) => {
                        write!(buffer, ",3,{}", bssid).unwrap();
                    }
                    UWSCSetTag::Authentication(ref authentification_type) => {
                        write!(buffer, ",5,{}", *authentification_type as u8).unwrap();
                    }
                    UWSCSetTag::WEPKeys(ref _key1, ref _key2, ref _key3, ref _key4) => {
                        // TODO: Key[1-4] ?
                        write!(buffer, ",6").unwrap();
                    }
                    UWSCSetTag::ActiveKey(ref key) => {
                        write!(buffer, ",7,{}", key).unwrap();
                    }
                    UWSCSetTag::Passphrase(ref passphrase) => {
                        write!(buffer, ",8,{}", passphrase).unwrap();
                    }
                    UWSCSetTag::Password(ref password) => {
                        write!(buffer, ",9,{}", password).unwrap();
                    }
                    UWSCSetTag::PublicUserName(ref username) => {
                        write!(buffer, ",10,{}", username).unwrap();
                    }
                    UWSCSetTag::PublicDomainName(ref domainname) => {
                        write!(buffer, ",11,{}", domainname).unwrap();
                    }
                    UWSCSetTag::Ipv4Mode(ref ipv4_mode) => {
                        write!(buffer, ",100,{}", *ipv4_mode as u8).unwrap();
                    }
                    UWSCSetTag::Ipv4Address(ref ipv4_addr) => {
                        write!(buffer, ",101,{}", ipv4_addr).unwrap();
                    }
                    UWSCSetTag::SubnetMask(ref ipv4_addr) => {
                        write!(buffer, ",102,{}", ipv4_addr).unwrap();
                    }
                    UWSCSetTag::DefaultGateway(ref ipv4_addr) => {
                        write!(buffer, ",103,{}", ipv4_addr).unwrap();
                    }
                    UWSCSetTag::PrimaryDns(ref ipv4_addr) => {
                        write!(buffer, ",104,{}", ipv4_addr).unwrap();
                    }
                    UWSCSetTag::SecondaryDns(ref ipv4_addr) => {
                        write!(buffer, ",105,{}", ipv4_addr).unwrap();
                    }
                    UWSCSetTag::Ipv6Mode(ref ipv6_mode) => {
                        write!(buffer, ",200,{}", *ipv6_mode as u8).unwrap();
                    }
                    UWSCSetTag::Ipv6Address(ref ipv6_addr) => {
                        write!(buffer, ",201,{}", ipv6_addr).unwrap();
                    }
                }
                buffer
            }
            Command::ExecSTAAction {
                configuration_id,
                ref action,
            } => {
                write!(buffer, "AT+UWSCA={},{}", configuration_id, *action as u8).unwrap();
                buffer
            }
            Command::STAGetConfigList => String::from("AT+UWASCL"),
            Command::STAScan { ref ssid } => {
                if let Some(ssid) = ssid {
                    write!(buffer, "AT+UWSCAN={}", ssid).unwrap();
                    buffer
                } else {
                    String::from("AT+UWSCAN")
                }
            }
            // Command::STASetChannelList { ref channel_list } => {
            //   write!(buffer, "AT+UWCL={}",
            //     channel_list.to_vec().iter().fold(String::new(), |mut acc, cur| {
            //       if acc.len() > 0 {
            //         acc.push_str(&",");
            //       }
            //       acc.push_str(&cur);
            //       acc
            //     })
            //   ).unwrap();
            //   buffer
            // },
            Command::STAGetChannelList => String::from("AT+UWCL?"),
            Command::WIFIGetWatchdogParameter { ref wd_type } => {
                write!(buffer, "AT+UWWS={}", *wd_type as u8).unwrap();
                buffer
            }
            Command::WIFISetWatchdogParameter { ref wd_type } => {
                match wd_type {
                    WIFIWatchDogTypeSet::DisconnectReset(ref enabled) => {
                        write!(buffer, "AT+UWWS=1,{}", *enabled as u8).unwrap();
                    }
                }
                buffer
            }
            Command::STAGetStatus { ref status_id } => {
                if *status_id == STAStatus::All {
                    write!(buffer, "AT+UWSSTAT").unwrap();
                } else {
                    write!(buffer, "AT+UWSSTAT={}", *status_id as u8).unwrap();
                }
                buffer
            }

            Command::GetHostname => String::from("AT+UNHN?"),
            Command::SetHostname { ref hostname } => {
                if hostname.len() > 64 {
                    // TODO: Substring for the first 64 chars?
                }
                write!(buffer, "AT+UNHN={}", hostname).unwrap();
                buffer
            }
            Command::GetNetworkStatus {
                ref interface_type,
                ref status_id,
            } => {
                write!(
                    buffer,
                    "AT+UNSTAT={},{}",
                    *interface_type as u8, *status_id as u8
                )
                .unwrap();
                buffer
            }
            _ => String::from(""),
        }
    }

    fn parse_resp(
        &self,
        response_lines: &mut Vec<String<at::MaxCommandLen>, at::MaxResponseLines>,
    ) -> ResponseType {
        if response_lines.is_empty() {
            return ResponseType::None;
        }

        // Handle list items
        let mut responses = at::utils::split_parameterized_resp(response_lines);

        let response = responses.pop().unwrap();
        match *self {
            Command::AT => ResponseType::None,
            Command::GetManufacturerId => ResponseType::SingleSolicited(Response::ManufacturerId {
                id: String::from(response[0]),
            }),
            Command::STASetConfig { .. } => ResponseType::SingleSolicited(Response::STASetConfig {
                configuration_id: response[0].parse::<u8>().unwrap(),
                param_tag: response[1..].into(),
            }),
            _ => ResponseType::None,
        }
    }

    fn parse_unsolicited(response_line: &str) -> ResponseType {
        let (cmd, response) = at::utils::split_parameterized_unsolicited(response_line);
        match cmd {
            _ => ResponseType::None,
        }
    }
}
