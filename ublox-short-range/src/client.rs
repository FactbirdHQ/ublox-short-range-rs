use atat::AtatClient;
use atat::atat_derive::{AtatCmd, AtatResp};
use core::cell::{Cell, RefCell};
use embedded_hal::timer::CountDown;
use embedded_nal::{SocketAddr, IpAddr};
use crate::{
    command::{
        edm::{
            EdmAtCmdWrapper,
            SwitchToEdmCommand,
            urc::EdmEvent,
        },
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
        data_mode::{ChangeMode, types::Mode},
        AT,
        Urc,
        wifi::{
            urc::{WifiLinkConnected, WifiLinkDisconnected},
            types::DisconnectReason,
        },
        ping::types::PingError,

    },
    wifi::connection::{WifiConnection, WiFiState, NetworkState},
    error::Error,
    sockets::SocketSet,
    socket::{ChannelId, SocketType, TcpSocket, UdpSocket, SocketHandle},
};
use heapless::{consts, ArrayLength, String};
    
    
#[cfg(feature = "logging")]
use defmt::info;
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

#[derive(PartialEq, Copy, Clone)]
pub enum DNSState {
    NotResolving,
    Resolving,
    Resolved(IpAddr),
    Error(PingError),
}

// macro_rules! size_of {
//     ($type:ident) => {
//         defmt::info!(
//             "Size of {}: {:?}",
//             stringify!($type),
//             core::mem::size_of::<$type>()
//         );
//     };
// }

pub struct UbloxClient<C, N, L>
where
    C: atat::AtatClient,
    N: 'static + ArrayLength<Option<crate::sockets::SocketSetItem<L>>>,
    L: 'static + ArrayLength<u8>,
{
    // pub(crate) state: Cell<State>,
    pub(crate) initialized: Cell<bool>,
    serial_mode: Cell<SerialMode>,
    pub(crate) wifi_connection: RefCell<Option<WifiConnection>>,
    pub(crate) client: RefCell<C>,
    pub(crate) sockets: RefCell<&'static mut SocketSet<N, L>>,
    pub(crate) dns_state: Cell<DNSState>,
    pub(crate) urc_attempts: Cell<u8>,
    pub(crate) max_urc_attempts: u8,
}

impl<C, N, L> UbloxClient<C, N, L>
where
    C: atat::AtatClient,
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
            dns_state: Cell::new(DNSState::NotResolving),
            max_urc_attempts: 5,
            urc_attempts: Cell::new(0),
        }
    }

    // pub fn init(&self, hostname: &str) -> Result<(), Error> {
    pub fn init(&self) -> Result<(), Error> {
        // Initilize a new ublox device to a known state (set RS232 settings, restart, wait for startup etc.)
        // size_of!(AtatCmd);
        // size_of!(AtatResp);
        // size_of!(ResponseType);
        // size_of!(Packet);

        // self.state.set(State::Initializing);

        //Switch to EDM on Init. If in EDM, fail and check with autosense
        if self.serial_mode.get() != SerialMode::ExtendedData {
            self.send_internal(&SwitchToEdmCommand, true)?;
            self.serial_mode.set(SerialMode::ExtendedData);
        }
        // self.autosense()?;
        
        //TODO: handle EDM settings quirk see EDM datasheet: 2.2.5.1 AT Request Serial settings
        self.send_internal(&EdmAtCmdWrapper(SetRS232Settings {
            baud_rate: BaudRate::B115200,
            flow_control: FlowControl::On,
            data_bits: 8,
            stop_bits: StopBits::One,
            parity: Parity::None,
            change_after_confirm: ChangeAfterConfirm::ChangeAfterOK,
        }), false)?;

        
        self.send_internal(&EdmAtCmdWrapper(StoreCurrentConfig), false)?;
        
        
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
            match self.client.try_borrow_mut()?.send(&EdmAtCmdWrapper(AT)) {
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
        // #[cfg(feature = "logging")]
        // defmt::debug!("SPIN");
        if !self.initialized.get(){
            return Err(Error::Uninitialized)
        }
        self.handle_urc()?;
        
        if let Some (ref mut con) = *self.wifi_connection.try_borrow_mut()? {
            if !con.is_connected(){
                return Err(Error::WifiState(con.wifi_state))
            }
        } else {
            return Err(Error::NoNetworkSetup)
        }

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
            SerialMode::ExtendedData => {}
        }

        if check_urc {
            if let Err(_e) = self.handle_urc() {
                #[cfg(features = "logging")]
                defmt::error!("Failed handle URC: {:?}", _e);
            }
        }

        self.client
            .try_borrow_mut()?
            .send(req)
            .map_err(|e| match e {
                nb::Error::Other(ate) => {
                    #[cfg(feature = "logging")]
                    match core::str::from_utf8(&req.as_bytes()) {
                        Ok(s) => defmt::error!("{:?}: [{:?}]", ate, s),
                        Err(_) => defmt::error!("{:?}: {:02x?}", ate, req.as_bytes()),
                    };
                    ate.into()
                }
                nb::Error::WouldBlock => Error::_Unknown,
            })
    }

    fn handle_urc(&self) -> Result<(), Error> {
        self.client.try_borrow_mut()?.peek_urc_with::<EdmEvent, _>(|edm_urc| {
            // #[cfg(feature = "logging")]
            // defmt::debug!("Handle URC");
            let res = match edm_urc {
                EdmEvent::ATEvent(urc) => {
                    match urc {
                        Urc::PeerConnected(_) => {
                            #[cfg(feature = "logging")]
                            defmt::debug!("[URC] PeerConnected");
                            true
                        }
                        Urc::PeerDisconnected(msg) => {
                            #[cfg(feature = "logging")]
                            defmt::debug!("[URC] PeerDisconnected");
                            if let Ok(ref mut sockets) = self.sockets.try_borrow_mut(){
                                let mut handle = SocketHandle(msg.handle);
                                match sockets.socket_type(handle) {
                                    Some(SocketType::Tcp) => {
                                        if let Ok(mut tcp) = sockets.get::<TcpSocket<_>>(handle){
                                            tcp.close();
                                        }
                                    }
                                    Some(SocketType::Udp) => {
                                        if let Ok(mut udp) = sockets.get::<UdpSocket<_>>(handle){
                                            udp.close();
                                        }
                                    }
                                    None => (),
                                }
                                if sockets.remove(handle).is_err(){
                                    false
                                } else {
                                    true
                                }
                            } else {
                                false
                            }
                        }
                        Urc::WifiLinkConnected(msg) => {
                            #[cfg(feature = "logging")]
                            defmt::debug!("[URC] WifiLinkConnected");
                            if let Ok(mut some) = self.wifi_connection.try_borrow_mut() {
                                if let Some(ref mut con) = *some {
                                        con.wifi_state = WiFiState::Connected;
                                        con.network.bssid = msg.bssid;
                                        con.network.channel = msg.channel;
                                }
                                true
                            } else {
                                false
                            }
                            
                        }
                        Urc::WifiLinkDisconnected(msg) => {
                            #[cfg(feature = "logging")]
                            defmt::debug!("[URC] WifiLinkDisconnected");
                            if let Ok(mut some) = self.wifi_connection.try_borrow_mut() {
                                if let Some(ref mut con) = *some {
                                    // con.sockets.prune();
                                    match msg.reason{
                                        DisconnectReason::NetworkDisabled => {
                                            con.wifi_state = WiFiState::Inactive;
                                        }
                                        DisconnectReason::SecurityProblems => {
                                            #[cfg(feature = "logging")]
                                            defmt::error!("Wifi Security Problems");
                                        }
                                        _ => {
                                            con.wifi_state = WiFiState::NotConnected;
                                        }
                                    }
                                }
                                true
                            } else {
                                false
                            }
                            
                        }
                        Urc::WifiAPUp(_) => {
                            #[cfg(feature = "logging")]
                            defmt::debug!("[URC] WifiAPUp");
                            true
                        }
                        Urc::WifiAPDown(_) => {
                            #[cfg(feature = "logging")]
                            defmt::debug!("[URC] WifiAPDown");
                            true
                        }
                        Urc::WifiAPStationConnected(_) => {
                            #[cfg(feature = "logging")]
                            defmt::debug!("[URC] WifiAPStationConnected");
                            true
                            
                        }
                        Urc::WifiAPStationDisconnected(_) => {
                            #[cfg(feature = "logging")]
                            defmt::debug!("[URC] WifiAPStationDisconnected");
                            true
                            
                        }
                        Urc::EthernetLinkUp(_) => {
                            #[cfg(feature = "logging")]
                            defmt::debug!("[URC] EthernetLinkUp");
                            true
                            
                        }
                        Urc::EthernetLinkDown(_) => {
                            #[cfg(feature = "logging")]
                            defmt::debug!("[URC] EthernetLinkDown");
                            true
                            
                        }
                        Urc::NetworkUp(_) => {
                            #[cfg(feature = "logging")]
                            defmt::debug!("[URC] NetworkUp");
                            if let Ok(mut some) = self.wifi_connection.try_borrow_mut() {
                                if let Some (ref mut con) = *some {
                                    con.network_state = NetworkState::Attached;
                                }
                                true
                            } else {
                                false
                            }
                            
                        }
                        Urc::NetworkDown(_) => {
                            #[cfg(feature = "logging")]
                            defmt::debug!("[URC] NetworkDown");
                            if let Ok(mut some) = self.wifi_connection.try_borrow_mut() {
                                if let Some(ref mut con) = *some {
                                    con.network_state = NetworkState::Attached;
                                }
                                true
                            } else {
                                false
                            }                            
                        }
                        Urc::NetworkError(_) => {
                            #[cfg(feature = "logging")]
                            defmt::debug!("[URC] NetworkError");
                            true
                        }
                        Urc::PingResponse(resp) => {
                            #[cfg(feature = "logging")]
                            defmt::debug!("[URC] PingResponse");
                            if self.dns_state.get() == DNSState::Resolving{
                                self.dns_state.set(DNSState::Resolved(resp.ip))
                            }
                            true
                        }
                        Urc::PingErrorResponse(resp) => {
                            #[cfg(feature = "logging")]
                            defmt::debug!("[URC] PingErrorResponse");
                            if self.dns_state.get() == DNSState::Resolving{
                                self.dns_state.set(DNSState::Error(resp.error))
                            }
                            true
                        }
                    }
                }, // end match urc
                EdmEvent::StartUp => {
                    #[cfg(feature = "logging")]
                    defmt::debug!("[EDM_URC] STARTUP");
                    // self.initialized.set(false);
                    // self.serial_mode.set(SerialMode::Cmd);
                    true
                    
                },
                EdmEvent::IPv4ConnectEvent(event) => {
                    #[cfg(feature = "logging")]
                    defmt::debug!("[EDM_URC] IPv4ConnectEvent! Channel_id: {:?}", event.channel_id);

                    if let Ok(mut sockets) = self.sockets.try_borrow_mut() {
                        let endpoint = SocketAddr::new(IpAddr::V4(event.remote_ip), event.remote_port);
                        match sockets.socket_type_by_endpoint(&endpoint) {
                            Some(SocketType::Tcp) => {
                                if let Ok(mut tcp) = sockets.get_by_endpoint::<TcpSocket<_>>(&endpoint){
                                    tcp.meta.channel_id.0 = event.channel_id;
                                    //maybe
                                    tcp.set_state(crate::socket::TcpState::Established);
                                    true
                                } else {
                                    false
                                }
                            },
                            Some(SocketType::Udp) => {
                                if let Ok(mut udp) = sockets.get_by_endpoint::<UdpSocket<_>>(&endpoint){
                                    udp.meta.channel_id.0 = event.channel_id;
                                    true
                                } else {
                                    false
                                }
                            },
                            None => true,
                        }
                    } else {
                        false
                    }
                }
                EdmEvent::IPv6ConnectEvent(event) => {
                    #[cfg(feature = "logging")]
                    defmt::debug!("[EDM_URC] IPv6ConnectEvent! Channel_id: {:?}", event.channel_id);
                    if let Ok(mut sockets) = self.sockets.try_borrow_mut() {
                        let endpoint = SocketAddr::new(IpAddr::V6(event.remote_ip), event.remote_port);
                        match sockets.socket_type_by_endpoint(&endpoint) {
                            Some(SocketType::Tcp) => {
                                if let Ok(mut tcp) = sockets.get_by_endpoint::<TcpSocket<_>>(&endpoint) {
                                    tcp.meta.channel_id.0 = event.channel_id;
                                    true
                                } else {
                                    false
                                }
                            }
                            Some(SocketType::Udp) => {
                                if let Ok(mut udp) = sockets.get_by_endpoint::<UdpSocket<_>>(&endpoint) {
                                    udp.meta.channel_id.0 = event.channel_id;
                                    true
                                } else {
                                    false
                                }
                            }
                            None => true,
                        }
                    } else {
                        false
                    }
                }
                EdmEvent::BluetoothConnectEvent(_) => {
                    #[cfg(feature = "logging")]
                    defmt::debug!("[EDM_URC] BluetoothConnectEvent"); 
                    true
                }
                EdmEvent::DisconnectEvent(channel_id) => {
                    #[cfg(feature = "logging")]
                    defmt::debug!("[EDM_URC] DisconnectEvent");
                    true
                }
                EdmEvent::DataEvent(mut event) => {
                    #[cfg(feature = "logging")]
                    defmt::debug!("[EDM_URC] DataEvent");
                    if let Ok(digested) = self.socket_ingress(ChannelId(event.channel_id), &event.data){
                        if digested < event.data.len() {
                            //resize packet and return false
                            event.data = heapless::Vec::from_slice(&event.data[digested .. event.data.len()]).unwrap();
                            false
                        } else {
                            true
                        }
                    } else {
                        false
                    }
                }
            };// end match emd-urc
            if !res {
                let a = self.urc_attempts.get();
                if a < self.max_urc_attempts {
                    #[cfg(feature = "logging")]
                    defmt::debug!("[EDM_URC] URC handeling failed");
                    self.urc_attempts.set(a + 1);
                    return false;
                }
                #[cfg(feature = "logging")]
                defmt::debug!("[EDM_URC] URC thrown away");
            }
            self.urc_attempts.set(0);
            true
        });
        Ok(())
    }

    /// Send AT command 
    /// Automaticaly waraps commands in EDM context
    pub fn send_at<A>(&self, cmd: A) -> Result<A::Response, Error> 
    where
        A: atat::AtatCmd,
        <A as atat::AtatCmd>::CommandLen: core::ops::Add<crate::command::edm::types::EdmAtCmdOverhead>,
        <<A as atat::AtatCmd>::CommandLen as core::ops::Add<crate::command::edm::types::EdmAtCmdOverhead>>::Output:
            atat::heapless::ArrayLength<u8>,
    {
        if !self.initialized.get() {
            self.init()?;
        }
        if self.serial_mode.get() == SerialMode::ExtendedData {
            self.send_internal(&EdmAtCmdWrapper(cmd), true)
        } else {
            self.send_internal(&cmd, true)
        }
    }
}
