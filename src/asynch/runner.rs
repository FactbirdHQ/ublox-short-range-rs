use super::{control::Control, network::NetDevice, state, Resources, UbloxUrc};
use crate::{
    asynch::control::ProxyClient,
    command::{
        data_mode::{self, ChangeMode},
        general::SoftwareVersion,
        system::{
            types::{BaudRate, ChangeAfterConfirm, EchoOn, FlowControl, Parity, StopBits},
            SetEcho, SetRS232Settings,
        },
        wifi::{
            types::{PowerSaveMode, WifiConfig as WifiConfigParam},
            SetWifiConfig,
        },
        OnOff, AT,
    },
    config::Transport,
    error::Error,
    WifiConfig, DEFAULT_BAUD_RATE,
};

use crate::asynch::OnDrop;

use atat::{
    asynch::{AtatClient as _, SimpleClient},
    AtatIngress as _, UrcChannel,
};
use embassy_futures::select::Either;
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Channel};
use embassy_time::{Duration, Timer};
use embedded_io_async::{BufRead, Write};

#[cfg(feature = "ppp")]
pub(crate) const URC_SUBSCRIBERS: usize = 2;
#[cfg(feature = "ppp")]
type Digester = atat::AtDigester<UbloxUrc>;

#[cfg(feature = "internal-network-stack")]
pub(crate) const URC_SUBSCRIBERS: usize = 3;
#[cfg(feature = "internal-network-stack")]
type Digester = crate::command::custom_digest::EdmDigester;

pub(crate) const MAX_CMD_LEN: usize = 256;

async fn at_bridge<'a, const INGRESS_BUF_SIZE: usize, const URC_CAPACITY: usize>(
    transport: &mut impl Transport,
    req_slot: &Channel<NoopRawMutex, heapless::Vec<u8, MAX_CMD_LEN>, 1>,
    ingress: &mut atat::Ingress<
        'a,
        Digester,
        UbloxUrc,
        INGRESS_BUF_SIZE,
        URC_CAPACITY,
        { URC_SUBSCRIBERS },
    >,
) -> ! {
    ingress.clear();

    let (mut tx, rx) = transport.split_ref();

    let tx_fut = async {
        loop {
            let msg = req_slot.receive().await;
            let _ = tx.write_all(&msg).await;
        }
    };

    embassy_futures::join::join(tx_fut, ingress.read_from(rx)).await;

    unreachable!()
}

/// Background runner for the Ublox Module.
///
/// You must call `.run()` in a background task for the Ublox Module to operate.
pub struct Runner<'a, T: Transport, C, const INGRESS_BUF_SIZE: usize, const URC_CAPACITY: usize> {
    transport: T,

    ch: state::Runner<'a>,
    config: C,

    pub urc_channel: &'a UrcChannel<UbloxUrc, URC_CAPACITY, { URC_SUBSCRIBERS }>,

    pub ingress:
        atat::Ingress<'a, Digester, UbloxUrc, INGRESS_BUF_SIZE, URC_CAPACITY, { URC_SUBSCRIBERS }>,
    pub res_slot: &'a atat::ResponseSlot<INGRESS_BUF_SIZE>,
    pub req_slot: &'a Channel<NoopRawMutex, heapless::Vec<u8, MAX_CMD_LEN>, 1>,

    #[cfg(feature = "ppp")]
    ppp_runner: Option<embassy_net_ppp::Runner<'a>>,
}

impl<'a, T, C, const INGRESS_BUF_SIZE: usize, const URC_CAPACITY: usize>
    Runner<'a, T, C, INGRESS_BUF_SIZE, URC_CAPACITY>
where
    T: Transport + BufRead,
    C: WifiConfig<'a> + 'a,
{
    pub fn new(
        transport: T,
        resources: &'a mut Resources<INGRESS_BUF_SIZE, URC_CAPACITY>,
        config: C,
    ) -> (Self, Control<'a, INGRESS_BUF_SIZE, URC_CAPACITY>) {
        let ch_runner = state::Runner::new(&mut resources.ch);

        let ingress = atat::Ingress::new(
            Digester::new(),
            &mut resources.ingress_buf,
            &resources.res_slot,
            &resources.urc_channel,
        );

        let control = Control::new(
            ch_runner.clone(),
            &resources.urc_channel,
            resources.req_slot.sender(),
            &resources.res_slot,
        );

        (
            Self {
                transport,

                ch: ch_runner,
                config,
                urc_channel: &resources.urc_channel,

                ingress,
                res_slot: &resources.res_slot,
                req_slot: &resources.req_slot,

                #[cfg(feature = "ppp")]
                ppp_runner: None,
            },
            control,
        )
    }

    #[cfg(feature = "ppp")]
    pub fn ppp_stack<'d: 'a, const N_RX: usize, const N_TX: usize>(
        &mut self,
        ppp_state: &'d mut embassy_net_ppp::State<N_RX, N_TX>,
    ) -> embassy_net_ppp::Device<'d> {
        let (net_device, ppp_runner) = embassy_net_ppp::new(ppp_state);
        self.ppp_runner.replace(ppp_runner);
        net_device
    }

    #[cfg(feature = "internal-network-stack")]
    pub fn internal_stack(
        &mut self,
    ) -> super::ublox_stack::Device<'a, INGRESS_BUF_SIZE, URC_CAPACITY> {
        super::ublox_stack::Device {
            state_ch: self.ch.clone(),
            at_client: core::cell::RefCell::new(ProxyClient::new(
                self.req_slot.sender(),
                &self.res_slot,
            )),
            urc_channel: &self.urc_channel,
        }
    }

    /// Probe a given baudrate with the goal of establishing initial
    /// communication with the module, so we can reconfigure it for desired
    /// baudrate
    async fn probe_baud(&mut self, baudrate: BaudRate) -> Result<(), Error> {
        info!("Probing wifi module using baud rate: {}", baudrate as u32);
        self.transport.set_baudrate(baudrate as u32);

        let baud_fut = async {
            let at_client = ProxyClient::new(self.req_slot.sender(), self.res_slot);

            // Hard reset module
            NetDevice::new(&self.ch, &mut self.config, &at_client, self.urc_channel)
                .reset()
                .await?;

            (&at_client).send_retry(&AT).await?;

            // Lets take a shortcut if we are probing for the desired baudrate
            if baudrate == C::BAUD_RATE {
                info!("Successfully shortcut the baud probing!");
                return Ok(None);
            }

            let flow_control = if C::FLOW_CONTROL {
                FlowControl::On
            } else {
                FlowControl::Off
            };

            (&at_client)
                .send_retry(&SetRS232Settings {
                    baud_rate: C::BAUD_RATE,
                    flow_control,
                    data_bits: 8,
                    stop_bits: StopBits::One,
                    parity: Parity::None,
                    change_after_confirm: ChangeAfterConfirm::ChangeAfterOK,
                })
                .await?;

            Ok::<_, Error>(Some(C::BAUD_RATE))
        };

        match embassy_futures::select::select(
            baud_fut,
            at_bridge(&mut self.transport, self.req_slot, &mut self.ingress),
        )
        .await
        {
            Either::First(Ok(Some(baud))) => {
                self.transport.set_baudrate(baud as u32);
                Timer::after_millis(40).await;
                Ok(())
            }
            Either::First(r) => r.map(drop),
        }
    }

    async fn init(&mut self) -> Result<(), Error> {
        // Initialize a new ublox device to a known state
        debug!("Initializing WiFi module");

        // Probe all possible baudrates with the goal of establishing initial
        // communication with the module, so we can reconfigure it for desired
        // baudrate.
        //
        // Start with the two most likely
        let mut found_baudrate = false;

        for baudrate in [
            C::BAUD_RATE,
            DEFAULT_BAUD_RATE,
            BaudRate::B9600,
            BaudRate::B14400,
            BaudRate::B19200,
            BaudRate::B28800,
            BaudRate::B38400,
            BaudRate::B57600,
            BaudRate::B76800,
            BaudRate::B115200,
            BaudRate::B230400,
            BaudRate::B250000,
            BaudRate::B460800,
            BaudRate::B921600,
            BaudRate::B3000000,
            BaudRate::B5250000,
        ] {
            if self.probe_baud(baudrate).await.is_ok() {
                if baudrate != C::BAUD_RATE {
                    // Attempt to store the desired baudrate, so we can shortcut
                    // this probing next time. Ignore any potential failures, as
                    // this is purely an optimization.
                    let _ = embassy_futures::select::select(
                        NetDevice::new(
                            &self.ch,
                            &mut self.config,
                            &ProxyClient::new(self.req_slot.sender(), self.res_slot),
                            self.urc_channel,
                        )
                        .restart(true),
                        at_bridge(&mut self.transport, self.req_slot, &mut self.ingress),
                    )
                    .await;
                }
                found_baudrate = true;
                break;
            }
        }

        if !found_baudrate {
            return Err(Error::BaudDetection);
        }

        let at_client = ProxyClient::new(self.req_slot.sender(), self.res_slot);

        let setup_fut = async {
            (&at_client).send_retry(&SoftwareVersion).await?;

            (&at_client)
                .send_retry(&SetEcho { on: EchoOn::Off })
                .await?;
            (&at_client)
                .send_retry(&SetWifiConfig {
                    config_param: WifiConfigParam::DropNetworkOnLinkLoss(OnOff::On),
                })
                .await?;

            // Disable all power savings for now
            (&at_client)
                .send_retry(&SetWifiConfig {
                    config_param: WifiConfigParam::PowerSaveMode(PowerSaveMode::ActiveMode),
                })
                .await?;

            #[cfg(feature = "internal-network-stack")]
            if let Some(size) = C::TLS_IN_BUFFER_SIZE {
                (&at_client)
                .send_retry(&crate::command::data_mode::SetPeerConfiguration {
                    parameter: crate::command::data_mode::types::PeerConfigParameter::TlsInBuffer(
                        size,
                    ),
                })
                .await?;
            }

            #[cfg(feature = "internal-network-stack")]
            if let Some(size) = C::TLS_OUT_BUFFER_SIZE {
                (&at_client)
                    .send_retry(&crate::command::data_mode::SetPeerConfiguration {
                        parameter:
                            crate::command::data_mode::types::PeerConfigParameter::TlsOutBuffer(
                                size,
                            ),
                    })
                    .await?;
            }

            Ok::<(), Error>(())
        };

        match embassy_futures::select::select(
            setup_fut,
            at_bridge(&mut self.transport, self.req_slot, &mut self.ingress),
        )
        .await
        {
            Either::First(r) => r?,
        }

        self.ch.mark_initialized();

        Ok(())
    }

    #[cfg(feature = "internal-network-stack")]
    pub async fn run(&mut self) -> ! {
        loop {
            if self.init().await.is_err() {
                continue;
            }

            embassy_futures::select::select(
                NetDevice::new(
                    &self.ch,
                    &mut self.config,
                    &ProxyClient::new(self.req_slot.sender(), &self.res_slot),
                    self.urc_channel,
                )
                .run(),
                at_bridge(&mut self.transport, &self.req_slot, &mut self.ingress),
            )
            .await;
        }
    }

    #[cfg(feature = "ppp")]
    pub async fn run(&mut self, stack: embassy_net::Stack<'_>) -> ! {
        loop {
            if self.init().await.is_err() {
                continue;
            }

            debug!("Done initializing WiFi module");

            let network_fut = async {
                // Allow control to send/receive AT commands directly on the
                // UART, until we are ready to establish connection using PPP
                let _ = embassy_futures::select::select(
                    at_bridge(&mut self.transport, self.req_slot, &mut self.ingress),
                    self.ch.wait_connected(),
                )
                .await;

                #[cfg(feature = "ppp")]
                let ppp_fut = async {
                    self.ch.wait_for_link_state(state::LinkState::Up).await;

                    {
                        let mut buf = [0u8; 8];
                        let mut at_client = SimpleClient::new(
                            &mut self.transport,
                            atat::AtDigester::<UbloxUrc>::new(),
                            &mut buf,
                            C::AT_CONFIG,
                        );

                        // Send AT command `ATO3` to enter PPP mode
                        let res = at_client
                            .send_retry(&ChangeMode {
                                mode: data_mode::types::Mode::PPPMode,
                            })
                            .await;

                        if let Err(e) = res {
                            warn!("ppp dial failed {:?}", e);
                            return;
                        }

                        // Drain the UART
                        let _ = embassy_time::with_timeout(Duration::from_millis(500), async {
                            loop {
                                self.transport.read(&mut buf).await.ok();
                            }
                        })
                        .await;
                    }

                    let ondrop = OnDrop::new(|| {
                        warn!("ppp connection dropped");
                        // Set stack to None might not be needed, but it will just be set again
                        // when we get a new connection
                        stack.set_config_v4(embassy_net::ConfigV4::None);
                    });

                    info!("RUNNING PPP");
                    let _ = self
                        .ppp_runner
                        .as_mut()
                        .unwrap()
                        .run(&mut self.transport, C::PPP_CONFIG, |ipv4| {
                            debug!("Running on_ipv4_up for wifi!");
                            let Some(addr) = ipv4.address else {
                                warn!("PPP did not provide an IP address.");
                                return;
                            };
                            let mut dns_servers = heapless::Vec::new();
                            for s in ipv4.dns_servers.iter().flatten() {
                                let _ = dns_servers.push(s.clone());
                            }
                            let config =
                                embassy_net::ConfigV4::Static(embassy_net::StaticConfigV4 {
                                    address: embassy_net::Ipv4Cidr::new(addr, 0),
                                    gateway: None,
                                    dns_servers,
                                });

                            stack.set_config_v4(config);
                        })
                        .await;
                    error!("ppp connection returned");
                    drop(ondrop);

                    info!("ppp failed");
                };

                let at_fut = async {
                    use crate::asynch::at_udp_socket::AtUdpSocket;
                    use embassy_net::udp::{PacketMetadata, UdpSocket};

                    let mut rx_meta = [PacketMetadata::EMPTY; 1];
                    let mut tx_meta = [PacketMetadata::EMPTY; 1];
                    let mut socket_rx_buf = [0u8; 64];
                    let mut socket_tx_buf = [0u8; 64];
                    let mut socket = UdpSocket::new(
                        stack,
                        &mut rx_meta,
                        &mut socket_rx_buf,
                        &mut tx_meta,
                        &mut socket_tx_buf,
                    );

                    socket.bind(AtUdpSocket::PPP_AT_PORT).unwrap();
                    let mut at_socket = AtUdpSocket(socket);

                    at_bridge(&mut at_socket, self.req_slot, &mut self.ingress).await;
                };

                embassy_futures::select::select(ppp_fut, at_fut).await;
            };

            let device_fut = async {
                let _ = NetDevice::new(
                    &self.ch,
                    &mut self.config,
                    &ProxyClient::new(self.req_slot.sender(), self.res_slot),
                    self.urc_channel,
                )
                .run()
                .await;

                warn!("Breaking to reboot device");
            };

            embassy_futures::select::select(device_fut, network_fut).await;
        }
    }
}
