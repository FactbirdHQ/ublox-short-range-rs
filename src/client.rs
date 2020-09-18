use atat::AtatClient;
use atat::atat_derive::{AtatCmd, AtatResp};
use core::cell::RefCell;
use embedded_hal::timer::CountDown;
use log::info;
use crate::{
    command::{
        system::{
            SetRS232Settings,
            StoreCurrentConfig,
            RebootDCE,
            types::{
                BaudRate,
                StopBits,
                FlowControl,
                Parity,
                ChangeAfterConfirm,
            }},
        AT,
        Urc,
    },
    error::Error,
};

// use edm::Packet;

#[macro_export]
macro_rules! wait_for_unsolicited {
    ($client:expr, $p:pat) => {{
        let mut res: nb::Result<UnsolicitedResponse, atat::Error> = Err(nb::Error::WouldBlock);
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

#[derive(Copy, Clone)]
pub enum State{
    Restarting,
    Initializing,
    Idle,
    Connecting,
    Connected,
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
    T: AtatClient
{
    state: RefCell<State>,
    initialized: RefCell<bool>,
    serial_mode: RefCell<SerialMode>,
    pub(crate) client: RefCell<T>,
}

impl<T> UbloxClient<T>
where
    T: AtatClient
{
    pub fn new(client: T) -> Self {
        UbloxClient {
            state: RefCell::new(State::Idle),
            initialized: RefCell::new(false),
            serial_mode: RefCell::new(SerialMode::Cmd),
            client: RefCell::new(client),
        }
    }

    pub(crate) fn set_state(&self, state: State) -> Result<State, Error> {
        let prev_state = self.get_state()?;
        *self.state.try_borrow_mut().map_err(|_| Error::SetState)? = state;
        Ok(prev_state)
    }

    pub fn get_state(&self) -> Result<State, Error> {
        Ok(*self.state.try_borrow().map_err(|_| Error::SetState)?)
    }

    pub fn init(&mut self) -> Result<(), Error> {
        // Initilize a new ublox device to a known state (set RS232 settings, restart, wait for startup etc.)
        // size_of!(AtatCmd);
        // size_of!(AtatResp);
        // size_of!(ResponseType);
        // size_of!(Packet);

        self.set_state(State::Initializing)?;

        self.send_internal(&SetRS232Settings {
            baud_rate: BaudRate::B115200,
            flow_control: FlowControl::Off,
            data_bits: 8,
            stop_bits: StopBits::One,
            parity: Parity::None,
            change_after_confirm: ChangeAfterConfirm::ChangeAfterOK,
        }, false)?;

        
        self.send_internal(&StoreCurrentConfig, false)?;
        
        // self.send_internal(&RebootDCE, false)?;
        // block!(wait_for_unsolicited!(self, UnsolicitedResponse::Startup)).unwrap();
        // self.send_internal(&AT, false)?;

        *self.initialized.try_borrow_mut()? = true;
        Ok(())
    }

    // fn send_internal(&mut self, cmd: AtatCmd) -> Result<AtatResp, atat::Error> {
    //     match self.serial_mode {
    //         SerialMode::Cmd => self.client.send(cmd),
    //         SerialMode::Data => Err(atat::Error::Write),
    //         SerialMode::ExtendedData => {
    //             // edm::Packet::new(edm::Identifier::AT, edm::Type::Request, cmd.get_cmd().into_bytes());
    //             Err(atat::Error::Write)
    //         }
    //     }
    // }
    pub(crate) fn send_internal<A: atat::AtatCmd>(
        &self,
        req: &A,
        check_urc: bool,
    ) -> Result<A::Response, Error> {

        match *self.serial_mode.try_borrow()? {
            SerialMode::Cmd => {},
            SerialMode::Data => return Err(Error::AT(atat::Error::Write)),
            SerialMode::ExtendedData => {
                // edm::Packet::new(edm::Identifier::AT, edm::Type::Request, cmd.get_cmd().into_bytes());
                return Err(Error::AT(atat::Error::Write))
            }
        }

        if check_urc {
            if let Err(_e) = self.handle_urc() {
                #[cfg(features = "logging")]
                log::error!("Failed handle URC: {:?}", _e);
            }
        }

        self.client
            .try_borrow_mut()?
            .send(req)
            .map_err(|e| match e {
                nb::Error::Other(ate) => {
                    #[cfg(feature = "logging")]
                    match core::str::from_utf8(&req.as_bytes()) {
                        Ok(s) => log::error!("{:?}: [{:?}]", ate, s),
                        Err(_) => log::error!("{:?}: {:02x?}", ate, req.as_bytes()),
                    };
                    ate.into()
                }
                nb::Error::WouldBlock => Error::_Unknown,
            })
    }

    fn handle_urc(&self) -> Result<(), Error> {
        let urc = self.client.try_borrow_mut()?.check_urc::<Urc>();

        match urc {
            Some(Urc::PeerConnected(_)) => {
                #[cfg(feature = "logging")]
                log::info!("[URC] PeerConnected");
                Ok(())
            }
            Some(Urc::PeerDisconnected(_)) => {
                #[cfg(feature = "logging")]
                log::info!("[URC] PeerDisconnected");
                Ok(())
            }
            Some(Urc::WifiLinkConnected(_)) => {
                #[cfg(feature = "logging")]
                log::info!("[URC] WifiLinkConnected");
                self.set_state(State::Connected)?;
                Ok(())
            }
            Some(Urc::WifiLinkDisconnected(_)) => {
                #[cfg(feature = "logging")]
                log::info!("[URC] WifiLinkDisconnected");
                self.set_state(State::Idle)?;
                Ok(())
            }
            Some(Urc::WifiAPUp(_)) => {
                #[cfg(feature = "logging")]
                log::info!("[URC] WifiAPUp");
                Ok(())
            }
            Some(Urc::WifiAPDown(_)) => {
                #[cfg(feature = "logging")]
                log::info!("[URC] WifiAPDown");
                Ok(())
            }
            Some(Urc::WifiAPStationConnected(_)) => {
                #[cfg(feature = "logging")]
                log::info!("[URC] WifiAPStationConnected");
                Ok(())
            }
            Some(Urc::WifiAPStationDisconnected(_)) => {
                #[cfg(feature = "logging")]
                log::info!("[URC] WifiAPStationDisconnected");
                Ok(())
            }
            Some(Urc::EthernetLinkUp(_)) => {
                #[cfg(feature = "logging")]
                log::info!("[URC] EthernetLinkUp");
                Ok(())
            }
            Some(Urc::EthernetLinkDown(_)) => {
                #[cfg(feature = "logging")]
                log::info!("[URC] EthernetLinkDown");
                Ok(())
            }
            Some(Urc::NetworkUp(_)) => {
                #[cfg(feature = "logging")]
                log::info!("[URC] NetworkUp");
                Ok(())
            }
            Some(Urc::NetworkDown(_)) => {
                #[cfg(feature = "logging")]
                log::info!("[URC] NetworkDown");
                Ok(())
            }
            Some(Urc::NetworkError(_)) => {
                #[cfg(feature = "logging")]
                log::info!("[URC] NetworkError");
                Ok(())
            }
            None => Ok(()),
        }
    }


    // pub fn send_at(&mut self, cmd: AtatCmd) -> Result<AtatResp, atat::Error> {
    //     if !self.initialized {
    //         self.init()?
    //     }

    //     self.send_internal(cmd, true)
    // }

    pub fn send_at<A: atat::AtatCmd>(&mut self, cmd: &A) -> Result<A::Response, Error> {
        if !*self.initialized.try_borrow()? {
            self.init()?;
        }

        self.send_internal(cmd, true)
    }
}
