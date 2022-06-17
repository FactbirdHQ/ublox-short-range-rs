use core::str::FromStr;

use crate::{
    command::{
        data_mode::{types::{IPProtocol, PeerConfigParameter}, SetPeerConfiguration},
        edm::{types::Protocol, urc::EdmEvent, EdmAtCmdWrapper, SwitchToEdmCommand},
        network::SetNetworkHostName,
        ping::types::PingError,
        system::{
            types::{BaudRate, ChangeAfterConfirm, FlowControl, Parity, StopBits},
            SetRS232Settings, StoreCurrentConfig,
        },
        wifi::types::DisconnectReason,
        OnOff, Urc,
    },
    config::Config,
    error::Error,
    wifi::{
        connection::{NetworkState, WiFiState, WifiConnection},
        SocketMap,
    },
};
use atat::clock::Clock;
use defmt::{debug, error, trace};
use embedded_hal::digital::blocking::OutputPin;
use embedded_nal::{nb, IpAddr, Ipv4Addr, SocketAddr};
use fugit::ExtU32;
use ublox_sockets::{
    udp_listener::UdpListener, AnySocket, SocketHandle, SocketSet, SocketType, TcpSocket, TcpState,
    UdpSocket, UdpState,
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

/// Creates new socket numbers
/// Properly not Async safe
pub fn new_socket_num<'a, const TIMER_HZ: u32, const N: usize, const L: usize>(
    sockets: &'a SocketSet<TIMER_HZ, N, L>,
) -> Result<u8, ()> {
    let mut num = 0;
    while sockets.socket_type(SocketHandle(num)).is_some() {
        num += 1;
        if num == u8::MAX {
            return Err(());
        }
    }
    Ok(num)
}

pub struct UbloxClient<C, CLK, RST, const TIMER_HZ: u32, const N: usize, const L: usize>
where
    C: atat::AtatClient,
    CLK: Clock<TIMER_HZ>,
    RST: OutputPin,
{
    pub(crate) initialized: bool,
    serial_mode: SerialMode,
    pub(crate) wifi_connection: Option<WifiConnection>,
    pub(crate) client: C,
    pub(crate) config: Config<RST>,
    pub(crate) sockets: Option<&'static mut SocketSet<TIMER_HZ, N, L>>,
    pub(crate) dns_state: DNSState,
    pub(crate) urc_attempts: u8,
    pub(crate) max_urc_attempts: u8,
    pub(crate) security_credentials: SecurityCredentials,
    pub(crate) timer: CLK,
    pub(crate) socket_map: SocketMap,
    pub(crate) udp_listener: UdpListener<4, N>,
}

impl<C, CLK, RST, const TIMER_HZ: u32, const N: usize, const L: usize>
    UbloxClient<C, CLK, RST, TIMER_HZ, N, L>
where
    C: atat::AtatClient,
    CLK: Clock<TIMER_HZ>,
    RST: OutputPin,
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
            socket_map: SocketMap::default(),
            udp_listener: UdpListener::new(),
        }
    }

    pub fn set_socket_storage(&mut self, socket_set: &'static mut SocketSet<TIMER_HZ, N, L>) {
        socket_set.prune();
        self.sockets.replace(socket_set);
    }

    pub fn take_socket_storage(&mut self) -> Option<&'static mut SocketSet<TIMER_HZ, N, L>> {
        self.sockets.take()
    }

    pub fn init(&mut self) -> Result<(), Error> {
        // Initilize a new ublox device to a known state (set RS232 settings)

        debug!("Initializing wifi");
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
        A: atat::AtatCmd<LEN>,
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
        self.wifi_connection = None;
        self.urc_attempts = 0;
        self.security_credentials = SecurityCredentials::default();
        self.socket_map = SocketMap::default();

        self.clear_buffers()?;

        if let Some(ref mut pin) = self.config.rst_pin {
            pin.set_low().ok();
            self.timer.start(50.millis()).map_err(|_| Error::Timer)?;
            nb::block!(self.timer.wait()).map_err(|_| Error::Timer)?;

            pin.set_high().ok();

            self.timer.start(3.secs()).map_err(|_| Error::Timer)?;
            nb::block!(self.timer.wait()).map_err(|_| Error::Timer)?;
        }
        Ok(())
    }

    pub(crate) fn clear_buffers(&mut self) -> Result<(), Error> {
        self.client.reset();
        if let Some(ref mut sockets) = self.sockets.as_deref_mut() {
            sockets.prune();
        }

        // Allow ATAT some time to clear the buffers
        self.timer.start(300.millis()).map_err(|_| Error::Timer)?;
        nb::block!(self.timer.wait()).map_err(|_| Error::Timer)?;

        Ok(())
    }

    pub fn spin(&mut self) -> Result<(), Error> {
        if !self.initialized {
            return Err(Error::Uninitialized);
        }

        while self.handle_urc()? {}

        self.connected_to_network()?;

        // TODO: Is this smart?
        // if let Some(ref mut sockets) = self.sockets.as_deref_mut() {
            // sockets.recycle(self.timer.now());
        // }

        Ok(())
    }

    pub(crate) fn send_internal<A, const LEN: usize>(
        &mut self,
        req: &A,
        check_urc: bool,
    ) -> Result<A::Response, Error>
    where
        A: atat::AtatCmd<LEN>,
    {
        if check_urc {
            if let Err(e) = self.handle_urc() {
                error!("Failed handle URC: {:?}", e);
            }
        }

        self.client.send(req).map_err(|e| match e {
            nb::Error::Other(ate) => {
                error!("{:?}: {=[u8]:a}", ate, req.as_bytes());
                ate.into()
            }
            nb::Error::WouldBlock => Error::_Unknown,
        })
    }

    fn handle_urc(&mut self) -> Result<bool, Error> {
        let mut ran = false;
        if let Some(ref mut sockets) = self.sockets.as_deref_mut() {
            let dns_state = &mut self.dns_state;
            let socket_map = &mut self.socket_map;
            let udp_listener = &mut self.udp_listener;
            let wifi_connection = self.wifi_connection.as_mut();
            let ts = self.timer.now();

            let mut a = self.urc_attempts;
            let max = self.max_urc_attempts;

            self.client.peek_urc_with::<EdmEvent, _>(|edm_urc| {
                ran = true;
                let res = match edm_urc {
                    EdmEvent::ATEvent(urc) => {
                        match urc {
                            Urc::PeerConnected(event) => {
                                debug!("[URC] PeerConnected");

                                // TODO:
                                //
                                // We should probably move
                                // `tcp.set_state(TcpState::Connected(endpoint));`
                                // + `udp.set_state(UdpState::Established);` as
                                //   well as `tcp.update_handle(*socket);` +
                                //   `udp.update_handle(*socket);` here, to make
                                //   sure that part also works without EDM mode


                                let remote_ip = Ipv4Addr::from_str(
                                    core::str::from_utf8(event.remote_address.as_slice()).unwrap(),
                                )
                                .unwrap();

                                let remote = SocketAddr::new(remote_ip.into(), event.remote_port);

                                if let Some(queue) = udp_listener.incoming(event.local_port) {
                                    trace!("[UDP Server] Server socket incomming");
                                    let mut handled = true;
                                    if sockets.len() >= sockets.capacity() {
                                        // Check if there are any sockets closed by remote, and close it
                                        // if it has exceeded its timeout, in order to recycle it.
                                        // TODO Is this correct?
                                        if !sockets.recycle(self.timer.now()) {
                                            handled = false;
                                        }
                                    }
                                    let peer_handle = event.handle;
                                    let socket_handle = SocketHandle(new_socket_num(sockets).unwrap());
                                    let mut new_socket = UdpSocket::new(socket_handle.0);
                                    new_socket.set_state(UdpState::Established);
                                    if new_socket.bind(remote).is_err(){
                                        error!("[UDP_URC] Binding connecting socket Error");
                                        handled = false
                                    }
                                    if sockets.add(new_socket).map_err(|_| {
                                        error!("[UDP_URC] Opening socket Error: Socket set full");
                                        Error::SocketMemory
                                    }).is_err(){
                                        handled = false;
                                    }

                                    if socket_map.insert_peer(peer_handle, socket_handle).map_err(|_| {
                                        error!("[UDP_URC] Opening socket Error: Socket Map full");
                                        Error::SocketMapMemory
                                    }).is_err(){
                                        handled = false;
                                    }
                                    debug!(
                                        "[URC] Binding remote {=[u8]:a} to UDP server on port: {:?} with handle: {:?}",
                                        event.remote_address.as_slice(),
                                        event.local_port,
                                        socket_handle
                                    );
                                    if queue.enqueue((socket_handle, remote)).is_err(){
                                        handled = false
                                    }
                                    handled
                                } else {
                                    match event.protocol {
                                        IPProtocol::TCP => {
                                            // if let Ok(mut tcp) =
                                            //     sockets.get::<TcpSocket<CLK, L>>(event.handle)
                                            // {
                                            //     debug!(
                                            //         "Binding remote {=[u8]:a} to TCP socket {:?}",
                                            //         event.remote_address.as_slice(),
                                            //         event.handle
                                            //     );
                                            //     tcp.set_state(TcpState::Connected(remote));
                                            //     return true;
                                            // }
                                        }
                                        IPProtocol::UDP => {
                                            // if let Ok(mut udp) =
                                            //     sockets.get::<UdpSocket<TIMER_HZ, L>>(event.handle)
                                            // {
                                            //     debug!(
                                            //         "Binding remote {=[u8]:a} to UDP socket {:?}",
                                            //         event.remote_address.as_slice(),
                                            //         event.handle
                                            //     );
                                            //     udp.bind(remote).unwrap();
                                            //     udp.set_state(UdpState::Established);
                                            //     return true;
                                            // }
                                        }
                                    }
                                    true
                                }
                            }
                            Urc::PeerDisconnected(msg) => {
                                debug!("[URC] PeerDisconnected");
                                if let Some(handle) = socket_map.peer_to_socket(&msg.handle) {
                                    match sockets.socket_type(*handle) {
                                        Some(SocketType::Tcp) => {
                                            if let Ok(mut tcp) =
                                                sockets.get::<TcpSocket<TIMER_HZ, L>>(*handle)
                                            {
                                                tcp.closed_by_remote(ts);
                                            }
                                        }
                                        Some(SocketType::Udp) => {
                                            if let Ok(mut udp) =
                                                sockets.get::<UdpSocket<TIMER_HZ, L>>(*handle)
                                            {
                                                udp.close();
                                            }
                                            sockets.remove(*handle).ok();
                                        }
                                        _ => {}
                                    }
                                    socket_map.remove_peer(&msg.handle).unwrap();
                                }
                                true
                            }
                            Urc::WifiLinkConnected(msg) => {
                                debug!("[URC] WifiLinkConnected");
                                if let Some(con) = wifi_connection {
                                    con.wifi_state = WiFiState::Connected;
                                    con.network.bssid = msg.bssid;
                                    con.network.channel = msg.channel;
                                }
                                true
                            }
                            Urc::WifiLinkDisconnected(msg) => {
                                debug!("[URC] WifiLinkDisconnected");
                                if let Some(con) = wifi_connection {
                                    match msg.reason {
                                        DisconnectReason::NetworkDisabled => {
                                            con.wifi_state = WiFiState::Inactive;
                                        }
                                        DisconnectReason::SecurityProblems => {
                                            error!("Wifi Security Problems");
                                        }
                                        _ => {
                                            con.wifi_state = WiFiState::NotConnected;
                                        }
                                    }
                                }
                                true
                            }
                            Urc::WifiAPUp(_) => {
                                debug!("[URC] WifiAPUp");
                                true
                            }
                            Urc::WifiAPDown(_) => {
                                debug!("[URC] WifiAPDown");
                                true
                            }
                            Urc::WifiAPStationConnected(client) => {
                                debug!(
                                    "[URC] WifiAPStationConnected {=[u8]:a}",
                                    client.mac_addr.into_inner()
                                );
                                true
                            }
                            Urc::WifiAPStationDisconnected(_) => {
                                debug!("[URC] WifiAPStationDisconnected");
                                true
                            }
                            Urc::EthernetLinkUp(_) => {
                                debug!("[URC] EthernetLinkUp");
                                true
                            }
                            Urc::EthernetLinkDown(_) => {
                                debug!("[URC] EthernetLinkDown");
                                true
                            }
                            Urc::NetworkUp(_) => {
                                debug!("[URC] NetworkUp");
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
                                debug!("[URC] NetworkDown");
                                if let Some(con) = wifi_connection {
                                    con.network_state = NetworkState::Unattached;
                                }
                                true
                            }
                            Urc::NetworkError(_) => {
                                debug!("[URC] NetworkError");
                                true
                            }
                            Urc::PingResponse(resp) => {
                                debug!("[URC] PingResponse");
                                if *dns_state == DNSState::Resolving {
                                    *dns_state = DNSState::Resolved(resp.ip)
                                }
                                true
                            }
                            Urc::PingErrorResponse(resp) => {
                                debug!("[URC] PingErrorResponse: {:?}", resp.error);
                                if *dns_state == DNSState::Resolving {
                                    *dns_state = DNSState::Error(resp.error)
                                }
                                true
                            }
                        }
                    } // end match urc
                    EdmEvent::StartUp => {
                        debug!("[EDM_URC] STARTUP");
                        true
                    }
                    EdmEvent::IPv4ConnectEvent(event) => {
                        debug!(
                            "[EDM_URC] IPv4ConnectEvent! Channel_id: {:?}",
                            event.channel_id
                        );

                        let endpoint = SocketAddr::new(event.remote_ip.into(), event.remote_port);

                        // This depends upon Connected AT-URC to arrive first.
                        if let Some(queue) = udp_listener.incoming(event.local_port) {
                            if let Some((socket_handle, _ )) = queue.into_iter().find(|(_, remote)| remote == &endpoint) {
                                socket_map.insert_channel(event.channel_id, *socket_handle).is_ok()
                            } else {
                                false
                            }
                        } else {
                            sockets
                                .iter_mut()
                                .find_map(|(h, s)| {
                                    match event.protocol {
                                        Protocol::TCP => {
                                            let mut tcp = TcpSocket::downcast(s).ok()?;
                                            if tcp.endpoint() == Some(endpoint) {
                                                socket_map.insert_channel(event.channel_id, h).unwrap();
                                                tcp.set_state(TcpState::Connected(endpoint));
                                                return Some(true);
                                            }
                                        }
                                        Protocol::UDP => {
                                            let mut udp = UdpSocket::downcast(s).ok()?;
                                            if udp.endpoint() == Some(endpoint) {
                                                socket_map.insert_channel(event.channel_id, h).unwrap();
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
                    }
                    EdmEvent::IPv6ConnectEvent(event) => {
                        debug!(
                            "[EDM_URC] IPv6ConnectEvent! Channel_id: {:?}",
                            event.channel_id
                        );

                        let endpoint = SocketAddr::new(event.remote_ip.into(), event.remote_port);

                        // This depends upon Connected AT-URC to arrive first.
                        if let Some(queue) = udp_listener.incoming(event.local_port) {
                            if let Some((socket_handle, _ )) = queue.into_iter().find(|(_, remote)| remote == &endpoint) {
                                socket_map.insert_channel(event.channel_id, *socket_handle).is_ok()
                            } else {
                                false
                            }
                        } else {
                            sockets
                                .iter_mut()
                                .find_map(|(h, s)| {
                                    match event.protocol {
                                        Protocol::TCP => {
                                            let mut tcp = TcpSocket::downcast(s).ok()?;
                                            if tcp.endpoint() == Some(endpoint) {
                                                socket_map.insert_channel(event.channel_id, h).unwrap();
                                                tcp.set_state(TcpState::Connected(endpoint));
                                                return Some(true);
                                            }
                                        }
                                        Protocol::UDP => {
                                            let mut udp = UdpSocket::downcast(s).ok()?;
                                            if udp.endpoint() == Some(endpoint) {
                                                socket_map.insert_channel(event.channel_id, h).unwrap();
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
                    }
                    EdmEvent::BluetoothConnectEvent(_) => {
                        debug!("[EDM_URC] BluetoothConnectEvent");
                        true
                    }
                    EdmEvent::DisconnectEvent(channel_id) => {
                        debug!("[EDM_URC] DisconnectEvent! Channel_id: {:?}", channel_id);
                        socket_map.remove_channel(&channel_id).unwrap();
                        true
                    }
                    EdmEvent::DataEvent(event) => {
                        debug!("[EDM_URC] DataEvent! Channel_id: {:?}", event.channel_id);
                        if !event.data.is_empty() {
                            if let Some(socket_handle) =
                                socket_map.channel_to_socket(&event.channel_id)
                            {
                                match sockets.socket_type(*socket_handle) {
                                    Some(SocketType::Tcp) => {
                                        // Handle tcp socket
                                        let mut tcp = sockets
                                            .get::<TcpSocket<TIMER_HZ, L>>(*socket_handle)
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
                                            .get::<UdpSocket<TIMER_HZ, L>>(*socket_handle)
                                            .unwrap();

                                        if udp.can_recv() {
                                            udp.rx_enqueue_slice(&event.data);
                                            true
                                        } else {
                                            false
                                        }
                                    }
                                    _ => {
                                        error!("SocketNotFound {:?}", socket_handle);
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
                        error!("[EDM_URC] URC handeling failed");
                        a += 1;
                        return false;
                    }
                    error!("[EDM_URC] URC thrown away");
                }
                a = 0;
                true
            });
            self.urc_attempts = a;
        }
        Ok(ran)
    }

    /// Send AT command
    /// Automaticaly waraps commands in EDM context
    pub fn send_at<A, const LEN: usize>(&mut self, cmd: A) -> Result<A::Response, Error>
    where
        A: atat::AtatCmd<LEN>,
    {
        if !self.initialized {
            self.init()?;
        }
        match self.serial_mode {
            SerialMode::ExtendedData => self.send_internal(&EdmAtCmdWrapper(cmd), true),
            SerialMode::Cmd => self.send_internal(&cmd, true),
        }
    }

    pub fn connected_to_network(&self) -> Result<(), Error> {
        if let Some(ref con) = self.wifi_connection {
            if !self.initialized {
                Err(Error::Uninitialized)
            } else if !con.is_connected() {
                Err(Error::WifiState(con.wifi_state))
            } else if self.sockets.is_none() {
                Err(Error::MissingSocketSet)
            } else {
                Ok(())
            }
        } else {
            Err(Error::NoWifiSetup)
        }
    }
}
