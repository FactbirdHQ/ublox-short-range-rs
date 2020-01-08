
use crate::{
  ATClient,
  command::*,
};

use at::ATInterface;
use embedded_hal::timer::CountDown;
use log::info;

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
          },
          Err(e) => Err(nb::Error::Other(e)),
          _ => Err(nb::Error::WouldBlock)

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

  pub fn init(&mut self) -> Result<(), at::Error>{
    // Initilize a new ublox device to a known state (set RS232 settings, restart, wait for startup etc.)
    self.send_at(Command::SetRS232Settings {
      baud_rate: BaudRate::Baud115200,
      flow_control: FlowControl::NotUsed,
      data_bits: 8,
      stop_bits: StopBits::StopBits1,
      parity: Parity::NoParity,
      change_after_confirm: ChangeAfterConfirm::NoChange,
    })?;

    self.send_at(Command::Store)?;
    self.send_at(Command::PwrOff)?;

    block!(wait_for_unsolicited!(self, UnsolicitedResponse::Startup)).unwrap();

    self.initialized = true;
    Ok(())
  }

  pub fn send_at(&mut self, cmd: Command) -> Result<ResponseType, at::Error> {
    match self.serial_mode {
      SerialMode::Cmd => self.client.send(cmd),
      _ => Err(at::Error::Write),
    }
  }
}
