use crate::{
    command::{
        edm::{types::Protocol, urc::EdmEvent, EdmAtCmdWrapper, SwitchToEdmCommand},
        ping::types::PingError,
        system::{
            types::{BaudRate, ChangeAfterConfirm, FlowControl, Parity, StopBits},
            SetRS232Settings, StoreCurrentConfig,
        },
        wifi::types::DisconnectReason,
        Urc,
    },
    error::Error,
    wifi::{
        connection::{NetworkState, WiFiState, WifiConnection},
        EdmMap,
    },
};
use core::convert::TryInto;
use embedded_hal::digital::OutputPin;
use embedded_nal::{nb, IpAddr, SocketAddr};
use embedded_time::duration::{Generic, Milliseconds};
use embedded_time::Clock;
use ublox_sockets::{AnySocket, SocketSet, SocketType, TcpSocket, TcpState, UdpSocket, UdpState};

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
    pub c_cert_name: Option<heapless::String<16>>, // TODO: Make &str with lifetime
    pub c_key_name: Option<heapless::String<16>>,
}

pub struct UbloxClient<C, CLK, RST, const N: usize, const L: usize>
where
    C: atat::AtatClient,
    CLK: 'static + Clock,
    RST: OutputPin,
{
    pub(crate) initialized: bool,
    serial_mode: SerialMode,
    pub(crate) wifi_connection: Option<WifiConnection>,
    pub(crate) client: C,
    pub(crate) sockets: Option<&'static mut SocketSet<CLK, N, L>>,
    pub(crate) dns_state: DNSState,
    pub(crate) urc_attempts: u8,
    pub(crate) max_urc_attempts: u8,
    pub(crate) security_credentials: Option<SecurityCredentials>,
    pub(crate) timer: CLK,
    pub(crate) reset_pin: Option<RST>,
    pub(crate) edm_mapping: EdmMap,
}

impl<C, CLK, RST, const N: usize, const L: usize> UbloxClient<C, CLK, RST, N, L>
where
    C: atat::AtatClient,
    CLK: 'static + Clock,
    RST: OutputPin,
    Generic<CLK::T>: TryInto<Milliseconds>,
{
    pub fn new(client: C, timer: CLK, reset_pin: Option<RST>) -> Self {
        UbloxClient {
            initialized: false,
            serial_mode: SerialMode::Cmd,
            wifi_connection: None,
            client,
            sockets: None,
            dns_state: DNSState::NotResolving,
            max_urc_attempts: 5,
            urc_attempts: 0,
            security_credentials: None,
            timer,
            reset_pin,
            edm_mapping: EdmMap::new(),
        }
    }

    pub fn set_socket_storage(&mut self, socket_set: &'static mut SocketSet<CLK, N, L>) {
        socket_set.prune();
        self.sockets.replace(socket_set);
    }

    pub fn take_socket_storage(&mut self) -> Option<&'static mut SocketSet<CLK, N, L>> {
        self.sockets.take()
    }

    pub fn init(&mut self) -> Result<(), Error> {
        // Initilize a new ublox device to a known state (set RS232 settings)

        defmt::debug!("Initializing wifi");
        // Hard reset module
        self.reset()?;

        // Switch to EDM on Init. If in EDM, fail and check with autosense
        if self.serial_mode != SerialMode::ExtendedData {
            // self.send_internal(&SwitchToEdmCommand, true)?;
            self.retry_send(&SwitchToEdmCommand, 5)?;
            self.serial_mode = SerialMode::ExtendedData;
        }

        // TODO: handle EDM settings quirk see EDM datasheet: 2.2.5.1 AT Request Serial settings
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

        self.initialized = true;
        Ok(())
    }

    pub fn retry_send<A, const LEN: usize>(
        &mut self,
        cmd: &A,
        attempts: usize,
    ) -> Result<A::Response, Error>
    where
        A: atat::AtatCmd<LEN, Error = atat::GenericError>,
    {
        for _ in 0..attempts {
            match self.send_internal(cmd, true) {
                Ok(resp) => {
                    return Ok(resp);
                }
                Err(_e) => {}
            };
        }
        Err(Error::BaudDetection)
    }

    fn reset(&mut self) -> Result<(), Error> {
        self.serial_mode = SerialMode::Cmd;
        self.initialized = false;

        if let Some(ref mut pin) = self.reset_pin {
            pin.try_set_low().ok();
            self.timer
                .new_timer(Milliseconds(50))
                .start()
                .map_err(|_| Error::Timer)?
                .wait()
                .map_err(|_| Error::Timer)?;
            pin.try_set_high().ok();
            self.timer
                .new_timer(Milliseconds(3000))
                .start()
                .map_err(|_| Error::Timer)?
                .wait()
                .map_err(|_| Error::Timer)?;
        }
        Ok(())
    }

    pub fn spin(&mut self) -> Result<(), Error> {
        if !self.initialized {
            return Err(Error::Uninitialized);
        }
        self.handle_urc()?;

        if let Some(ref mut con) = self.wifi_connection {
            if !con.is_connected() {
                return Err(Error::WifiState(con.wifi_state));
            }
        } else {
            return Err(Error::NoWifiSetup);
        }

        Ok(())
    }

    pub(crate) fn send_internal<A, const LEN: usize>(
        &mut self,
        req: &A,
        check_urc: bool,
    ) -> Result<A::Response, Error>
    where
        A: atat::AtatCmd<LEN, Error = atat::GenericError>,
    {
        if check_urc {
            if let Err(e) = self.handle_urc() {
                defmt::error!("Failed handle URC: {:?}", e);
            }
        }

        self.client.send(req).map_err(|e| match e {
            nb::Error::Other(ate) => {
                defmt::error!("{:?}: [{=[u8]:a}]", ate, req.as_bytes());
                ate.into()
            }
            nb::Error::WouldBlock => Error::_Unknown,
        })
    }

    fn handle_urc(&mut self) -> Result<(), Error> {
        if let Some(ref mut sockets) = self.sockets.as_deref_mut() {
            let dns_state = &mut self.dns_state;
            let edm_mapping = &mut self.edm_mapping;
            let wifi_connection = self.wifi_connection.as_mut();
            let ts = self.timer.try_now().map_err(|_| Error::Timer)?;

            let mut a = self.urc_attempts;
            let max = self.max_urc_attempts;

            self.client.peek_urc_with::<EdmEvent, _>(|edm_urc| {
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
                                match sockets.socket_type(msg.handle) {
                                    Some(SocketType::Tcp) => {
                                        if let Ok(mut tcp) =
                                            sockets.get::<TcpSocket<CLK, L>>(msg.handle)
                                        {
                                            tcp.closed_by_remote(ts);
                                        }
                                    }
                                    Some(SocketType::Udp) => {
                                        if let Ok(mut udp) =
                                            sockets.get::<UdpSocket<CLK, L>>(msg.handle)
                                        {
                                            udp.close();
                                        }
                                        sockets.remove(msg.handle).ok();
                                    }
                                    _ => {}
                                }
                                true
                            }
                            Urc::WifiLinkConnected(msg) => {
                                defmt::debug!("[URC] WifiLinkConnected");
                                if let Some(con) = wifi_connection {
                                    con.wifi_state = WiFiState::Connected;
                                    con.network.bssid = msg.bssid;
                                    con.network.channel = msg.channel;
                                }
                                true
                            }
                            Urc::WifiLinkDisconnected(msg) => {
                                defmt::debug!("[URC] WifiLinkDisconnected");
                                if let Some(con) = wifi_connection {
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
                                if let Some(con) = wifi_connection {
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
                            }
                            Urc::NetworkDown(_) => {
                                defmt::debug!("[URC] NetworkDown");
                                if let Some(con) = wifi_connection {
                                    con.network_state = NetworkState::Unattached;
                                }
                                true
                            }
                            Urc::NetworkError(_) => {
                                defmt::debug!("[URC] NetworkError");
                                true
                            }
                            Urc::PingResponse(resp) => {
                                defmt::debug!("[URC] PingResponse");
                                if *dns_state == DNSState::Resolving {
                                    *dns_state = DNSState::Resolved(resp.ip)
                                }
                                true
                            }
                            Urc::PingErrorResponse(resp) => {
                                defmt::debug!("[URC] PingErrorResponse: {:?}", resp.error);
                                if *dns_state == DNSState::Resolving {
                                    *dns_state = DNSState::Error(resp.error)
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

                        let endpoint = SocketAddr::new(event.remote_ip.into(), event.remote_port);

                        sockets
                            .iter_mut()
                            .find_map(|(h, s)| {
                                match event.protocol {
                                    Protocol::TCP => {
                                        let mut tcp = TcpSocket::downcast(s).ok()?;
                                        if tcp.endpoint() == Some(endpoint) {
                                            edm_mapping.insert(event.channel_id, h).unwrap();
                                            tcp.set_state(TcpState::Connected(endpoint));
                                            return Some(true);
                                        }
                                    }
                                    Protocol::UDP => {
                                        let mut udp = UdpSocket::downcast(s).ok()?;
                                        if udp.endpoint() == Some(endpoint) {
                                            edm_mapping.insert(event.channel_id, h).unwrap();
                                            udp.set_state(UdpState::Established);
                                            return Some(true);
                                        }
                                    }
                                    _ => {}
                                }
                                None
                            })
                            .is_some()
                    }
                    EdmEvent::IPv6ConnectEvent(event) => {
                        defmt::debug!(
                            "[EDM_URC] IPv6ConnectEvent! Channel_id: {:?}",
                            event.channel_id
                        );

                        let endpoint = SocketAddr::new(event.remote_ip.into(), event.remote_port);

                        sockets
                            .iter_mut()
                            .find_map(|(h, s)| {
                                match event.protocol {
                                    Protocol::TCP => {
                                        let mut tcp = TcpSocket::downcast(s).ok()?;
                                        if tcp.endpoint() == Some(endpoint) {
                                            edm_mapping.insert(event.channel_id, h).unwrap();
                                            tcp.set_state(TcpState::Connected(endpoint));
                                            return Some(true);
                                        }
                                    }
                                    Protocol::UDP => {
                                        let mut udp = UdpSocket::downcast(s).ok()?;
                                        if udp.endpoint() == Some(endpoint) {
                                            edm_mapping.insert(event.channel_id, h).unwrap();
                                            udp.set_state(UdpState::Established);
                                            return Some(true);
                                        }
                                    }
                                    _ => {}
                                }
                                None
                            })
                            .is_some()
                    }
                    EdmEvent::BluetoothConnectEvent(_) => {
                        defmt::debug!("[EDM_URC] BluetoothConnectEvent");
                        true
                    }
                    EdmEvent::DisconnectEvent(channel_id) => {
                        defmt::debug!("[EDM_URC] DisconnectEvent! Channel_id: {:?}", channel_id);
                        edm_mapping.remove(&channel_id).unwrap();
                        true
                    }
                    EdmEvent::DataEvent(event) => {
                        defmt::debug!("[EDM_URC] DataEvent! Channel_id: {:?}", event.channel_id);
                        if !event.data.is_empty() {
                            if let Some(socket_handle) =
                                edm_mapping.socket_handle(&event.channel_id)
                            {
                                match sockets.socket_type(*socket_handle) {
                                    Some(SocketType::Tcp) => {
                                        // Handle tcp socket
                                        let mut tcp = sockets
                                            .get::<TcpSocket<CLK, L>>(*socket_handle)
                                            .unwrap();
                                        if tcp.can_recv() {
                                            tcp.rx_enqueue_slice(&event.data);
                                            true
                                        } else {
                                            false
                                        }
                                    }
                                    Some(SocketType::Udp) => {
                                        // Handle udp socket
                                        let mut udp = sockets
                                            .get::<UdpSocket<CLK, L>>(*socket_handle)
                                            .unwrap();

                                        if udp.can_recv() {
                                            udp.rx_enqueue_slice(&event.data);
                                            true
                                        } else {
                                            false
                                        }
                                    }
                                    _ => {
                                        defmt::error!("SocketNotFound {:?}", socket_handle);
                                        false
                                    }
                                }
                            } else {
                                false
                            }
                        } else {
                            false
                        }

                        // FIXME: ? This never worked as intended?!
                        // if let Ok(digested) =
                        //     self.socket_ingress(ChannelId(event.channel_id), &event.data)
                        // {
                        //     if digested < event.data.len() {
                        //         // resize packet and return false
                        //         event.data =
                        //             heapless::Vec::from_slice(&event.data[digested..event.data.len()])
                        //                 .unwrap();
                        //         false
                        //     } else {
                        //         true
                        //     }
                        // } else {
                        //     false
                        // }
                    }
                }; // end match edm-urc
                if !res {
                    if a < max {
                        defmt::error!("[EDM_URC] URC handeling failed");
                        a += 1;
                        return false;
                    }
                    defmt::error!("[EDM_URC] URC thrown away");
                }
                a = 0;
                true
            });
            self.urc_attempts = a;
        }
        Ok(())
    }

    /// Send AT command
    /// Automaticaly waraps commands in EDM context
    pub fn send_at<A, const LEN: usize>(&mut self, cmd: A) -> Result<A::Response, Error>
    where
        A: atat::AtatCmd<LEN, Error = atat::GenericError>,
    {
        if !self.initialized {
            self.init()?;
        }
        match self.serial_mode {
            SerialMode::ExtendedData => self.send_internal(&EdmAtCmdWrapper(cmd), true),
            SerialMode::Cmd => self.send_internal(&cmd, true),
        }
    }
}
