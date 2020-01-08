use super::*;
use heapless::{consts, String, Vec};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Response {
    ManufacturerId {
        id: String<at::MaxCommandLen>,
    },
    ModelId {
        id: String<at::MaxCommandLen>,
    },
    FWVersion {
        version: String<at::MaxCommandLen>,
    },
    SerialNum {
        serial: String<at::MaxCommandLen>,
    },
    Id {
        id: String<at::MaxCommandLen>,
    },
    GreetingText {
        enable: bool,
        text: String<at::MaxCommandLen>,
    },

    DTR {
        value: DTRValue,
    },
    DSR {
        value: DSRValue,
    },
    Echo {
        enable: bool,
    },
    Escape {
        esc_char: char,
    },
    Termination {
        line_term: char,
    },
    Formatting {
        term: char,
    },
    Backspace {
        backspace: char,
    },
    StartMode {
        start_mode: Mode,
    },
    LocalAddr {
        interface_id: InterfaceId,
        address: String<at::MaxCommandLen>,
    },
    SystemStatus {
        status_id: bool,
    },
    RS232Settings {
        baud_rate: BaudRate,
        flow_control: FlowControl,
        data_bits: u8,
        stop_bits: StopBits,
        parity: Parity,
        change_after_confirm: ChangeAfterConfirm,
    },

    Mode {
        mode: Mode,
    },

    ConnectPeer {
        url: String<at::MaxCommandLen>,
    },
    ClosePeerConnection {
        peer_handle: u8,
    },
    GetDefaultPeer {
        peer_id: u8,
    },
    SetDefaultPeer {
        peer_id: u8,
        url: String<at::MaxCommandLen>,
        connect_scheme: u8,
    },
    // SetServerCfg(u8, u8),
    // GetServerCfg(u8),
    GetWatchdogSettings {
        wd_type: WatchDogType,
    },
    SetWatchdogSettings {
        wd_type: WatchDogType,
        timeout: u32,
    },
    GetPeerConfig {
        param: PeerConfigGet,
    },
    SetPeerConfig {
        param: PeerConfigSet,
    },

    // 8 Bluetooth Commands
    GetDiscoverable,
    SetDiscoverable {
        discoverability_mode: DiscoverabilityMode,
    },
    GetConnectable,
    SetConnectable {
        connectability_mode: ConnectabilityMode,
    },
    GetParingMode,
    SetParingMode {
        pairing_mode: PairingMode,
    },
    GetSecurityMode,
    SetSecurityMode {
        security_mode: SecurityMode,
        security_mode_bt2_0: SecurityModeBT2_0,
        fixed_pin: String<at::MaxCommandLen>,
    },
    UserConfirmation {
        bd_addr: String<at::MaxCommandLen>,
        yes_no: bool,
    },
    UserPasskey {
        bd_addr: String<at::MaxCommandLen>,
        ok_cancel: bool,
        passkey: u16,
    },
    NameDiscovery {
        device_name: String<at::MaxCommandLen>,
        mode: BTMode,
    },
    GetManufacturerId,
    Inquiry {
        inquiry_type: InquiryType,
        inquiry_length: u8,
    },
    Discovery {
        discovery_type: DiscoveryType,
        mode: DiscoveryMode,
        inquiry_length: u8,
    },
    Bond {
        bd_addr: String<at::MaxCommandLen>,
        mode: BTMode,
    },
    UnBond {
        bd_addr: String<at::MaxCommandLen>,
    },
    GetBonds {
        mode: BTMode,
    },
    GetLocalName,
    SetLocalName {
        device_name: String<at::MaxCommandLen>,
    },
    GetLocalCOD,
    SetLocalCOD {
        cod: Vec<u8, consts::U8>,
    },
    GetMasterSlaveRole {
        bd_addr: String<at::MaxCommandLen>,
    },
    GetRolePolicy,
    SetRolePolicy {
        role_policy: bool,
    },
    GetRSSI {
        bd_addr: String<at::MaxCommandLen>,
    },
    GetLinkQuality {
        bd_addr: String<at::MaxCommandLen>,
    },
    GetRoleConfiguration,
    SetRoleConfiguration {
        role: BTRole,
    },
    GetLEAdvertiseData,
    SetLEAdvertiseData {
        data: Vec<u8, consts::U8>,
    },
    GetLEScanResponseData,
    SetLEScanResponseData {
        data: Vec<u8, consts::U8>,
    },
    ServiceSearch {
        bd_addr: String<at::MaxCommandLen>,
        service_type: ServiceType,
        uuid: Vec<u8, consts::U8>,
    },
    // GetWatchdogParameter(u8),
    // SetWatchdogParameter(u8, u8),
    // GetBTConfig(u8),
    // SetBTConfig(u8, u8),
    // GetBTLEConfig(u8),
    // SetBTLEConfig(u8, u8),

    // 9 Wi-Fi
    STAGetConfig {
        configuration_id: ConfigId,
        param_tag: UWSCGetTag,
    },
    STASetConfig {
        configuration_id: ConfigId,
        param_tag: UWSCSetTag,
    },
    ExecSTAAction {
        configuration_id: ConfigId,
        action: STAAction,
    },
    STAGetConfigList,
    STAScan {
        bssid: String<at::MaxCommandLen>,
        op_mode: OPMode,
        ssid: String<at::MaxCommandLen>,
        channel: u8,
        rssi: i16,
        authentication_suites: u8,
        unicast_ciphers: u8,
        group_ciphers: u8,
    },
    STASetChannelList {
        channel_list: Vec<u8, consts::U8>,
    },
    STAGetChannelList,
    WIFIGetWatchdogParameter {
        wd_type: WIFIWatchDogTypeGet,
    },
    WIFISetWatchdogParameter {
        wd_type: WIFIWatchDogTypeSet,
    },
    STAGetStatus {
        status_id: STAStatus,
    },

    // 10 Network
    GetHostname,
    SetHostname {
        hostname: String<at::MaxCommandLen>,
    },
    GetNetworkStatus {
        interface_type: InterfaceType,
        status_id: StatusId,
    },
}


/// Unsolicited
#[derive(Debug, Clone)]
pub enum UnsolicitedResponse {
    Startup,
    /// 5.10 Peer connected \
    /// A Bluetooth peer has been connected
    BluetoothPeerConnected {
        peer_handle: u8,
        profile: PeerProfile,
        address: u8,
        frame_size: u64,
    },
    /// An IP peer has been connected
    IPPeerConnected { peer_handle: u8, r#type: u8 },
    /// 5.11 Peer disconnected \
    /// A connection to a remote peer has been disconnected
    PeerDisconnected { peer_handle: u8 },
    /// 7.15 Wi-Fi Link connected +UUWLE
    /// Unsolicited response code for Wi-Fi
    /// Wi-Fi connection established
    WifiLinkConnected { connection_id: u8, bssid: String<at::MaxCommandLen>, channel: u8 },
    /// 7.16 Wi-Fi Link disconnected +UUWLD
    /// Unsolicited response code for Wi-Fi
    /// Wi-Fi connection disconnected.
    WifiLinkDisconnected { connection_id: u8, reason: u8 },
    /// 10.6 Network down
    /// Network is down
    NetworkUp { interface_id: u8 },
    /// 10.7 Network down
    /// Network is down
    NetworkDown { interface_id: u8 },
    /// 10.8 Network error
    /// An error has occured
    NetworkError { interface_id: u8, error_code: u8 },

}
