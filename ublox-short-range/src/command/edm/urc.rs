use super::{NoResponse, Urc};
use atat::AtatUrc;
use heapless::{consts, String, Vec};
use no_std_net::{Ipv4Addr, Ipv6Addr};
// use responses::*;
use super::calc_payload_len;
use super::types::*;

#[derive(Debug, Clone, PartialEq)]
pub enum EdmEvent {
    BluetoothConnectEvent(BluetoothConnectEvent),
    IPv4ConnectEvent(IPv4ConnectEvent),
    IPv6ConnectEvent(IPv6ConnectEvent),
    /// Disconnect wrapping Channel Id
    DisconnectEvent(ChannelId),
    DataEvent(DataEvent),
    ATEvent(Urc),
    // TODO: Handle modlue restart. Especially to Digest
    StartUp,
}

impl AtatUrc for EdmEvent {
    /// The type of the response. Usually the enum this trait is implemented on.
    type Response = EdmEvent;

    /// Parse the response into a `Self::Response` instance.
    fn parse(resp: &[u8]) -> Result<Self::Response, atat::Error> {
        //
        // defmt::info!("[Parse URC] {:?}", resp);
        //Startup message?
        //TODO: simplify maby no packet check.
        if resp
            .windows(STARTUPMESSAGE.len())
            .position(|window| window == STARTUPMESSAGE)
            == Some(0)
        {
            return Ok(EdmEvent::StartUp);
        }

        if resp.len() < PAYLOAD_OVERHEAD
            || !resp.starts_with(&[STARTBYTE])
            || !resp.ends_with(&[ENDBYTE])
        {
            //
            // defmt::info!("[Parse URC Error] {:?}", resp);
            return Err(atat::Error::InvalidResponse);
        };
        let payload_len = calc_payload_len(resp);
        if resp.len() != payload_len + EDM_OVERHEAD {
            //
            // defmt::info!("[Parse URC Error] {:?}", resp);
            return Err(atat::Error::InvalidResponse);
        }

        match resp[4].into() {
            PayloadType::ATEvent => {
                //
                // defmt::info!("[Parse URC AT-CMD]: {:?}", &resp[AT_COMMAND_POSITION .. PAYLOAD_POSITION + payload_len]);
                let cmd = Urc::parse(&resp[AT_COMMAND_POSITION..PAYLOAD_POSITION + payload_len])?;
                Ok(EdmEvent::ATEvent(cmd))
            }

            PayloadType::ConnectEvent => {
                if payload_len < 4 {
                    return Err(atat::Error::InvalidResponse);
                }

                match resp[6].into() {
                    ConnectType::IPv4 => {
                        if payload_len != 17 {
                            return Err(atat::Error::InvalidResponse);
                        }
                        let event = IPv4ConnectEvent {
                            channel_id: resp[5],
                            protocol: resp[7].into(),
                            remote_ip: Ipv4Addr::from([resp[8], resp[9], resp[10], resp[11]]),
                            remote_port: ((resp[12] as u16) << 8) | resp[13] as u16,
                            local_ip: Ipv4Addr::from([resp[14], resp[15], resp[16], resp[17]]),
                            local_port: ((resp[18] as u16) << 8) | resp[19] as u16,
                        };

                        if event.protocol == Protocol::Unknown {
                            return Err(atat::Error::InvalidResponse);
                        }
                        Ok(EdmEvent::IPv4ConnectEvent(event))
                    }
                    ConnectType::IPv6 => {
                        if payload_len != 41 {
                            return Err(atat::Error::InvalidResponse);
                        }
                        let event = IPv6ConnectEvent {
                            channel_id: resp[5],
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
                            return Err(atat::Error::InvalidResponse);
                        }
                        Ok(EdmEvent::IPv6ConnectEvent(event))
                    }
                    _ => Err(atat::Error::InvalidResponse),
                }
            }

            PayloadType::DisconnectEvent => {
                if payload_len != 3 {
                    return Err(atat::Error::InvalidResponse);
                }
                Ok(EdmEvent::DisconnectEvent(resp[5]))
            }

            PayloadType::DataEvent => {
                if payload_len < 4 {
                    return Err(atat::Error::InvalidResponse);
                }

                let vec: Vec<u8, DataPackageSize> = Vec::from_slice(&resp[6..payload_len + 3])
                    .map_err(|_e| atat::Error::InvalidResponse)?;
                let event = DataEvent {
                    channel_id: resp[5],
                    data: vec,
                };
                Ok(EdmEvent::DataEvent(event))
            }

            _ => {
                //
                // defmt::info!("[Parse URC Error] {:?}", resp);
                Err(atat::Error::InvalidResponse)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::command::{data_mode::urc::PeerDisconnected, edm::types::DataPackageSize, Urc};
    use atat::{
        heapless::{consts, Vec},
        AtatCmd, AtatLen, AtatResp, AtatUrc, Error,
    };

    #[test]
    fn parse_at_urc() {
        // AT-urc: +UUDPD:3
        let resp = &[
            // 0xAAu8, 0x00, 0x0E, 0x00, PayloadType::ATEvent as u8, 0x0D, 0x0A, 0x2B, 0x55, 0x55, 0x44, 0x50, 0x44, 0x3A, 0x33, 0x0D, 0x0A, 0x55,
            0xAAu8,
            0x00,
            0x0C,
            0x00,
            PayloadType::ATEvent as u8,
            0x2B,
            0x55,
            0x55,
            0x44,
            0x50,
            0x44,
            0x3A,
            0x33,
            0x0D,
            0x0A,
            0x55,
            // 0xAAu8, 0x00, 0x1B, 0x00, 0x41, 0x2B, 0x55, 0x55, 0x57, 0x4C, 0x45, 0x3A, 0x30, 0x2C, 0x33, 0x32, 0x41, 0x42, 0x36, 0x41, 0x37, 0x41, 0x34, 0x30, 0x34, 0x34, 0x2C, 0x31, 0x0D, 0x0A, 0x55,
        ];
        let urc = EdmEvent::ATEvent(Urc::PeerDisconnected(PeerDisconnected { handle: 3 }));
        let parsed_urc = EdmEvent::parse(resp);
        assert_eq!(parsed_urc, Ok(urc), "Parsing URC failed");
    }

    #[test]
    fn parse_ipv4_connect_event() {
        // AT-urc: +UUDPD:3
        let resp = &[
            0xAA, 0x00, 0x11, 0x00, 0x11, 0x05, 0x02, 0x00, 0xC0, 0xA8, 0x00, 0x02, 0x13, 0x88,
            0xC0, 0xA8, 0x00, 0x01, 0x0F, 0xA0, 0x55,
        ];
        let event = EdmEvent::IPv4ConnectEvent(IPv4ConnectEvent {
            channel_id: 5,
            protocol: Protocol::TCP,
            remote_ip: Ipv4Addr::new(192, 168, 0, 2),
            remote_port: 5000,
            local_ip: Ipv4Addr::new(192, 168, 0, 1),
            local_port: 4000,
        });
        let parsed_event = EdmEvent::parse(resp);
        assert_eq!(parsed_event, Ok(event), "Parsing IPv4 Connect Event failed");
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
            channel_id: 5,
            protocol: Protocol::TCP,
            remote_ip: Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 2),
            remote_port: 5000,
            local_ip: Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1),
            local_port: 4000,
        });
        let parsed_event = EdmEvent::parse(resp);
        assert_eq!(parsed_event, Ok(event), "Parsing IPv6 Connect Event failed");
    }

    #[test]
    fn parse_disconnect_event() {
        // AT-event: +UUDPD:3
        let resp = &[0xAA, 0x00, 0x03, 0x00, 0x21, 0x03, 0x55];
        let event = EdmEvent::DisconnectEvent(3);
        let parsed_event = EdmEvent::parse(resp);
        assert_eq!(parsed_event, Ok(event), "Parsing Disconnect Event failed");
    }

    #[test]
    fn parse_data_event() {
        // AT-event: +UUDPD:3
        let resp = &[0xAA, 0x00, 0x05, 0x00, 0x31, 0x03, 0x12, 0x34, 0x55];
        let event = EdmEvent::DataEvent(DataEvent {
            channel_id: 3,
            data: Vec::<u8, DataPackageSize>::from_slice(&[0x12, 0x34]).unwrap(),
        });
        let parsed_event = EdmEvent::parse(resp);
        assert_eq!(parsed_event, Ok(event), "Parsing Data Event failed");
    }
}
