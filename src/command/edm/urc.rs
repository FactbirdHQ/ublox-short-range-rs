use super::calc_payload_len;
use super::types::*;
use super::Urc;
use atat::helpers::LossyStr;
use atat::AtatUrc;
use core::net::{Ipv4Addr, Ipv6Addr};
use heapless::Vec;
use ublox_sockets::ChannelId;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
pub enum EdmEvent {
    BluetoothConnectEvent(BluetoothConnectEvent),
    IPv4ConnectEvent(IPv4ConnectEvent),
    IPv6ConnectEvent(IPv6ConnectEvent),
    /// Disconnect wrapping Channel Id
    DisconnectEvent(ChannelId),
    DataEvent(DataEvent),
    ATEvent(Urc),
    // TODO: Handle module restart. Especially to Digest
    StartUp,
}

impl EdmEvent {
    pub fn extract_urc(self) -> Option<Urc> {
        match self {
            EdmEvent::ATEvent(urc) => Some(urc),
            _ => None,
        }
    }
}

impl AtatUrc for EdmEvent {
    /// The type of the response. Usually the enum this trait is implemented on.
    type Response = Self;

    /// Parse the response into a `Self::Response` instance.
    fn parse(resp: &[u8]) -> Option<Self::Response> {
        trace!("[Parse URC] {:?}", LossyStr(resp));
        // Startup message?
        // TODO: simplify mayby no packet check.
        if resp.len() >= STARTUPMESSAGE.len()
            && resp[..2] == *b"\r\n"
            && resp[resp.len() - 2..] == *b"\r\n"
        {
            if resp == STARTUPMESSAGE {
                return EdmEvent::ATEvent(Urc::StartUp).into();
            } else if resp.len() == AUTOCONNECTMESSAGE.len()
                || resp.len() == AUTOCONNECTMESSAGE.len() + 1
            {
                let mut urc = resp;
                if let Some(i) = urc.iter().position(|x| !x.is_ascii_whitespace()) {
                    urc = &urc[i..];
                };

                let cmd = Urc::parse(urc)?;
                return EdmEvent::ATEvent(cmd).into();
            }
        }
        if resp
            .windows(STARTUPMESSAGE.len())
            .position(|window| window == STARTUPMESSAGE)
            == Some(0)
        {
            return EdmEvent::StartUp.into();
        }

        if resp.len() < PAYLOAD_OVERHEAD
            || !resp.starts_with(&[STARTBYTE])
            || !resp.ends_with(&[ENDBYTE])
        {
            error!("[Parse URC Start/End byte Error] {:?}", LossyStr(resp));
            return None;
        };
        let payload_len = calc_payload_len(resp);
        if resp.len() != payload_len + EDM_OVERHEAD {
            error!("[Parse URC length Error] {:?}", LossyStr(resp));
            return None;
        }

        match resp[4].into() {
            PayloadType::ATEvent => {
                let mut urc = &resp[AT_COMMAND_POSITION..PAYLOAD_POSITION + payload_len];
                if let Some(i) = urc.iter().position(|x| !x.is_ascii_whitespace()) {
                    urc = &urc[i..];
                };
                let cmd = Urc::parse(urc)?;
                EdmEvent::ATEvent(cmd).into()
            }

            PayloadType::ConnectEvent => {
                if payload_len < 4 {
                    return None;
                }

                match resp[6].into() {
                    ConnectType::IPv4 => {
                        if payload_len != 17 {
                            return None;
                        }
                        let event = IPv4ConnectEvent {
                            channel_id: ChannelId(resp[5]),
                            protocol: resp[7].into(),
                            remote_ip: Ipv4Addr::from([resp[8], resp[9], resp[10], resp[11]]),
                            remote_port: ((resp[12] as u16) << 8) | resp[13] as u16,
                            local_ip: Ipv4Addr::from([resp[14], resp[15], resp[16], resp[17]]),
                            local_port: ((resp[18] as u16) << 8) | resp[19] as u16,
                        };

                        if event.protocol == Protocol::Unknown {
                            return None;
                        }
                        EdmEvent::IPv4ConnectEvent(event).into()
                    }
                    ConnectType::IPv6 => {
                        if payload_len != 41 {
                            return None;
                        }
                        let event = IPv6ConnectEvent {
                            channel_id: ChannelId(resp[5]),
                            protocol: resp[7].into(),
                            remote_ip: Ipv6Addr::from([
                                resp[8], resp[9], resp[10], resp[11], resp[12], resp[13], resp[14],
                                resp[15], resp[16], resp[17], resp[18], resp[19], resp[20],
                                resp[21], resp[22], resp[23],
                            ]),
                            remote_port: ((resp[24] as u16) << 8) | resp[25] as u16,
                            local_ip: Ipv6Addr::from([
                                resp[26], resp[27], resp[28], resp[29], resp[30], resp[31],
                                resp[32], resp[33], resp[34], resp[35], resp[36], resp[37],
                                resp[38], resp[39], resp[40], resp[41],
                            ]),
                            local_port: ((resp[42] as u16) << 8) | resp[43] as u16,
                        };

                        if event.protocol == Protocol::Unknown {
                            return None;
                        }
                        EdmEvent::IPv6ConnectEvent(event).into()
                    }
                    _ => None,
                }
            }

            PayloadType::DisconnectEvent => {
                if payload_len != 3 {
                    return None;
                }
                EdmEvent::DisconnectEvent(ChannelId(resp[5])).into()
            }

            PayloadType::DataEvent => {
                if payload_len < 4 {
                    return None;
                }

                Vec::from_slice(&resp[6..payload_len + 3])
                    .ok()
                    .map(|vec| DataEvent {
                        channel_id: ChannelId(resp[5]),
                        data: vec,
                    })
                    .map(EdmEvent::DataEvent)
            }

            PayloadType::StartEvent => EdmEvent::StartUp.into(),

            _ => {
                error!("[Parse URC Error] {:?}", LossyStr(resp));
                None
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::command::{data_mode::urc::PeerConnected, edm::types::DATA_PACKAGE_SIZE, Urc};
    use atat::{heapless::Vec, heapless_bytes::Bytes, AtatUrc};
    use ublox_sockets::PeerHandle;

    #[test]
    fn parse_at_urc() {
        // AT-urc: +UUDPD:3
        let resp = &[
            170, 0, 46, 0, 65, 13, 10, 43, 85, 85, 68, 80, 67, 58, 50, 44, 50, 44, 49, 44, 48, 46,
            48, 46, 48, 46, 48, 44, 48, 44, 49, 54, 50, 46, 49, 53, 57, 46, 50, 48, 48, 46, 49, 44,
            49, 50, 51, 13, 10, 85,
        ];
        // URC: b"\xaa\x00.\x00A\r\n+UUDPC:2,2,1,0.0.0.0,0,162.159.200.1,123\r\nU"
        let urc = EdmEvent::ATEvent(Urc::PeerConnected(PeerConnected {
            handle: PeerHandle(2),
            connection_type: crate::command::data_mode::types::ConnectionType::IPv4,
            protocol: crate::command::data_mode::types::IPProtocol::UDP,
            local_address: Bytes::from_slice("0.0.0.0".as_bytes()).unwrap(),
            local_port: 0,
            remote_address: Bytes::from_slice("162.159.200.1".as_bytes()).unwrap(),
            remote_port: 123,
        }));
        let parsed_urc = EdmEvent::parse(resp);
        assert_eq!(parsed_urc, Some(urc), "Parsing URC failed");
    }

    #[test]
    fn parse_ipv4_connect_event() {
        // AT-urc: +UUDPD:3
        let resp = &[
            0xAA, 0x00, 0x11, 0x00, 0x11, 0x05, 0x02, 0x00, 0xC0, 0xA8, 0x00, 0x02, 0x13, 0x88,
            0xC0, 0xA8, 0x00, 0x01, 0x0F, 0xA0, 0x55,
        ];
        let event = EdmEvent::IPv4ConnectEvent(IPv4ConnectEvent {
            channel_id: ChannelId(5),
            protocol: Protocol::TCP,
            remote_ip: Ipv4Addr::new(192, 168, 0, 2),
            remote_port: 5000,
            local_ip: Ipv4Addr::new(192, 168, 0, 1),
            local_port: 4000,
        });
        let parsed_event = EdmEvent::parse(resp);
        assert_eq!(
            parsed_event,
            Some(event),
            "Parsing IPv4 Connect Event failed"
        );
    }

    #[test]
    fn parse_ipv6_connect_event() {
        // AT-event: +UUDPD:3
        let resp = &[
            0xAA, 0x00, 0x29, 0x00, 0x11, 0x05, 0x03, 0x00, 0xFE, 0x80, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x13, 0x88, 0xFE, 0x80,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
            0x0F, 0xA0, 0x55,
        ];
        let event = EdmEvent::IPv6ConnectEvent(IPv6ConnectEvent {
            channel_id: ChannelId(5),
            protocol: Protocol::TCP,
            remote_ip: Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 2),
            remote_port: 5000,
            local_ip: Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1),
            local_port: 4000,
        });
        let parsed_event = EdmEvent::parse(resp);
        assert_eq!(
            parsed_event,
            Some(event),
            "Parsing IPv6 Connect Event failed"
        );
    }

    #[test]
    fn parse_disconnect_event() {
        // AT-event: +UUDPD:3
        let resp = &[0xAA, 0x00, 0x03, 0x00, 0x21, 0x03, 0x55];
        let event = EdmEvent::DisconnectEvent(ChannelId(3));
        let parsed_event = EdmEvent::parse(resp);
        assert_eq!(parsed_event, Some(event), "Parsing Disconnect Event failed");
    }

    #[test]
    fn parse_data_event() {
        // AT-event: +UUDPD:3
        let resp = &[0xAA, 0x00, 0x05, 0x00, 0x31, 0x03, 0x12, 0x34, 0x55];
        let event = EdmEvent::DataEvent(DataEvent {
            channel_id: ChannelId(3),
            data: Vec::<u8, DATA_PACKAGE_SIZE>::from_slice(&[0x12, 0x34]).unwrap(),
        });
        let parsed_event = EdmEvent::parse(resp);
        assert_eq!(parsed_event, Some(event), "Parsing Data Event failed");
    }
}
