use crate::{
    command::{
        data_mode::types::IPProtocol,
        edm::{types::Protocol, urc::EdmEvent, EdmAtCmdWrapper, SwitchToEdmCommand},
        network::SetNetworkHostName,
        ping::types::PingError,
        system::{
            types::{BaudRate, ChangeAfterConfirm, FlowControl, Parity, StopBits},
            SetRS232Settings, StoreCurrentConfig,
        },
        wifi::types::DisconnectReason,
        Urc,
    },
    config::Config,
    error::Error,
    wifi::{
        connection::{NetworkState, WiFiState, WifiConnection},
        EdmMap,
    },
};
use core::convert::TryInto;
use core::str::FromStr;
use embedded_hal::digital::OutputPin;
use embedded_nal::{nb, IpAddr, Ipv4Addr, SocketAddr};
use embedded_time::duration::{Generic, Milliseconds};
use embedded_time::Clock;
use ublox_sockets::{
    tcp_listener::TcpListener, udp_listener::UdpListener, AnySocket, SocketSet, SocketType,
    TcpSocket, TcpState, UdpSocket, UdpState,
};

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

#[derive(PartialEq, Clone, Default)]
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
    pub(crate) config: Config<RST>,
    pub(crate) sockets: Option<&'static mut SocketSet<CLK, N, L>>,
    pub(crate) dns_state: DNSState,
    pub(crate) urc_attempts: u8,
    pub(crate) max_urc_attempts: u8,
    pub(crate) security_credentials: SecurityCredentials,
    pub(crate) timer: CLK,
    pub(crate) edm_mapping: EdmMap,
    pub(crate) tcp_listener: TcpListener<3, N>,
    pub(crate) udp_listener: UdpListener<3, N>,
}

impl<C, CLK, RST, const N: usize, const L: usize> UbloxClient<C, CLK, RST, N, L>
where
    C: atat::AtatClient,
    CLK: 'static + Clock,
    RST: OutputPin,
    Generic<CLK::T>: TryInto<Milliseconds>,
{
    pub fn new(client: C, timer: CLK, config: Config<RST>) -> Self {
        UbloxClient {
            initialized: false,
            serial_mode: SerialMode::Cmd,
            wifi_connection: None,
            client,
            config,
            sockets: None,
            dns_state: DNSState::NotResolving,
            max_urc_attempts: 5,
            urc_attempts: 0,
            security_credentials: SecurityCredentials::default(),
            timer,
            edm_mapping: EdmMap::new(),
            tcp_listener: TcpListener::new(),
            udp_listener: UdpListener::new(),
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

        if let Some(hostname) = self.config.hostname.clone() {
            self.send_internal(
                &EdmAtCmdWrapper(SetNetworkHostName {
                    host_name: hostname.as_str(),
                }),
                false,
            )?;
        }

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

    pub fn reset(&mut self) -> Result<(), Error> {
        self.serial_mode = SerialMode::Cmd;
        self.initialized = false;

        if let Some(ref mut pin) = self.config.rst_pin {
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
                defmt::error!("{:?}: {=[u8]:a}", ate, req.as_bytes());
                ate.into()
            }
            nb::Error::WouldBlock => Error::_Unknown,
        })
    }

    fn handle_urc(&mut self) -> Result<(), Error> {
        if let Some(ref mut sockets) = self.sockets.as_deref_mut() {
            let dns_state = &mut self.dns_state;
            let tcp_listener = &mut self.tcp_listener;
            let udp_listener = &mut self.udp_listener;
            let edm_mapping = &mut self.edm_mapping;
            let wifi_connection = self.wifi_connection.as_mut();
            let ts = self.timer.try_now().map_err(|_| Error::Timer)?;

            let mut a = self.urc_attempts;
            let max = self.max_urc_attempts;

            self.client.peek_urc_with::<EdmEvent, _>(|edm_urc| {
                let res = match edm_urc {
                    EdmEvent::ATEvent(urc) => {
                        match urc {
                            Urc::PeerConnected(event) => {
                                defmt::trace!(
                                    "[URC] PeerConnected {:?}",
                                    defmt::Debug2Format(&event)
                                );

                                let remote_ip = Ipv4Addr::from_str(
                                    core::str::from_utf8(event.remote_address.as_slice()).unwrap(),
                                )
                                .unwrap();

                                let remote = SocketAddr::new(remote_ip.into(), event.remote_port);

                                if let Some(queue) = tcp_listener.incoming(event.local_port) {
                                    queue.enqueue((event.handle, remote)).unwrap();
                                    return true;
                                } else if let Some(queue) = udp_listener.incoming(event.local_port)
                                {
                                    queue.enqueue((event.handle, remote)).unwrap();
                                    return true;
                                } else {
                                    match event.protocol {
                                        IPProtocol::TCP => {
                                            if let Ok(mut tcp) =
                                                sockets.get::<TcpSocket<CLK, L>>(event.handle)
                                            {
                                                defmt::debug!(
                                                    "Binding remote {=[u8]:a} to TCP socket {:?}",
                                                    event.remote_address.as_slice(),
                                                    event.handle
                                                );
                                                tcp.set_state(TcpState::Connected(remote));
                                                return true;
                                            }
                                        }
                                        IPProtocol::UDP => {
                                            if let Ok(mut udp) =
                                                sockets.get::<UdpSocket<CLK, L>>(event.handle)
                                            {
                                                defmt::debug!(
                                                    "Binding remote {=[u8]:a} to UDP socket {:?}",
                                                    event.remote_address.as_slice(),
                                                    event.handle
                                                );
                                                udp.bind(remote).unwrap();
                                                udp.set_state(UdpState::Established);
                                                return true;
                                            }
                                        }
                                    }
                                }

                                false
                            }
                            Urc::PeerDisconnected(msg) => {
                                defmt::trace!("[URC] PeerDisconnected");
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
                                        // FIXME: Is this correct?
                                        sockets.remove(msg.handle).ok();
                                    }
                                    _ => {}
                                }
                                true
                            }
                            Urc::WifiLinkConnected(msg) => {
                                defmt::trace!("[URC] WifiLinkConnected");
                                if let Some(con) = wifi_connection {
                                    con.wifi_state = WiFiState::Connected;
                                    con.network.bssid = msg.bssid;
                                    con.network.channel = msg.channel;
                                }
                                true
                            }
                            Urc::WifiLinkDisconnected(msg) => {
                                defmt::trace!("[URC] WifiLinkDisconnected");
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
                                defmt::trace!("[URC] WifiAPUp");
                                if let Some(con) = wifi_connection {
                                    con.wifi_state = WiFiState::Connected;
                                }
                                true
                            }
                            Urc::WifiAPDown(_) => {
                                defmt::trace!("[URC] WifiAPDown");
                                if let Some(con) = wifi_connection {
                                    con.wifi_state = WiFiState::NotConnected;
                                }
                                true
                            }
                            Urc::WifiAPStationConnected(client) => {
                                defmt::trace!(
                                    "[URC] WifiAPStationConnected {=[u8]:a}",
                                    client.mac_addr.into_inner()
                                );
                                true
                            }
                            Urc::WifiAPStationDisconnected(_) => {
                                defmt::trace!("[URC] WifiAPStationDisconnected");
                                true
                            }
                            Urc::EthernetLinkUp(_) => {
                                defmt::trace!("[URC] EthernetLinkUp");
                                true
                            }
                            Urc::EthernetLinkDown(_) => {
                                defmt::trace!("[URC] EthernetLinkDown");
                                true
                            }
                            Urc::NetworkUp(_) => {
                                defmt::trace!("[URC] NetworkUp");
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
                                }
                                true
                            }
                            Urc::NetworkDown(_) => {
                                defmt::trace!("[URC] NetworkDown");
                                if let Some(con) = wifi_connection {
                                    con.network_state = NetworkState::Unattached;
                                }
                                true
                            }
                            Urc::NetworkError(_) => {
                                defmt::trace!("[URC] NetworkError");
                                true
                            }
                            Urc::PingResponse(resp) => {
                                defmt::trace!("[URC] PingResponse");
                                if *dns_state == DNSState::Resolving {
                                    *dns_state = DNSState::Resolved(resp.ip)
                                }
                                true
                            }
                            Urc::PingErrorResponse(resp) => {
                                defmt::trace!("[URC] PingErrorResponse: {:?}", resp.error);
                                if *dns_state == DNSState::Resolving {
                                    *dns_state = DNSState::Error(resp.error)
                                }
                                true
                            }
                        }
                    } // end match urc
                    EdmEvent::StartUp => {
                        defmt::trace!("[EDM_URC] STARTUP");
                        true
                    }
                    EdmEvent::IPv4ConnectEvent(event) => {
                        defmt::trace!(
                            "[EDM_URC] IPv4ConnectEvent! Channel_id: {:?}",
                            defmt::Debug2Format(&event)
                        );

                        let endpoint = SocketAddr::new(event.remote_ip.into(), event.remote_port);

                        // Peek the queue of unaccepted requests, to attempt
                        // binding an EDM channel to a socket handle
                        if let Some(queue) = tcp_listener.incoming(event.local_port) {
                            match queue.peek() {
                                Some((h, remote)) if remote == &endpoint => {
                                    return edm_mapping.insert(event.channel_id, *h).is_ok();
                                }
                                _ => {}
                            }
                        } else if let Some(queue) = udp_listener.incoming(event.local_port) {
                            match queue.peek() {
                                Some((h, remote)) if remote == &endpoint => {
                                    return edm_mapping.insert(event.channel_id, *h).is_ok();
                                }
                                _ => {}
                            }
                        }

                        sockets
                            .iter_mut()
                            .find_map(|(h, s)| {
                                match event.protocol {
                                    Protocol::TCP => {
                                        let tcp = TcpSocket::downcast(s).ok()?;
                                        if tcp.endpoint() == Some(endpoint) {
                                            defmt::debug!(
                                                "Binding EDM channel {} to TCP socket {}",
                                                event.channel_id,
                                                h
                                            );
                                            edm_mapping.insert(event.channel_id, h).unwrap();
                                            return Some(());
                                        }
                                    }
                                    Protocol::UDP => {
                                        let udp = UdpSocket::downcast(s).ok()?;
                                        if udp.endpoint() == Some(endpoint) {
                                            defmt::debug!(
                                                "Binding EDM channel {} to UDP socket {}",
                                                event.channel_id,
                                                h
                                            );
                                            edm_mapping.insert(event.channel_id, h).unwrap();
                                            return Some(());
                                        }
                                    }
                                    _ => {}
                                }
                                None
                            })
                            .is_some()
                    }
                    EdmEvent::IPv6ConnectEvent(event) => {
                        defmt::trace!(
                            "[EDM_URC] IPv6ConnectEvent! Channel_id: {:?}",
                            defmt::Debug2Format(&event)
                        );

                        let endpoint = SocketAddr::new(event.remote_ip.into(), event.remote_port);

                        // Peek the queue of unaccepted requests, to attempt
                        // binding an EDM channel to a socket handle
                        if let Some(queue) = tcp_listener.incoming(event.local_port) {
                            match queue.peek() {
                                Some((h, remote)) if remote == &endpoint => {
                                    return edm_mapping.insert(event.channel_id, *h).is_ok();
                                }
                                _ => {}
                            }
                        } else if let Some(queue) = udp_listener.incoming(event.local_port) {
                            match queue.peek() {
                                Some((h, remote)) if remote == &endpoint => {
                                    return edm_mapping.insert(event.channel_id, *h).is_ok();
                                }
                                _ => {}
                            }
                        }

                        sockets
                            .iter_mut()
                            .find_map(|(h, s)| {
                                match event.protocol {
                                    Protocol::TCP => {
                                        let tcp = TcpSocket::downcast(s).ok()?;
                                        if tcp.endpoint() == Some(endpoint) {
                                            defmt::debug!(
                                                "Binding EDM channel {} to TCP socket {}",
                                                event.channel_id,
                                                h
                                            );
                                            edm_mapping.insert(event.channel_id, h).unwrap();
                                            return Some(());
                                        }
                                    }
                                    Protocol::UDP => {
                                        let udp = UdpSocket::downcast(s).ok()?;
                                        if udp.endpoint() == Some(endpoint) {
                                            defmt::debug!(
                                                "Binding EDM channel {} to UDP socket {}",
                                                event.channel_id,
                                                h
                                            );
                                            edm_mapping.insert(event.channel_id, h).unwrap();
                                            return Some(());
                                        }
                                    }
                                    _ => {}
                                }
                                None
                            })
                            .is_some()
                    }
                    EdmEvent::BluetoothConnectEvent(_) => {
                        defmt::trace!("[EDM_URC] BluetoothConnectEvent");
                        true
                    }
                    EdmEvent::DisconnectEvent(channel_id) => {
                        defmt::trace!("[EDM_URC] DisconnectEvent! Channel_id: {:?}", channel_id);
                        edm_mapping.remove(&channel_id).unwrap();
                        true
                    }
                    EdmEvent::DataEvent(event) => {
                        defmt::trace!("[EDM_URC] DataEvent! Channel_id: {:?}", event.channel_id);
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
                                        tcp.can_recv()
                                            .then(|| tcp.rx_enqueue_slice(&event.data))
                                            .is_some()
                                    }
                                    Some(SocketType::Udp) => {
                                        // Handle udp socket
                                        let mut udp = sockets
                                            .get::<UdpSocket<CLK, L>>(*socket_handle)
                                            .unwrap();

                                        udp.can_recv()
                                            .then(|| udp.rx_enqueue_slice(&event.data))
                                            .is_some()
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
