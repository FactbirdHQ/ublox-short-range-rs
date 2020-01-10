use crate::{command::*, ATClient};

use at::ATInterface;
use embedded_hal::timer::CountDown;
use log::info;

use edm::Packet;

#[macro_export]
macro_rules! wait_for_unsolicited {
    ($client:expr, $p:pat) => {{
        let mut res: nb::Result<UnsolicitedResponse, at::Error> = Err(nb::Error::WouldBlock);
        if let Ok(ResponseType::Unsolicited(_)) = $client.client.peek_response() {
            res = match $client.client.wait_response() {
                Ok(ResponseType::Unsolicited(r)) => {
                    info!("{:?}", r);
                    if let $p = r {
                        Ok(r)
                    } else {
                        Err(nb::Error::WouldBlock)
                    }
                }
                Err(e) => Err(nb::Error::Other(e)),
                _ => Err(nb::Error::WouldBlock),
            }
        }
        res
    }};
}

#[derive(PartialEq)]
pub enum SerialMode {
    Cmd,
    Data,
    ExtendedData,
}

macro_rules! size_of {
    ($type:ident) => {
        info!(
            "Size of {}: {:?}",
            stringify!($type),
            core::mem::size_of::<$type>()
        );
    };
}

pub struct UbloxClient<T>
where
    T: CountDown,
    T::Time: Copy,
{
    initialized: bool,
    serial_mode: SerialMode,
    pub(crate) client: ATClient<T>,
}

impl<T> UbloxClient<T>
where
    T: CountDown,
    T::Time: Copy,
{
    pub fn new(client: ATClient<T>) -> Self {
        UbloxClient {
            initialized: false,
            serial_mode: SerialMode::Cmd,
            client,
        }
    }

    pub fn init(&mut self) -> Result<(), at::Error> {
        // Initilize a new ublox device to a known state (set RS232 settings, restart, wait for startup etc.)
        size_of!(Command);
        size_of!(Response);
        size_of!(ResponseType);
        size_of!(Packet);

        self.send_internal(Command::SetRS232Settings {
            baud_rate: BaudRate::Baud115200,
            flow_control: FlowControl::NotUsed,
            data_bits: 8,
            stop_bits: StopBits::StopBits1,
            parity: Parity::NoParity,
            change_after_confirm: ChangeAfterConfirm::NoChange,
        })?;

        self.send_internal(Command::Store)?;
        self.send_internal(Command::PwrOff)?;

        block!(wait_for_unsolicited!(self, UnsolicitedResponse::Startup)).unwrap();

        self.send_internal(Command::AT)?;

        self.initialized = true;
        Ok(())
    }

    fn send_internal(&mut self, cmd: Command) -> Result<ResponseType, at::Error> {
        match self.serial_mode {
            SerialMode::Cmd => self.client.send(RequestType::Cmd(cmd)),
            SerialMode::Data => Err(at::Error::Write),
            SerialMode::ExtendedData => {
                // edm::Packet::new(edm::Identifier::AT, edm::Type::Request, cmd.get_cmd().into_bytes());
                Err(at::Error::Write)
            }
        }
    }

    pub fn send_at(&mut self, cmd: Command) -> Result<ResponseType, at::Error> {
        if !self.initialized {
            self.init()?
        }

        self.send_internal(cmd)
    }
}
