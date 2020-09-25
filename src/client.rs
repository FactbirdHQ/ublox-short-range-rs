use atat::AtatClient;
use atat::atat_derive::{AtatCmd, AtatResp};
use core::cell::{Cell, RefCell};
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
    wifi::connection::{WifiConnection, WiFiState},
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

#[derive(PartialEq, Copy, Clone)]
pub enum State{
    Restarting,
    Initializing,
    Idle,
}

#[derive(PartialEq, Copy, Clone)]
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
    pub(crate) state: Cell<State>,
    initialized: Cell<bool>,
    serial_mode: Cell<SerialMode>,
    pub(crate) wifi_connection: RefCell<Option<WifiConnection>>,
    pub(crate) client: RefCell<T>,
}

impl<T> UbloxClient<T>
where
    T: AtatClient
{
    pub fn new(client: T) -> Self {
        UbloxClient {
            state: Cell::new(State::Idle),
            initialized: Cell::new(false),
            serial_mode: Cell::new(SerialMode::Cmd),
            wifi_connection: RefCell::new(None),
            client: RefCell::new(client),
        }
    }

    pub fn init(&mut self) -> Result<(), Error> {
        // Initilize a new ublox device to a known state (set RS232 settings, restart, wait for startup etc.)
        // size_of!(AtatCmd);
        // size_of!(AtatResp);
        // size_of!(ResponseType);
        // size_of!(Packet);

        self.state.set(State::Initializing);

        self.send_internal(&SetRS232Settings {
            baud_rate: BaudRate::B115200,
            flow_control: FlowControl::Off,
            data_bits: 8,
            stop_bits: StopBits::One,
            parity: Parity::None,
            change_after_confirm: ChangeAfterConfirm::ChangeAfterOK,
        }, false)?;

        
        self.send_internal(&StoreCurrentConfig, false)?;
        
        // TODO: Wait for connect
        
        // self.send_internal(&RebootDCE, false)?;
        // block!(wait_for_unsolicited!(self, UnsolicitedResponse::Startup)).unwrap();
        // self.send_internal(&AT, false)?;

        self.initialized.set(true);
        Ok(())
    }


    pub(crate) fn send_internal<A: atat::AtatCmd>(
        &self,
        req: &A,
        check_urc: bool,
    ) -> Result<A::Response, Error> {

        match self.serial_mode.get() {
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
            Some(Urc::WifiLinkConnected(msg)) => {
                #[cfg(feature = "logging")]
                log::info!("[URC] WifiLinkConnected");
                if let Some (ref mut con) = *self.wifi_connection.try_borrow_mut()? {
                        con.state = WiFiState::Connected;
                        con.network.bssid = msg.bssid;
                        con.network.channel = msg.channel;
                }
                Ok(())
            }
            Some(Urc::WifiLinkDisconnected(_)) => {
                #[cfg(feature = "logging")]
                log::info!("[URC] WifiLinkDisconnected{:?}", msg);
                if let Some (ref mut con) = *self.wifi_connection.try_borrow_mut()? {
                    // con.sockets.prune();
                    con.state = WiFiState::Disconnected;
                }
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
                if let Some (ref mut con) = *self.wifi_connection.try_borrow_mut()? {
                    con.state = WiFiState::EthernetUp;
                }
                Ok(())
            }
            Some(Urc::NetworkDown(_)) => {
                #[cfg(feature = "logging")]
                log::info!("[URC] NetworkDown");
                if let Some (ref mut con) = *self.wifi_connection.try_borrow_mut()? {
                    // con.sockets.prune();
                    if con.state == WiFiState::EthernetUp{
                        con.state = WiFiState::Connected;
                    }
                }
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

        pub fn send_at<A: atat::AtatCmd>(&mut self, cmd: &A) -> Result<A::Response, Error> {
        if !self.initialized.get() {
            self.init()?;
        }

        self.send_internal(cmd, true)
    }
}
