// pub mod responses;
pub mod types;
pub mod urc;

use core::convert::TryInto;

use crate::command::{data_mode, data_mode::ChangeMode};
use crate::command::{NoResponse, Urc};
// use crate::wifi::EGRESS_CHUNK_SIZE;
/// Containing EDM structs with custom serialaization and deserilaisation.
use atat::AtatCmd;
use heapless::Vec;
use types::*;
use ublox_sockets::ChannelId;

pub(crate) fn calc_payload_len(resp: &[u8]) -> usize {
    (u16::from_be_bytes(resp[1..3].try_into().unwrap()) & EDM_FULL_SIZE_FILTER) as usize
}
/// EDM wrapper for AT-Commands
// Note:
// The AT+UMRS command to change serial settings does not work exactly the same as in command
// mode. When executed in the extended data mode, it is not possible to change the settings directly
// using the <change_after_confirm> parameter. Instead the <change_after_confirm> parameter must
// be set to 0 and the serial settings will take effect when the module is reset.
#[derive(Debug, Clone)]
pub(crate) struct EdmAtCmdWrapper<T: AtatCmd>(pub T);

impl<T: AtatCmd> atat::AtatCmd for EdmAtCmdWrapper<T> {
    type Response = T::Response;

    const MAX_LEN: usize = T::MAX_LEN + 6;

    const MAX_TIMEOUT_MS: u32 = T::MAX_TIMEOUT_MS;

    fn write(&self, buf: &mut [u8]) -> usize {
        let at_len = self.0.write(&mut buf[5..]);
        let payload_len = (at_len + 2) as u16;

        buf[0..5].copy_from_slice(&[
            STARTBYTE,
            (payload_len >> 8) as u8 & EDM_SIZE_FILTER,
            (payload_len & 0xffu16) as u8,
            0x00,
            PayloadType::ATRequest as u8,
        ]);

        buf[5 + at_len] = ENDBYTE;

        5 + at_len + 1
    }

    fn parse(
        &self,
        resp: Result<&[u8], atat::InternalError>,
    ) -> core::result::Result<Self::Response, atat::Error> {
        let resp = resp.and_then(|resp| {
            if resp.len() < PAYLOAD_OVERHEAD
                || !resp.starts_with(&[STARTBYTE])
                || !resp.ends_with(&[ENDBYTE])
            {
                return Err(atat::InternalError::InvalidResponse);
            };

            let payload_len = calc_payload_len(resp);

            if resp.len() != payload_len + EDM_OVERHEAD
                || resp[4] != PayloadType::ATConfirmation as u8
            {
                return Err(atat::InternalError::InvalidResponse);
            }

            // Recieved OK response code in EDM response?
            match resp
                .windows(b"\r\nOK".len())
                .position(|window| window == b"\r\nOK")
            {
                // Cutting OK out leaves an empth string for NoResponse, for
                // other responses just removes "\r\nOK\r\n"
                Some(pos) => Ok(&resp[AT_COMMAND_POSITION..pos]),
                // Isolate the AT_response
                None => Ok(&resp[AT_COMMAND_POSITION..PAYLOAD_POSITION + payload_len]),
            }
        });

        self.0.parse(resp)
    }
}

#[derive(Debug, Clone)]
pub struct EdmDataCommand<'a> {
    pub channel: ChannelId,
    pub data: &'a [u8],
}
// wifi::socket::EGRESS_CHUNK_SIZE + PAYLOAD_OVERHEAD = 512 + 6 + 1 = 519
impl<'a> atat::AtatCmd for EdmDataCommand<'a> {
    type Response = NoResponse;

    const MAX_LEN: usize = DATA_PACKAGE_SIZE + 7;

    const EXPECTS_RESPONSE_CODE: bool = false;

    fn parse(
        &self,
        _resp: Result<&[u8], atat::InternalError>,
    ) -> core::result::Result<Self::Response, atat::Error> {
        Ok(NoResponse)
    }

    fn write(&self, buf: &mut [u8]) -> usize {
        let payload_len = (self.data.len() + 3) as u16;
        buf[0..6].copy_from_slice(&[
            STARTBYTE,
            (payload_len >> 8) as u8 & EDM_SIZE_FILTER,
            (payload_len & 0xffu16) as u8,
            0x00,
            PayloadType::DataCommand as u8,
            self.channel.0,
        ]);

        buf[6..6 + self.data.len()].copy_from_slice(self.data);
        buf[6 + self.data.len()] = ENDBYTE;

        6 + self.data.len() + 1
    }
}

#[derive(Debug, Clone)]
pub struct EdmResendConnectEventsCommand;

impl atat::AtatCmd for EdmResendConnectEventsCommand {
    type Response = NoResponse;

    const MAX_LEN: usize = 6;

    fn write(&self, buf: &mut [u8]) -> usize {
        buf[0..6].copy_from_slice(&[
            STARTBYTE,
            0x00,
            0x02,
            0x00,
            PayloadType::ResendConnectEventsCommand as u8,
            ENDBYTE,
        ]);

        6
    }

    fn parse(
        &self,
        _resp: Result<&[u8], atat::InternalError>,
    ) -> core::result::Result<Self::Response, atat::Error> {
        Ok(NoResponse)
    }
}

#[derive(Debug, Clone)]
pub struct SwitchToEdmCommand;

impl atat::AtatCmd for SwitchToEdmCommand {
    type Response = NoResponse;

    const MAX_LEN: usize = 6;

    const MAX_TIMEOUT_MS: u32 = 2000;

    fn write(&self, buf: &mut [u8]) -> usize {
        ChangeMode {
            mode: data_mode::types::Mode::ExtendedDataMode,
        }
        .write(buf)
    }

    fn parse(
        &self,
        _resp: Result<&[u8], atat::InternalError>,
    ) -> core::result::Result<Self::Response, atat::Error> {
        // let resp = resp?;
        // // Parse EDM startup command
        // let correct = &[0xAA, 0x00, 0x02, 0x00, 0x71, 0x55]; // &[0xAA,0x00,0x06,0x00,0x45,0x4f,0x4b,0x0D,0x0a,0x55]; // AA 00 06 00 44 41 54 0D 0A 0D 0A 4F 4B 0D 0A 55 ?
        // if resp.len() != correct.len()
        //     || resp[.. correct.len()] != *correct {
        //     // TODO: check this
        //     return Err(atat::Error::InvalidResponse);
        // }
        Ok(NoResponse)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::command::{
        system::{responses::SystemStatusResponse, types::StatusID, SystemStatus},
        AT,
    };
    use atat::{heapless::Vec, AtatCmd, Error};

    #[test]
    fn parse_at_commands() {
        let parse = EdmAtCmdWrapper(AT);
        let correct_response = NoResponse;

        // AT-command: "AT"
        let correct_cmd = Vec::<u8, 10>::from_slice(&[
            0xAA, 0x00, 0x06, 0x00, 0x44, 0x41, 0x54, 0x0D, 0x0a, 0x55,
        ])
        .unwrap();
        // AT-response: NoResponse
        let response = &[
            0xAA,
            0x00,
            0x02,
            0x00,
            PayloadType::ATConfirmation as u8,
            0x55,
        ];

        let mut buf = [0u8; <EdmAtCmdWrapper<AT> as AtatCmd>::MAX_LEN];
        let len = parse.write(&mut buf);

        assert_eq!(buf[..len], correct_cmd);
        assert_eq!(parse.parse(Ok(response)), Ok(correct_response));

        let parse = EdmAtCmdWrapper(SystemStatus {
            status_id: StatusID::SavedStatus,
        });
        let correct_response = SystemStatusResponse {
            status_id: StatusID::SavedStatus,
            status_val: 100,
        };
        // AT-command: "at+umstat=1"
        let correct = Vec::<u8, 19>::from_slice(&[
            0xAA, 0x00, 0x0F, 0x00, 0x44, 0x41, 0x54, 0x2b, 0x55, 0x4d, 0x53, 0x54, 0x41, 0x54,
            0x3d, 0x31, 0x0D, 0x0A, 0x55,
        ])
        .unwrap();
        // AT-response: "at+umstat:1,100"
        let response = &[
            0xAA,
            0x00,
            0x11,
            0x00,
            PayloadType::ATConfirmation as u8,
            0x2B,
            0x55,
            0x4D,
            0x53,
            0x54,
            0x41,
            0x54,
            0x3A,
            0x31,
            0x2C,
            0x31,
            0x30,
            0x30,
            0x0D,
            0x0A,
            0x55,
        ];
        let mut buf = [0u8; <EdmAtCmdWrapper<SystemStatus> as AtatCmd>::MAX_LEN];
        let len = parse.write(&mut buf);

        assert_eq!(buf[..len], correct);
        assert_eq!(parse.parse(Ok(response)), Ok(correct_response));
    }

    #[test]
    fn parse_wrong_at_responses() {
        let parse = EdmAtCmdWrapper(AT);
        // AT-response: NoResponse
        let response = &[
            0xAA,
            0x00,
            0x06,
            0x00,
            PayloadType::ATConfirmation as u8,
            0x55,
        ];
        assert_eq!(
            parse.parse(Ok(response)),
            Err(Error::InvalidResponse),
            "Response shorter than indicated not invalid"
        );

        let parse = EdmAtCmdWrapper(SystemStatus {
            status_id: StatusID::SavedStatus,
        });
        // AT-response: "at+umstat:1,100"
        let response = &[
            0xAA,
            0x00,
            0x01,
            0x00,
            PayloadType::ATConfirmation as u8,
            0x2B,
            0x55,
            0x4D,
            0x53,
            0x54,
            0x41,
            0x54,
            0x3A,
            0x31,
            0x2C,
            0x31,
            0x30,
            0x30,
            0x0D,
            0x0A,
            0x55,
        ];
        assert_eq!(
            parse.parse(Ok(response)),
            Err(Error::InvalidResponse),
            "Response longer than indicated not invalid"
        );

        let response = &[
            0xAA,
            0x00,
            0x11,
            0x00,
            PayloadType::ATConfirmation as u8,
            0x2B,
            0x55,
            0x4D,
            0x53,
            0x54,
            0x41,
            0x54,
            0x3A,
            0x31,
            0x2C,
            0x31,
            0x30,
            0x30,
            0x0D,
            0x0A,
            0x00,
        ];
        assert_eq!(
            parse.parse(Ok(response)),
            Err(Error::InvalidResponse),
            "Response wrong endbyte not invalid"
        );

        let response = &[
            0x00u8,
            0x00,
            0x11,
            0x00,
            PayloadType::ATConfirmation as u8,
            0x2B,
            0x55,
            0x4D,
            0x53,
            0x54,
            0x41,
            0x54,
            0x3A,
            0x31,
            0x2C,
            0x31,
            0x30,
            0x30,
            0x0D,
            0x0A,
            0x55,
        ];
        assert_eq!(
            parse.parse(Ok(response)),
            Err(Error::InvalidResponse),
            "Response wrong startbyte not invalid"
        );

        let response = &[
            0xAA,
            0x00,
            0x02,
            0x00,
            PayloadType::ATConfirmation as u8,
            0x55,
        ];
        assert_eq!(
            parse.parse(Ok(response)),
            Err(Error::Parse),
            "Response wrong not invalid"
        );
    }

    #[test]
    fn change_to_edm_cmd() {
        let resp = &[0xAA, 0x00, 0x02, 0x00, 0x71, 0x55];
        let correct = Vec::<_, 6>::from_slice(b"ATO2\r\n").unwrap();

        let mut buf = [0u8; SwitchToEdmCommand::MAX_LEN];
        let len = SwitchToEdmCommand.write(&mut buf);

        assert_eq!(buf[..len], correct);
        assert_eq!(SwitchToEdmCommand.parse(Ok(resp)).unwrap(), NoResponse);
    }
}
