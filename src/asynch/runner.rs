use super::{control::Control, network::NetDevice, state, Resources, UbloxUrc};
use crate::{
    asynch::control::ProxyClient,
    command::{
        general::SoftwareVersion,
        system::{
            types::{
                BaudRate, ChangeAfterConfirm, EchoOn, FlowControl, ModuleStartMode, Parity,
                StopBits,
            },
            ModuleStart, SetEcho, SetRS232Settings,
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

#[cfg(feature = "ppp")]
use crate::asynch::OnDrop;
#[cfg(feature = "ppp")]
use crate::command::data_mode::{self, ChangeMode};

#[cfg(feature = "ppp")]
use atat::asynch::SimpleClient;
use atat::{asynch::AtatClient as _, AtatIngress as _, UrcChannel};
use embassy_futures::select::Either;
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Channel};
use embassy_time::{Duration, Timer};
use embedded_io_async::{BufRead, Read as _, Write};

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
            warn!("WiFi module did not answer at any baud rate in the table");
            return Err(Error::BaudDetection);
        }

        let at_client = ProxyClient::new(self.req_slot.sender(), self.res_slot);

        let setup_fut = async {
            (&at_client).send_retry(&SoftwareVersion).await?;

            (&at_client)
                .send_retry(&SetEcho { on: EchoOn::Off })
                .await?;

            // Pin the start mode to command mode. `+UMSM` is persistent, and a
            // module that came up in data or PPP mode would answer nothing on
            // the AT interface at any baud rate -- indistinguishable from dead
            // hardware, and unrecoverable here since the reset line is the only
            // control available. Nothing in this driver ever sets it, so its
            // value is whatever was last stored; assert it rather than trust it.
            (&at_client)
                .send_retry(&ModuleStart {
                    mode: ModuleStartMode::CommandMode,
                })
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

    /// Consecutive whole-table probe failures before the module is declared
    /// unreachable.
    ///
    /// A carousel that is going to recover does so within about three passes --
    /// the worst *recovering* case seen in CI was 43 probes, i.e. it failed two
    /// full passes and succeeded partway through the third. Anything past that
    /// has never come back on its own.
    const MUTE_PASSES: u32 = 3;

    /// How long to wait between probe attempts once the module is declared
    /// unreachable.
    ///
    /// Retrying is kept alive rather than abandoned, because the task cannot
    /// return -- but at a cadence that stops it churning the UART and drowning
    /// the log for the rest of the session.
    const MUTE_BACKOFF: Duration = Duration::from_secs(30);

    /// Pause between probe passes before the module is declared unreachable.
    ///
    /// Small, but deliberately never zero: a full pass normally spends ~80s in
    /// `wait_startup`, yet nothing guarantees `init()` awaits on every error
    /// path. Without a yield here, one that returns synchronously would spin
    /// the executor instead of retrying.
    const RETRY_PAUSE: Duration = Duration::from_millis(500);

    /// Log the diagnosis for a failed probe pass.
    ///
    /// Worth being explicit in the log: on this hardware the reset line is the
    /// *only* control the MCU has over the module -- there is no power-enable --
    /// so once it stops answering, no amount of retrying here can recover it.
    /// Saying so turns a silent carousel into a diagnosis.
    fn report_probe_failure(passes: u32) {
        if passes < Self::MUTE_PASSES {
            warn!(
                "WiFi module probe pass {}/{} failed; retrying",
                passes,
                Self::MUTE_PASSES
            );
        } else {
            error!(
                "WiFi module UNRESPONSIVE after {} full baud-table passes. The reset \
                 line is the only control available, and it has not revived the module, \
                 so this cannot be recovered in firmware -- the board needs a power \
                 cycle. Backing off to {}s between attempts.",
                passes,
                Self::MUTE_BACKOFF.as_secs()
            );
        }
    }

    #[cfg(feature = "internal-network-stack")]
    pub async fn run(&mut self) -> ! {
        let mut failed_passes = 0u32;
        loop {
            if self.init().await.is_err() {
                failed_passes += 1;
                Self::report_probe_failure(failed_passes);
                Timer::after(if failed_passes >= Self::MUTE_PASSES {
                    Self::MUTE_BACKOFF
                } else {
                    Self::RETRY_PAUSE
                })
                .await;
                continue;
            }
            failed_passes = 0;

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

    /// End the PPP session so the line is idle before `init()` resets the module.
    ///
    /// Dropping the PPP future stops the *host* driving the link but never tells
    /// the module anything, so it keeps framing HDLC and the reset lands
    /// mid-frame -- CI caught one 0.7ms after PPP bytes were still arriving,
    /// after which the module went silent across 35 resets and two full
    /// baud-table passes.
    ///
    /// This does **not** return the module to command mode, and nothing can:
    /// the manual lists only the escape sequence and a DTR transition, DTR is
    /// not wired on this hardware, and the ODIN-W2 ignores `+++` in PPP mode
    /// (measured -- with the session terminated and the line quiet for 1.2s it
    /// still answers nothing, and a following `AT` gets no reply). The reset is
    /// genuinely the only way out. What this buys is that the reset arrives
    /// while the module is idle rather than mid-transmission.
    ///
    /// `ppproto` can only answer a peer's Terminate-Request, never initiate one
    /// (`_send_terminate_request` is dead code), so the frame is emitted
    /// directly. It is fixed apart from the identifier:
    ///
    ///   7E FF 03 C0 21 05 01 00 04 3D C7 7E
    ///   |  |     |     |  |  |     |     `- flag
    ///   |  |     |     |  |  |     `------- FCS-16
    ///   |  |     |     |  |  `------------- length 4
    ///   |  |     |     |  `---------------- identifier
    ///   |  |     |     `------------------- code 5 = Terminate-Request
    ///   |  |     `------------------------- protocol C021 = LCP
    ///   |  `------------------------------- address / control
    ///   `---------------------------------- flag
    ///
    /// The module answers Terminate-Ack and stops transmitting. Best-effort: if
    /// it is already quiet or already wedged this is inert, and we reset anyway.
    #[cfg(feature = "ppp")]
    async fn terminate_ppp(&mut self) {
        const LCP_TERMINATE_REQ: [u8; 12] = [
            0x7E, 0xFF, 0x03, 0xC0, 0x21, 0x05, 0x01, 0x00, 0x04, 0x3D, 0xC7, 0x7E,
        ];
        /// Cap on the drain, so a module that never stops talking cannot hold
        /// the teardown open.
        const MAX_DRAIN_READS: usize = 12;

        debug!("Terminating PPP session so the module is idle before reset");
        let (mut tx, mut rx) = self.transport.split_ref();
        let _ = tx.write_all(&LCP_TERMINATE_REQ).await;

        // Drain until the line goes quiet -- that quiet is the whole point.
        let mut buf = [0u8; 32];
        let mut drained = 0usize;
        for _ in 0..MAX_DRAIN_READS {
            match embassy_time::with_timeout(Duration::from_millis(250), rx.read(&mut buf)).await {
                Ok(Ok(n)) if n > 0 => drained += n,
                _ => break,
            }
        }
        debug!("PPP terminated, drained {} bytes; line idle", drained);
    }

    #[cfg(feature = "ppp")]
    pub async fn run(&mut self, stack: embassy_net::Stack<'_>) -> ! {
        let mut failed_passes = 0u32;
        loop {
            if self.init().await.is_err() {
                failed_passes += 1;
                Self::report_probe_failure(failed_passes);
                Timer::after(if failed_passes >= Self::MUTE_PASSES {
                    Self::MUTE_BACKOFF
                } else {
                    Self::RETRY_PAUSE
                })
                .await;
                continue;
            }
            failed_passes = 0;

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
                        self.ch.update_connection_with(|con| {
                            con.reset();
                        });
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
                            // IPCP only hands out the module's own DNS proxy
                            // (its PPP-side address), which on some networks
                            // never answers while the DHCP servers behind it
                            // do. Try those first, the proxy last.
                            let dhcp = self.ch.dns_servers();
                            let mut dns_servers = heapless::Vec::new();
                            for s in [dhcp.primary, dhcp.secondary]
                                .into_iter()
                                .chain(ipv4.dns_servers)
                                .flatten()
                            {
                                if !dns_servers.contains(&s) {
                                    let _ = dns_servers.push(s);
                                }
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

            // The session is over and the PPP future has been dropped, so the
            // host has stopped driving the link -- but the module has not been
            // told. End the session properly before `init()` resets it.
            self.terminate_ppp().await;
        }
    }
}
