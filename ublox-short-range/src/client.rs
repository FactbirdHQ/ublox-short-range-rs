use atat::AtatClient;
use atat::atat_derive::{AtatCmd, AtatResp};
use core::cell::{Cell, RefCell};
use embedded_hal::timer::CountDown;
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
            },
        },
        AT,
        Urc,
    },
    wifi::connection::{WifiConnection, WiFiState, NetworkState},
    error::Error,
    sockets::SocketSet,
};
use heapless::{consts, ArrayLength, String};
    
    
#[cfg(feature = "logging")]
use log::info;
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
    Uninitialized,
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
        log::info!(
            "Size of {}: {:?}",
            stringify!($type),
            core::mem::size_of::<$type>()
        );
    };
}

pub struct UbloxClient<C, N, L>
where
    C: AtatClient,
    N: 'static + ArrayLength<Option<crate::sockets::SocketSetItem<L>>>,
    L: 'static + ArrayLength<u8>,
{
    // pub(crate) state: Cell<State>,
    initialized: Cell<bool>,
    serial_mode: Cell<SerialMode>,
    pub(crate) wifi_connection: RefCell<Option<WifiConnection>>,
    pub(crate) client: RefCell<C>,
    pub(crate) sockets: RefCell<&'static mut SocketSet<N, L>>,
}

impl<C, N, L> UbloxClient<C, N, L>
where
    C: AtatClient,
    N: ArrayLength<Option<crate::sockets::SocketSetItem<L>>>,
    L: ArrayLength<u8>,
{
    pub fn new(
        client: C,
        socket_set: &'static mut SocketSet<N, L>,
    ) -> Self {
        UbloxClient {
            // state: Cell::new(State::Uninitialized),
            initialized: Cell::new(false),
            serial_mode: Cell::new(SerialMode::Cmd),
            wifi_connection: RefCell::new(None),
            client: RefCell::new(client),
            sockets: RefCell::new(socket_set),
        }
    }

    // pub fn init(&mut self, hostname: &str) -> Result<(), Error> {
    pub fn init(&mut self) -> Result<(), Error> {
        // Initilize a new ublox device to a known state (set RS232 settings, restart, wait for startup etc.)
        // size_of!(AtatCmd);
        // size_of!(AtatResp);
        // size_of!(ResponseType);
        // size_of!(Packet);

        // self.state.set(State::Initializing);

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

    #[inline]
    fn low_power_mode(&self, _enable: bool) -> Result<(), atat::Error> {
        // if let Some(ref _dtr) = self.config.dtr_pin {
        //     // if enable {
        //     // dtr.set_high().ok();
        //     // } else {
        //     // dtr.set_low().ok();
        //     // }
        //     return Ok(());
        // }
        Ok(())
    }

    #[inline]
    fn autosense(&self) -> Result<(), Error> {
        for _ in 0..15 {
            match self.client.try_borrow_mut()?.send(&AT) {
                Ok(_) => {
                    return Ok(());
                }
                Err(_e) => {}
            };
        }
        Err(Error::BaudDetection)
    }

    #[inline]
    fn reset(&self) -> Result<(), Error> {
        // self.send_internal(
        //     &SetModuleFunctionality {
        //         fun: Functionality::SilentResetWithSimReset,
        //         rst: None,
        //     },
        //     false,
        // )?;
        Ok(())
    }

    pub fn spin(&self) -> Result<(), Error> {
        self.handle_urc()?;

        // match self.state.get() {
        //     State::Attached => {}
        //     State::Sending => {
        //         return Ok(());
        //     }
        //     s => {
        //         return Err(Error::NetworkState(s));
        //     }
        // }

        // // Occasionally poll every open socket, in case a `SocketDataAvailable`
        // // URC was missed somehow. TODO: rewrite this to readable code
        // let data_available: heapless::Vec<(SocketHandle, usize), consts::U4> = {
        //     let sockets = self.sockets.try_borrow()?;

        //     if sockets.len() > 0 && self.poll_cnt(false) >= 500 {
        //         self.poll_cnt(true);

        //         sockets
        //             .iter()
        //             .filter_map(|(h, s)| {
        //                 // Figure out if socket is TCP or UDP
        //                 match s.get_type() {
        //                     SocketType::Tcp => self
        //                         .send_internal(
        //                             &ReadSocketData {
        //                                 socket: h,
        //                                 length: 0,
        //                             },
        //                             false,
        //                         )
        //                         .map_or(None, |s| {
        //                             if s.length > 0 {
        //                                 Some((h, s.length))
        //                             } else {
        //                                 None
        //                             }
        //                         }),
        //                     // SocketType::Udp => self
        //                     //     .send_internal(
        //                     //         &ReadUDPSocketData {
        //                     //             socket: h,
        //                     //             length: 0,
        //                     //         },
        //                     //         false,
        //                     //     )
        //                     //     .map_or(None, |s| {
        //                     //         if s.length > 0 {
        //                     //             Some((h, s.length))
        //                     //         } else {
        //                     //             None
        //                     //         }
        //                     //     }),
        //                     _ => None,
        //                 }
        //             })
        //             .collect()
        //     } else {
        //         heapless::Vec::new()
        //     }
        // };

        // data_available
        //     .iter()
        //     .try_for_each(|(handle, len)| self.socket_ingress(*handle, *len).map(|_| ()))
        //     .map_err(|e| {
        //         #[cfg(feature = "logging")]
        //         log::error!("ERROR: {:?}", e);
        //         e
        //     })?;

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
                        con.wifi_state = WiFiState::Connected;
                        con.network.bssid = msg.bssid;
                        con.network.channel = msg.channel;
                }
                Ok(())
            }
            Some(Urc::WifiLinkDisconnected(msg)) => {
                #[cfg(feature = "logging")]
                log::info!("[URC] WifiLinkDisconnected{:?}", msg);
                if let Some (ref mut con) = *self.wifi_connection.try_borrow_mut()? {
                    // con.sockets.prune();
                    con.wifi_state = WiFiState::Disconnected;
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
                    con.network_state = NetworkState::Attached;
                }
                Ok(())
            }
            Some(Urc::NetworkDown(_)) => {
                #[cfg(feature = "logging")]
                log::info!("[URC] NetworkDown");
                if let Some (ref mut con) = *self.wifi_connection.try_borrow_mut()? {
                    con.network_state = NetworkState::Unattached;
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
