use crate::{
    command::{
        edm::{urc::EdmEvent, EdmAtCmdWrapper, SwitchToEdmCommand},
        ping::types::PingError,
        system::{
            types::{BaudRate, ChangeAfterConfirm, FlowControl, Parity, StopBits},
            SetRS232Settings, StoreCurrentConfig,
        },
        wifi::types::DisconnectReason,
        Urc,
    },
    error::Error,
    socket::{ChannelId, SocketHandle, SocketType, TcpSocket, TcpState, UdpSocket, UdpState},
    sockets::SocketSet,
    wifi::connection::{NetworkState, WiFiState, WifiConnection},
};
use core::cell::{Cell, RefCell};
use embedded_nal::{IpAddr, SocketAddr};

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
pub enum SerialMode {
    Cmd,
    ExtendedData,
}

#[derive(PartialEq, Copy, Clone)]
pub enum DNSState {
    NotResolving,
    Resolving,
    Resolved(IpAddr),
    Error(PingError),
}

#[derive(PartialEq, Clone)]
pub struct SecurityCredentials {
    pub ca_cert_name: Option<heapless::String<16>>,
    pub c_cert_name: Option<heapless::String<16>>, //TODO: Make &str with lifetime
    pub c_key_name: Option<heapless::String<16>>,
}

pub struct UbloxClient<C, const N: usize, const L: usize>
where
    C: atat::AtatClient,
{
    pub(crate) initialized: Cell<bool>,
    serial_mode: Cell<SerialMode>,
    pub(crate) wifi_connection: RefCell<Option<WifiConnection>>,
    pub(crate) client: RefCell<C>,
    pub(crate) sockets: RefCell<&'static mut SocketSet<N, L>>,
    pub(crate) dns_state: Cell<DNSState>,
    pub(crate) urc_attempts: Cell<u8>,
    pub(crate) max_urc_attempts: u8,
    pub(crate) security_credentials: Option<SecurityCredentials>,
}

impl<C, const N: usize, const L: usize> UbloxClient<C, N, L>
where
    C: atat::AtatClient,
{
    pub fn new(client: C, socket_set: &'static mut SocketSet<N, L>) -> Self {
        UbloxClient {
            initialized: Cell::new(false),
            serial_mode: Cell::new(SerialMode::Cmd),
            wifi_connection: RefCell::new(None),
            client: RefCell::new(client),
            sockets: RefCell::new(socket_set),
            dns_state: Cell::new(DNSState::NotResolving),
            max_urc_attempts: 5,
            urc_attempts: Cell::new(0),
            security_credentials: None,
        }
    }

    pub fn init(&self) -> Result<(), Error> {
        // Initilize a new ublox device to a known state (set RS232 settings)

        //Switch to EDM on Init. If in EDM, fail and check with autosense
        if self.serial_mode.get() != SerialMode::ExtendedData {
            self.send_internal(&SwitchToEdmCommand, true)?;
            self.serial_mode.set(SerialMode::ExtendedData);
        }

        //TODO: handle EDM settings quirk see EDM datasheet: 2.2.5.1 AT Request Serial settings
        self.send_internal(
            &EdmAtCmdWrapper(SetRS232Settings {
                baud_rate: BaudRate::B115200,
                flow_control: FlowControl::On,
                data_bits: 8,
                stop_bits: StopBits::One,
                parity: Parity::None,
                change_after_confirm: ChangeAfterConfirm::ChangeAfterOK,
            }),
            false,
        )?;

        self.send_internal(&EdmAtCmdWrapper(StoreCurrentConfig), false)?;

        self.initialized.set(true);
        Ok(())
    }

    /// Not implemented
    // #[inline]
    // fn low_power_mode(&self, _enable: bool) -> Result<(), Error> {
    //     Err(Error::Unimplemented)
    // }

    ///Not in use
    // #[inline]
    // fn autosense(&self) -> Result<(), Error> {
    //     for _ in 0..15 {
    //         match self.client.try_borrow_mut()?.send(&EdmAtCmdWrapper(AT)) {
    //             Ok(_) => {
    //                 return Ok(());
    //             }
    //             Err(_e) => {}
    //         };
    //     }
    //     Err(Error::BaudDetection)
    // }

    ///Not implemented
    // #[inline]
    // fn reset(&self) -> Result<(), Error> {
    //     Err(Error::Unimplemented)
    // }

    pub fn spin(&self) -> Result<(), Error> {
        // defmt::debug!("SPIN");
        if !self.initialized.get() {
            return Err(Error::Uninitialized);
        }
        self.handle_urc()?;

        if let Some(ref mut con) = *self.wifi_connection.try_borrow_mut()? {
            if !con.is_connected() {
                return Err(Error::WifiState(con.wifi_state));
            }
        } else {
            return Err(Error::NoWifiSetup);
        }

        Ok(())
    }

    pub(crate) fn send_internal<A, const LEN: usize>(
        &self,
        req: &A,
        check_urc: bool,
    ) -> Result<A::Response, Error>
    where
        A: atat::AtatCmd<LEN, Error = atat::GenericError>,
    {
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
                    match core::str::from_utf8(&req.as_bytes()) {
                        Ok(s) => defmt::error!("{:?}: [{=str}]", ate, s),
                        Err(_) => defmt::error!(
                            "{:?}: {:?}",
                            ate,
                            core::convert::AsRef::<[u8]>::as_ref(&req.as_bytes())
                        ),
                    };
                    ate.into()
                }
                nb::Error::WouldBlock => Error::_Unknown,
            })
    }

    fn handle_urc(&self) -> Result<(), Error> {
        self.client
            .try_borrow_mut()?
            .peek_urc_with::<EdmEvent, _>(|edm_urc| {
                defmt::trace!("Handle URC");
                let res = match edm_urc {
                    EdmEvent::ATEvent(urc) => {
                        match urc {
                            Urc::PeerConnected(_) => {
                                defmt::debug!("[URC] PeerConnected");
                                true
                            }
                            Urc::PeerDisconnected(msg) => {
                                defmt::debug!("[URC] PeerDisconnected");
                                if let Ok(ref mut sockets) = self.sockets.try_borrow_mut() {
                                    let handle = SocketHandle(msg.handle);
                                    match sockets.socket_type(handle) {
                                        Some(SocketType::Tcp) => {
                                            if let Ok(mut tcp) = sockets.get::<TcpSocket<L>>(handle)
                                            {
                                                tcp.close();
                                                sockets.remove(handle).ok();
                                            }
                                        }
                                        Some(SocketType::Udp) => {
                                            if let Ok(mut udp) = sockets.get::<UdpSocket<L>>(handle)
                                            {
                                                udp.close();
                                            }
                                            sockets.remove(handle).ok();
                                        }
                                        None => {}
                                    }
                                    true
                                } else {
                                    false
                                }
                            }
                            Urc::WifiLinkConnected(msg) => {
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
                                defmt::debug!("[URC] WifiLinkDisconnected");
                                if let Ok(mut some) = self.wifi_connection.try_borrow_mut() {
                                    if let Some(ref mut con) = *some {
                                        // con.sockets.prune();
                                        match msg.reason {
                                            DisconnectReason::NetworkDisabled => {
                                                con.wifi_state = WiFiState::Inactive;
                                            }
                                            DisconnectReason::SecurityProblems => {
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
                                defmt::debug!("[URC] WifiAPUp");
                                true
                            }
                            Urc::WifiAPDown(_) => {
                                defmt::debug!("[URC] WifiAPDown");
                                true
                            }
                            Urc::WifiAPStationConnected(_) => {
                                defmt::debug!("[URC] WifiAPStationConnected");
                                true
                            }
                            Urc::WifiAPStationDisconnected(_) => {
                                defmt::debug!("[URC] WifiAPStationDisconnected");
                                true
                            }
                            Urc::EthernetLinkUp(_) => {
                                defmt::debug!("[URC] EthernetLinkUp");
                                true
                            }
                            Urc::EthernetLinkDown(_) => {
                                defmt::debug!("[URC] EthernetLinkDown");
                                true
                            }
                            Urc::NetworkUp(_) => {
                                defmt::debug!("[URC] NetworkUp");
                                if let Ok(mut some) = self.wifi_connection.try_borrow_mut() {
                                    if let Some(ref mut con) = *some {
                                        match con.network_state {
                                            NetworkState::Attached => (),
                                            NetworkState::AlmostAttached => {
                                                con.network_state = NetworkState::Attached
                                            }
                                            NetworkState::Unattached => {
                                                con.network_state = NetworkState::AlmostAttached
                                            }
                                        }
                                        // con.network_state = NetworkState::Attached;
                                    }
                                    true
                                } else {
                                    false
                                }
                            }
                            Urc::NetworkDown(_) => {
                                defmt::debug!("[URC] NetworkDown");
                                if let Ok(mut some) = self.wifi_connection.try_borrow_mut() {
                                    if let Some(ref mut con) = *some {
                                        con.network_state = NetworkState::Unattached;
                                    }
                                    true
                                } else {
                                    false
                                }
                            }
                            Urc::NetworkError(_) => {
                                defmt::debug!("[URC] NetworkError");
                                true
                            }
                            Urc::PingResponse(resp) => {
                                defmt::debug!("[URC] PingResponse");
                                if self.dns_state.get() == DNSState::Resolving {
                                    self.dns_state.set(DNSState::Resolved(resp.ip))
                                }
                                true
                            }
                            Urc::PingErrorResponse(resp) => {
                                defmt::debug!("[URC] PingErrorResponse: {:?}", resp.error);
                                if self.dns_state.get() == DNSState::Resolving {
                                    self.dns_state.set(DNSState::Error(resp.error))
                                }
                                true
                            }
                        }
                    } // end match urc
                    EdmEvent::StartUp => {
                        defmt::debug!("[EDM_URC] STARTUP");
                        true
                    }
                    EdmEvent::IPv4ConnectEvent(event) => {
                        defmt::debug!(
                            "[EDM_URC] IPv4ConnectEvent! Channel_id: {:?}",
                            event.channel_id
                        );

                        if let Ok(mut sockets) = self.sockets.try_borrow_mut() {
                            let endpoint =
                                SocketAddr::new(IpAddr::V4(event.remote_ip), event.remote_port);
                            match sockets.socket_type_by_endpoint(&endpoint) {
                                Some(SocketType::Tcp) => {
                                    if let Ok(mut tcp) =
                                        sockets.get_by_endpoint::<TcpSocket<L>>(&endpoint)
                                    {
                                        tcp.meta.channel_id.0 = event.channel_id;
                                        tcp.set_state(TcpState::Established);
                                        true
                                    } else {
                                        defmt::debug!("[EDM_URC] Socket not found!");
                                        false
                                    }
                                }
                                Some(SocketType::Udp) => {
                                    if let Ok(mut udp) =
                                        sockets.get_by_endpoint::<UdpSocket<L>>(&endpoint)
                                    {
                                        udp.meta.channel_id.0 = event.channel_id;
                                        udp.set_state(UdpState::Established);
                                        true
                                    } else {
                                        defmt::debug!("[EDM_URC] Socket not found!");
                                        false
                                    }
                                }
                                None => {
                                    defmt::debug!("[EDM_URC] Socket type not excisting!");
                                    true
                                }
                            }
                        } else {
                            defmt::debug!("[EDM_URC] Could not borrow sockets!");
                            false
                        }
                    }
                    EdmEvent::IPv6ConnectEvent(event) => {
                        defmt::debug!(
                            "[EDM_URC] IPv6ConnectEvent! Channel_id: {:?}",
                            event.channel_id
                        );
                        if let Ok(mut sockets) = self.sockets.try_borrow_mut() {
                            let endpoint =
                                SocketAddr::new(IpAddr::V6(event.remote_ip), event.remote_port);
                            match sockets.socket_type_by_endpoint(&endpoint) {
                                Some(SocketType::Tcp) => {
                                    if let Ok(mut tcp) =
                                        sockets.get_by_endpoint::<TcpSocket<L>>(&endpoint)
                                    {
                                        tcp.meta.channel_id.0 = event.channel_id;
                                        tcp.set_state(TcpState::Established);
                                        true
                                    } else {
                                        false
                                    }
                                }
                                Some(SocketType::Udp) => {
                                    if let Ok(mut udp) =
                                        sockets.get_by_endpoint::<UdpSocket<L>>(&endpoint)
                                    {
                                        udp.meta.channel_id.0 = event.channel_id;
                                        udp.set_state(UdpState::Established);
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
                        defmt::debug!("[EDM_URC] BluetoothConnectEvent");
                        true
                    }
                    EdmEvent::DisconnectEvent(channel_id) => {
                        defmt::debug!("[EDM_URC] DisconnectEvent! Channel_id: {:?}", channel_id);
                        true
                    }
                    EdmEvent::DataEvent(mut event) => {
                        defmt::debug!("[EDM_URC] DataEvent! Channel_id: {:?}", event.channel_id);
                        if let Ok(digested) =
                            self.socket_ingress(ChannelId(event.channel_id), &event.data)
                        {
                            if digested < event.data.len() {
                                //resize packet and return false
                                event.data = heapless::Vec::from_slice(
                                    &event.data[digested..event.data.len()],
                                )
                                .unwrap();
                                false
                            } else {
                                true
                            }
                        } else {
                            false
                        }
                    }
                }; // end match emd-urc
                if !res {
                    let a = self.urc_attempts.get();
                    if a < self.max_urc_attempts {
                        defmt::error!("[EDM_URC] URC handeling failed");
                        self.urc_attempts.set(a + 1);
                        return false;
                    }
                    defmt::error!("[EDM_URC] URC thrown away");
                }
                self.urc_attempts.set(0);
                true
            });
        Ok(())
    }

    /// Send AT command
    /// Automaticaly waraps commands in EDM context
    pub fn send_at<A, const LEN: usize>(&self, cmd: A) -> Result<A::Response, Error>
    where
        A: atat::AtatCmd<LEN, Error = atat::GenericError>,
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
