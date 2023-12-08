use core::str::FromStr;

use crate::{
    blocking_timer::BlockingTimer,
    command::{
        custom_digest::EdmDigester,
        data_mode::{
            types::{IPProtocol, PeerConfigParameter},
            SetPeerConfiguration,
        },
        edm::{types::Protocol, urc::EdmEvent, EdmAtCmdWrapper, SwitchToEdmCommand},
        general::{types::FirmwareVersion, SoftwareVersion},
        network::SetNetworkHostName,
        ping::types::PingError,
        system::{
            types::{BaudRate, ChangeAfterConfirm, FlowControl, Parity, StopBits},
            RebootDCE, SetRS232Settings, StoreCurrentConfig,
        },
        wifi::{
            responses::WifiStatusResponse,
            types::{DisconnectReason, StatusId, WifiConfig, WifiStatus},
            GetWifiStatus, SetWifiConfig,
        },
        Urc,
    },
    config::Config,
    error::Error,
    wifi::{
        connection::{NetworkState, WiFiState, WifiConnection},
        network::{WifiMode, WifiNetwork},
        SocketMap,
    },
    UbloxWifiBuffers, UbloxWifiIngress, UbloxWifiUrcChannel,
};
use atat::{blocking::AtatClient, heapless_bytes::Bytes, AtatUrcChannel, UrcSubscription};
use defmt::{debug, error, trace};
use embassy_time::{Duration, Instant};
use embedded_hal::digital::OutputPin;
use embedded_nal::{IpAddr, Ipv4Addr, SocketAddr};
use ublox_sockets::{
    udp_listener::UdpListener, AnySocket, SocketHandle, SocketSet, SocketType, TcpSocket, TcpState,
    UdpSocket, UdpState,
};

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum SerialMode {
    Cmd,
    ExtendedData,
}

#[derive(PartialEq, Copy, Clone)]
pub enum DNSState {
    Resolving,
    Resolved(IpAddr),
    Error(PingError),
}

/// From u-connectXpress AT commands manual:
/// <domain> depends on the <scheme>. For internet domain names, the maximum
/// length is 64 characters.
/// Domain name length is 128 for NINA-W13 and NINA-W15 software version 4.0
/// .0 or later.
#[cfg(not(feature = "nina_w1xx"))]
pub const MAX_DOMAIN_NAME_LENGTH: usize = 64;

#[cfg(feature = "nina_w1xx")]
pub const MAX_DOMAIN_NAME_LENGTH: usize = 128;

pub struct DNSTableEntry {
    domain_name: heapless::String<MAX_DOMAIN_NAME_LENGTH>,
    state: DNSState,
}

impl DNSTableEntry {
    pub const fn new(
        state: DNSState,
        domain_name: heapless::String<MAX_DOMAIN_NAME_LENGTH>,
    ) -> Self {
        Self { domain_name, state }
    }
}

pub struct DNSTable {
    pub table: heapless::Deque<DNSTableEntry, 3>,
}

impl DNSTable {
    const fn new() -> Self {
        Self {
            table: heapless::Deque::new(),
        }
    }
    pub fn upsert(&mut self, new_entry: DNSTableEntry) {
        if let Some(entry) = self
            .table
            .iter_mut()
            .find(|e| e.domain_name == new_entry.domain_name)
        {
            entry.state = new_entry.state;
            return;
        }

        if self.table.is_full() {
            self.table.pop_front();
        }
        unsafe {
            self.table.push_back_unchecked(new_entry);
        }
    }

    pub fn get_state(
        &self,
        domain_name: heapless::String<MAX_DOMAIN_NAME_LENGTH>,
    ) -> Option<DNSState> {
        self.table
            .iter()
            .find(|e| e.domain_name == domain_name)
            .map(|x| x.state)
    }
    pub fn reverse_lookup(&self, ip: IpAddr) -> Option<&heapless::String<MAX_DOMAIN_NAME_LENGTH>> {
        match self
            .table
            .iter()
            .find(|e| e.state == DNSState::Resolved(ip))
        {
            Some(entry) => Some(&entry.domain_name),
            None => None,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Default)]
pub struct SecurityCredentials {
    pub ca_cert_name: Option<heapless::String<16>>,
    pub c_cert_name: Option<heapless::String<16>>, // TODO: Make &str with lifetime
    pub c_key_name: Option<heapless::String<16>>,
}

/// Creates new socket numbers
/// Properly not Async safe
pub fn new_socket_num<const N: usize, const L: usize>(sockets: &SocketSet<N, L>) -> Result<u8, ()> {
    let mut num = 0;
    while sockets.socket_type(SocketHandle(num)).is_some() {
        num += 1;
        if num == u8::MAX {
            return Err(());
        }
    }
    Ok(num)
}

pub(crate) const URC_CAPACITY: usize = 2;
pub(crate) const URC_SUBSCRIBERS: usize = 1;

pub struct UbloxClient<'buf, 'sub, AtCl, AtUrcCh, RST, const N: usize, const L: usize>
where
    RST: OutputPin,
{
    pub(crate) module_started: bool,
    pub(crate) initialized: bool,
    serial_mode: SerialMode,
    pub(crate) dns_table: DNSTable,
    pub(crate) wifi_connection: Option<WifiConnection>,
    pub(crate) wifi_config_active_on_startup: Option<u8>,
    pub(crate) client: AtCl,
    pub(crate) config: Config<RST>,
    pub(crate) sockets: Option<&'static mut SocketSet<N, L>>,
    pub(crate) security_credentials: SecurityCredentials,
    pub(crate) socket_map: SocketMap,
    pub(crate) udp_listener: UdpListener<2, N>,
    urc_subscription: UrcSubscription<'sub, EdmEvent, URC_CAPACITY, URC_SUBSCRIBERS>,
    _urc_channel: &'buf AtUrcCh,
}

impl<'buf, 'sub, W, RST, const INGRESS_BUF_SIZE: usize, const N: usize, const L: usize>
    UbloxClient<
        'buf,
        'sub,
        atat::blocking::Client<'buf, W, INGRESS_BUF_SIZE>,
        UbloxWifiUrcChannel,
        RST,
        N,
        L,
    >
where
    'buf: 'sub,
    W: embedded_io::Write,
    RST: OutputPin,
{
    /// Create new u-blox device
    ///
    /// Look for [`data_service`](Device::data_service) how to handle data connection automatically.
    ///
    pub fn from_buffers(
        buffers: &'buf UbloxWifiBuffers<INGRESS_BUF_SIZE>,
        tx: W,
        config: Config<RST>,
    ) -> (UbloxWifiIngress<INGRESS_BUF_SIZE>, Self) {
        let (ingress, client) =
            buffers.split_blocking(tx, EdmDigester::default(), atat::Config::default());

        (
            ingress,
            UbloxClient::new(client, &buffers.urc_channel, config),
        )
    }
}

impl<'buf, 'sub, AtCl, AtUrcCh, RST, const N: usize, const L: usize>
    UbloxClient<'buf, 'sub, AtCl, AtUrcCh, RST, N, L>
where
    'buf: 'sub,
    AtCl: AtatClient,
    AtUrcCh: AtatUrcChannel<EdmEvent, URC_CAPACITY, URC_SUBSCRIBERS>,
    RST: OutputPin,
{
    pub fn new(client: AtCl, _urc_channel: &'buf AtUrcCh, config: Config<RST>) -> Self {
        let urc_subscription = _urc_channel.subscribe().unwrap();
        UbloxClient {
            module_started: false,
            initialized: false,
            serial_mode: SerialMode::Cmd,
            dns_table: DNSTable::new(),
            wifi_connection: None,
            wifi_config_active_on_startup: None,
            client,
            config,
            sockets: None,
            security_credentials: SecurityCredentials::default(),
            socket_map: SocketMap::default(),
            udp_listener: UdpListener::new(),
            urc_subscription,
            _urc_channel,
        }
    }
}

impl<'buf, 'sub, AtCl, AtUrcCh, RST, const N: usize, const L: usize>
    UbloxClient<'buf, 'sub, AtCl, AtUrcCh, RST, N, L>
where
    'buf: 'sub,
    AtCl: AtatClient,
    RST: OutputPin,
{
    pub fn set_socket_storage(&mut self, socket_set: &'static mut SocketSet<N, L>) {
        socket_set.prune();
        self.sockets.replace(socket_set);
    }

    pub fn take_socket_storage(&mut self) -> Option<&'static mut SocketSet<N, L>> {
        self.sockets.take()
    }

    pub fn has_socket_storage(&self) -> bool {
        self.sockets.is_some()
    }

    pub fn init(&mut self) -> Result<(), Error> {
        // Initilize a new ublox device to a known state (set RS232 settings)

        debug!("Initializing wifi");
        // Hard reset module
        self.reset()?;

        // Switch to EDM on Init. If in EDM, fail and check with autosense
        // if self.serial_mode != SerialMode::ExtendedData {
        //     self.retry_send(&SwitchToEdmCommand, 5)?;
        //     self.serial_mode = SerialMode::ExtendedData;
        // }

        while self.serial_mode != SerialMode::ExtendedData {
            self.send_internal(&SwitchToEdmCommand, true).ok();
            BlockingTimer::after(Duration::from_millis(100)).wait();
            self.handle_urc()?;
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

        self.send_internal(
            &EdmAtCmdWrapper(SetWifiConfig {
                config_param: WifiConfig::RemainOnChannel(0),
            }),
            false,
        )?;

        self.send_internal(&EdmAtCmdWrapper(StoreCurrentConfig), false)?;

        self.software_reset()?;

        while self.serial_mode != SerialMode::ExtendedData {
            self.send_internal(&SwitchToEdmCommand, true).ok();
            BlockingTimer::after(Duration::from_millis(100)).wait();
            self.handle_urc()?;
        }

        if self.firmware_version()? < FirmwareVersion::new(8, 0, 0) {
            self.config.network_up_bug = true;
        } else {
            if let Some(size) = self.config.tls_in_buffer_size {
                self.send_internal(
                    &EdmAtCmdWrapper(SetPeerConfiguration {
                        parameter: PeerConfigParameter::TlsInBuffer(size),
                    }),
                    false,
                )?;
            }

            if let Some(size) = self.config.tls_out_buffer_size {
                self.send_internal(
                    &EdmAtCmdWrapper(SetPeerConfiguration {
                        parameter: PeerConfigParameter::TlsOutBuffer(size),
                    }),
                    false,
                )?;
            }
        }

        self.load_wifi_config();

        self.initialized = true;

        Ok(())
    }

    pub fn firmware_version(&mut self) -> Result<FirmwareVersion, Error> {
        let response = self.send_internal(&EdmAtCmdWrapper(SoftwareVersion), false)?;
        Ok(response.version)
    }

    pub fn signal_strength(&mut self) -> Result<i16, Error> {
        if let WifiStatusResponse {
            status_id: WifiStatus::RSSI(rssi),
        } = self.send_internal(
            &EdmAtCmdWrapper(GetWifiStatus {
                status_id: StatusId::RSSI,
            }),
            false,
        )? {
            Ok(rssi)
        } else {
            Err(Error::_Unknown)
        }
    }

    fn load_wifi_config(&mut self) {
        //Check if we have active network config if so then update wifi_connection
        if let Ok(active) = self.is_active_on_startup() {
            if active {
                if let Ok(ssid) = self.get_ssid() {
                    defmt::info!("Found network in module configuring as active network");
                    self.wifi_connection.replace(WifiConnection::new(
                        WifiNetwork {
                            bssid: Bytes::new(),
                            op_mode: crate::command::wifi::types::OperationMode::Infrastructure,
                            ssid: ssid,
                            channel: 0,
                            rssi: 1,
                            authentication_suites: 0,
                            unicast_ciphers: 0,
                            group_ciphers: 0,
                            mode: WifiMode::Station,
                        },
                        WiFiState::NotConnected,
                    ));
                }
            }
        }
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
        self.module_started = false;
        self.wifi_connection = None;
        self.wifi_config_active_on_startup = None;
        self.security_credentials = SecurityCredentials::default();
        self.socket_map = SocketMap::default();
        self.udp_listener = UdpListener::new();

        self.clear_buffers()?;

        if let Some(ref mut pin) = self.config.rst_pin {
            defmt::warn!("Hard resetting Ublox Short Range");
            pin.set_low().ok();

            BlockingTimer::after(Duration::from_millis(50)).wait();

            pin.set_high().ok();

            let expiration = Instant::now() + Duration::from_secs(4);

            while Instant::now() < expiration {
                self.handle_urc().ok();
                if self.module_started {
                    return Ok(());
                }
            }
            return Err(Error::Timeout);
        }

        Ok(())
    }

    pub fn software_reset(&mut self) -> Result<(), Error> {
        self.serial_mode = SerialMode::Cmd;
        self.initialized = false;
        self.module_started = false;
        self.wifi_connection = None;
        self.wifi_config_active_on_startup = None;
        self.security_credentials = SecurityCredentials::default();
        self.socket_map = SocketMap::default();
        self.udp_listener = UdpListener::new();

        defmt::warn!("Soft resetting Ublox Short Range");
        self.send_internal(&EdmAtCmdWrapper(RebootDCE), false)?;
        self.clear_buffers()?;

        let expiration = Instant::now() + Duration::from_secs(4);
        while Instant::now() < expiration {
            self.handle_urc().ok();
            if self.module_started {
                return Ok(());
            }
        }
        Err(Error::Timeout)
    }

    pub(crate) fn clear_buffers(&mut self) -> Result<(), Error> {
        if let Some(ref mut sockets) = self.sockets.as_deref_mut() {
            sockets.prune();
        }

        Ok(())
    }

    pub fn spin(&mut self) -> Result<(), Error> {
        if !self.initialized {
            return Err(Error::Uninitialized);
        }

        self.handle_urc()?;

        self.connected_to_network()?;

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

        self.client.send(req).map_err(|e| {
            error!("{:?}: {=[u8]:a}", e, req.as_bytes());
            e.into()
        })
    }

    fn handle_urc(&mut self) -> Result<(), Error> {
        if let Some(edm_urc) = self.urc_subscription.try_next_message_pure() {
            match edm_urc {
                EdmEvent::ATEvent(urc) => {
                    match urc {
                        Urc::StartUp => {
                            debug!("[URC] Startup");
                            self.module_started = true;
                            self.initialized = false;
                            self.serial_mode = SerialMode::Cmd;
                        }
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

                            if let Some(sockets) = self.sockets.as_deref_mut() {
                                let remote_ip = Ipv4Addr::from_str(
                                    core::str::from_utf8(event.remote_address.as_slice()).unwrap(),
                                )
                                .unwrap();

                                let remote = SocketAddr::new(remote_ip.into(), event.remote_port);

                                if let Some(queue) = self.udp_listener.incoming(event.local_port) {
                                    trace!("[UDP Server] Server socket incomming");
                                    if sockets.len() >= sockets.capacity() {
                                        // Check if there are any sockets closed by remote, and close it
                                        // if it has exceeded its timeout, in order to recycle it.
                                        // TODO Is this correct?
                                        if !sockets.recycle() {}
                                    }
                                    let peer_handle = event.handle;
                                    let socket_handle =
                                        SocketHandle(new_socket_num(sockets).unwrap());
                                    let mut new_socket = UdpSocket::new(socket_handle.0);
                                    new_socket.set_state(UdpState::Established);
                                    if new_socket.bind(remote).is_err() {
                                        error!("[UDP_URC] Binding connecting socket Error");
                                    }
                                    if sockets.add(new_socket).is_err() {
                                        error!("[UDP_URC] Opening socket Error: Socket set full");
                                    }
                                    if self
                                        .socket_map
                                        .insert_peer(peer_handle, socket_handle)
                                        .is_err()
                                    {
                                        error!("[UDP_URC] Opening socket Error: Socket Map full");
                                    }
                                    debug!(
                                        "[URC] Binding remote {=[u8]:a} to UDP server on port: {:?} with handle: {:?}",
                                        event.remote_address.as_slice(),
                                        event.local_port,
                                        socket_handle
                                    );
                                    queue.enqueue((socket_handle, remote)).ok();
                                } else {
                                    match event.protocol {
                                        IPProtocol::TCP => {
                                            // if let Ok(mut tcp) =
                                            //     sockets.get::<TcpSocket L>>(event.handle)
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
                                            //     sockets.get::<UdpSocket<L>>(event.handle)
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
                                }
                            }
                        }
                        Urc::PeerDisconnected(msg) => {
                            debug!("[URC] PeerDisconnected");
                            if let Some(sockets) = self.sockets.as_deref_mut() {
                                if let Some(handle) = self.socket_map.peer_to_socket(&msg.handle) {
                                    match sockets.socket_type(*handle) {
                                        Some(SocketType::Tcp) => {
                                            if let Ok(mut tcp) =
                                                sockets.get::<TcpSocket<L>>(*handle)
                                            {
                                                tcp.closed_by_remote();
                                            }
                                        }
                                        Some(SocketType::Udp) => {
                                            if let Ok(mut udp) =
                                                sockets.get::<UdpSocket<L>>(*handle)
                                            {
                                                udp.close();
                                            }
                                            sockets.remove(*handle).ok();
                                        }
                                        _ => {}
                                    }
                                    self.socket_map.remove_peer(&msg.handle);
                                }
                            }
                        }
                        Urc::WifiLinkConnected(msg) => {
                            debug!("[URC] WifiLinkConnected");
                            if let Some(ref mut con) = self.wifi_connection {
                                con.wifi_state = WiFiState::Connected;
                                con.network.bssid = msg.bssid;
                                con.network.channel = msg.channel;
                            } else {
                                debug!("[URC] Active network config discovered");
                                self.wifi_connection.replace(
                                    WifiConnection::new(
                                        WifiNetwork {
                                            bssid: msg.bssid,
                                            op_mode: crate::command::wifi::types::OperationMode::Infrastructure,
                                            ssid: heapless::String::new(),
                                            channel: msg.channel,
                                            rssi: 1,
                                            authentication_suites: 0,
                                            unicast_ciphers: 0,
                                            group_ciphers: 0,
                                            mode: WifiMode::Station,
                                        },
                                        WiFiState::Connected,
                                    ).activate()
                                );
                            }
                        }
                        Urc::WifiLinkDisconnected(msg) => {
                            debug!("[URC] WifiLinkDisconnected");
                            if let Some(ref mut con) = self.wifi_connection {
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
                        }
                        Urc::WifiAPUp(_) => {
                            debug!("[URC] WifiAPUp");
                        }
                        Urc::WifiAPDown(_) => {
                            debug!("[URC] WifiAPDown");
                        }
                        Urc::WifiAPStationConnected(client) => {
                            debug!(
                                "[URC] WifiAPStationConnected {=[u8]:a}",
                                client.mac_addr.into_inner()
                            );
                        }
                        Urc::WifiAPStationDisconnected(_) => {
                            debug!("[URC] WifiAPStationDisconnected");
                        }
                        Urc::EthernetLinkUp(_) => {
                            debug!("[URC] EthernetLinkUp");
                        }
                        Urc::EthernetLinkDown(_) => {
                            debug!("[URC] EthernetLinkDown");
                        }
                        Urc::NetworkUp(_) => {
                            debug!("[URC] NetworkUp");
                            if let Some(ref mut con) = self.wifi_connection {
                                if self.config.network_up_bug {
                                    match con.network_state {
                                        NetworkState::Attached => (),
                                        NetworkState::AlmostAttached => {
                                            con.network_state = NetworkState::Attached
                                        }
                                        NetworkState::Unattached => {
                                            con.network_state = NetworkState::AlmostAttached
                                        }
                                    }
                                } else {
                                    con.network_state = NetworkState::Attached;
                                }
                            }
                        }
                        Urc::NetworkDown(_) => {
                            debug!("[URC] NetworkDown");
                            if let Some(ref mut con) = self.wifi_connection {
                                con.network_state = NetworkState::Unattached;
                            }
                        }
                        Urc::NetworkError(_) => {
                            debug!("[URC] NetworkError");
                        }
                        Urc::PingResponse(resp) => {
                            debug!("[URC] PingResponse");
                            self.dns_table.upsert(DNSTableEntry {
                                domain_name: resp.hostname,
                                state: DNSState::Resolved(resp.ip),
                            });
                        }
                        Urc::PingErrorResponse(resp) => {
                            debug!("[URC] PingErrorResponse: {:?}", resp.error);
                        }
                    }
                } // end match urc
                EdmEvent::StartUp => {
                    debug!("[EDM_URC] STARTUP");
                    self.module_started = true;
                    self.serial_mode = SerialMode::ExtendedData;
                }
                EdmEvent::IPv4ConnectEvent(event) => {
                    debug!(
                        "[EDM_URC] IPv4ConnectEvent! Channel_id: {:?}",
                        event.channel_id
                    );

                    if let Some(sockets) = self.sockets.as_deref_mut() {
                        let endpoint = SocketAddr::new(event.remote_ip.into(), event.remote_port);

                        // This depends upon Connected AT-URC to arrive first.
                        if let Some(queue) = self.udp_listener.incoming(event.local_port) {
                            if let Some((socket_handle, _)) =
                                queue.into_iter().find(|(_, remote)| remote == &endpoint)
                            {
                                self.socket_map
                                    .insert_channel(event.channel_id, *socket_handle)
                                    .ok();
                            }
                        } else {
                            for (h, s) in sockets.iter_mut() {
                                match (&event.protocol, s.get_type()) {
                                    (Protocol::TCP, SocketType::Tcp) => {
                                        let mut tcp = TcpSocket::downcast(s)?;
                                        if tcp.endpoint() == Some(endpoint) {
                                            self.socket_map
                                                .insert_channel(event.channel_id, h)
                                                .ok();
                                            tcp.set_state(TcpState::Connected(endpoint));
                                        }
                                    }
                                    (Protocol::UDP, SocketType::Udp) => {
                                        let mut udp = UdpSocket::downcast(s)?;
                                        if udp.endpoint() == Some(endpoint) {
                                            self.socket_map
                                                .insert_channel(event.channel_id, h)
                                                .ok();
                                            udp.set_state(UdpState::Established);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                EdmEvent::IPv6ConnectEvent(event) => {
                    debug!(
                        "[EDM_URC] IPv6ConnectEvent! Channel_id: {:?}",
                        event.channel_id
                    );

                    if let Some(sockets) = self.sockets.as_deref_mut() {
                        let endpoint = SocketAddr::new(event.remote_ip.into(), event.remote_port);

                        // This depends upon Connected AT-URC to arrive first.
                        if let Some(queue) = self.udp_listener.incoming(event.local_port) {
                            if let Some((socket_handle, _)) =
                                queue.into_iter().find(|(_, remote)| remote == &endpoint)
                            {
                                self.socket_map
                                    .insert_channel(event.channel_id, *socket_handle)
                                    .ok();
                            }
                        } else {
                            for (h, s) in sockets.iter_mut() {
                                match (&event.protocol, s.get_type()) {
                                    (Protocol::TCP, SocketType::Tcp) => {
                                        let mut tcp = TcpSocket::downcast(s)?;
                                        if tcp.endpoint() == Some(endpoint) {
                                            self.socket_map
                                                .insert_channel(event.channel_id, h)
                                                .ok();
                                            tcp.set_state(TcpState::Connected(endpoint));
                                        }
                                    }
                                    (Protocol::UDP, SocketType::Udp) => {
                                        let mut udp = UdpSocket::downcast(s)?;
                                        if udp.endpoint() == Some(endpoint) {
                                            self.socket_map
                                                .insert_channel(event.channel_id, h)
                                                .ok();
                                            udp.set_state(UdpState::Established);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                EdmEvent::BluetoothConnectEvent(_) => {
                    debug!("[EDM_URC] BluetoothConnectEvent");
                }
                EdmEvent::DisconnectEvent(channel_id) => {
                    debug!("[EDM_URC] DisconnectEvent! Channel_id: {:?}", channel_id);
                    self.socket_map.remove_channel(&channel_id);
                }
                EdmEvent::DataEvent(event) => {
                    debug!("[EDM_URC] DataEvent! Channel_id: {:?}", event.channel_id);
                    if let Some(sockets) = self.sockets.as_deref_mut() {
                        if !event.data.is_empty() {
                            if let Some(socket_handle) =
                                self.socket_map.channel_to_socket(&event.channel_id)
                            {
                                match sockets.socket_type(*socket_handle) {
                                    Some(SocketType::Tcp) => {
                                        // Handle tcp socket
                                        let mut tcp =
                                            sockets.get::<TcpSocket<L>>(*socket_handle).unwrap();
                                        if tcp.can_recv() {
                                            tcp.rx_enqueue_slice(&event.data);
                                        }
                                    }
                                    Some(SocketType::Udp) => {
                                        // Handle udp socket
                                        let mut udp =
                                            sockets.get::<UdpSocket<L>>(*socket_handle).unwrap();

                                        if udp.can_recv() {
                                            udp.rx_enqueue_slice(&event.data);
                                        }
                                    }
                                    _ => {
                                        error!("SocketNotFound {:?}", socket_handle);
                                    }
                                }
                            }
                        }
                    }
                }
            };
        };
        Ok(())
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

    /// Is the module attached to a WiFi and ready to open sockets
    pub(crate) fn connected_to_network(&self) -> Result<(), Error> {
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

    /// Is the module attached to a WiFi
    ///
    // TODO: handle this case for better stability
    // WiFi connection can disconnect momentarily, but if the network state does not change
    // the current context is safe.
    pub fn attached_to_wifi(&self) -> Result<(), Error> {
        if let Some(ref con) = self.wifi_connection {
            if !self.initialized {
                Err(Error::Uninitialized)
            // } else if !(con.network_state == NetworkState::Attached) {
            } else if !con.is_connected() {
                if con.wifi_state == WiFiState::Connected {
                    Err(Error::NetworkState(con.network_state))
                } else {
                    Err(Error::WifiState(con.wifi_state))
                }
            } else {
                Ok(())
            }
        } else {
            Err(Error::NoWifiSetup)
        }
    }
}
