use super::{
    control::{Control, ControlResources},
    network::NetDevice,
    state, Resources, UbloxUrc,
};
#[cfg(feature = "edm")]
use crate::command::edm::SwitchToEdmCommand;
use crate::{
    asynch::at_udp_socket::AtUdpSocket,
    command::{
        data_mode::{self, ChangeMode},
        Urc,
    },
    WifiConfig,
};
use atat::{
    asynch::{AtatClient, SimpleClient},
    AtatIngress as _, UrcChannel,
};
use embassy_futures::select::Either;
use embassy_net::{
    udp::{PacketMetadata, UdpSocket},
    Ipv4Address,
};
use embassy_time::{Duration, Instant, Timer};
use embedded_io_async::{BufRead, Read, Write};

pub(crate) const URC_SUBSCRIBERS: usize = 3;

/// Background runner for the Ublox Module.
///
/// You must call `.run()` in a background task for the Ublox Module to operate.
pub struct Runner<'a, R, W, C, const INGRESS_BUF_SIZE: usize, const URC_CAPACITY: usize> {
    iface: (R, W),

    ch: state::Runner<'a>,
    config: C,

    pub urc_channel: &'a UrcChannel<Urc, URC_CAPACITY, URC_SUBSCRIBERS>,

    pub ingress: atat::Ingress<
        'a,
        atat::AtDigester<Urc>,
        Urc,
        INGRESS_BUF_SIZE,
        URC_CAPACITY,
        URC_SUBSCRIBERS,
    >,
    pub cmd_buf: &'a mut [u8],
    pub res_slot: &'a atat::ResponseSlot<INGRESS_BUF_SIZE>,

    #[cfg(feature = "ppp")]
    ppp_runner: Option<embassy_net_ppp::Runner<'a>>,
}

impl<'a, R, W, C, const INGRESS_BUF_SIZE: usize, const URC_CAPACITY: usize>
    Runner<'a, R, W, C, INGRESS_BUF_SIZE, URC_CAPACITY>
where
    R: BufRead + Read,
    W: Write,
    C: WifiConfig<'a> + 'a,
{
    pub fn new<const CMD_BUF_SIZE: usize>(
        iface: (R, W),
        resources: &'a mut Resources<CMD_BUF_SIZE, INGRESS_BUF_SIZE, URC_CAPACITY>,
        config: C,
    ) -> Self {
        let ch_runner = state::Runner::new(&mut resources.ch);

        let ingress = atat::Ingress::new(
            atat::AtDigester::new(),
            &mut resources.ingress_buf,
            &resources.res_slot,
            &resources.urc_channel,
        );

        Self {
            iface,

            ch: ch_runner,
            config,
            urc_channel: &resources.urc_channel,

            ingress,
            cmd_buf: &mut resources.cmd_buf,
            res_slot: &resources.res_slot,

            #[cfg(feature = "ppp")]
            ppp_runner: None,
        }
    }

    pub fn control<'r, D: embassy_net::driver::Driver>(
        &self,
        resources: &'r mut ControlResources,
        stack: &'r embassy_net::Stack<D>,
    ) -> Control<'a, 'r, URC_CAPACITY> {
        Control::new(self.ch.clone(), &self.urc_channel, resources, stack)
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
    pub fn internal_stack(&mut self) -> state::Device<URC_CAPACITY> {
        state::Device {
            shared: &self.ch.shared,
            urc_subscription: self.urc_channel.subscribe().unwrap(),
        }
    }

    pub async fn run<D: embassy_net::driver::Driver>(mut self, stack: &embassy_net::Stack<D>) -> ! {
        #[cfg(feature = "ppp")]
        let mut ppp_runner = self.ppp_runner.take().unwrap();

        let at_config = atat::Config::default();
        loop {
            // Run the cellular device from full power down to the
            // `DataEstablished` state, handling power on, module configuration,
            // network registration & operator selection and PDP context
            // activation along the way.
            //
            // This is all done directly on the serial line, before setting up
            // virtual channels through multiplexing.
            {
                let at_client = atat::asynch::Client::new(
                    &mut self.iface.1,
                    self.res_slot,
                    self.cmd_buf,
                    at_config,
                );
                let mut wifi_device =
                    NetDevice::new(&self.ch, &mut self.config, at_client, self.urc_channel);

                // Clean up and start from completely powered off state. Ignore URCs in the process.
                self.ingress.clear();

                match embassy_futures::select::select(
                    self.ingress.read_from(&mut self.iface.0),
                    wifi_device.init(),
                )
                .await
                {
                    Either::First(_) => {
                        // This has return type never (`-> !`)
                        unreachable!()
                    }
                    Either::Second(Err(_)) => {
                        // Reboot the wifi module and try again!
                        continue;
                    }
                    Either::Second(Ok(_)) => {
                        // All good! We are now ready to start communication services!
                    }
                }
            }

            #[cfg(feature = "ppp")]
            let ppp_fut = async {
                let mut iface = super::ReadWriteAdapter(&mut self.iface.0, &mut self.iface.1);

                let mut fails = 0;
                let mut last_start = None;

                loop {
                    if let Some(last_start) = last_start {
                        Timer::at(last_start + Duration::from_secs(10)).await;
                        // Do not attempt to start too fast.

                        // If was up stably for at least 1 min, reset fail counter.
                        if Instant::now() > last_start + Duration::from_secs(60) {
                            fails = 0;
                        } else {
                            fails += 1;
                            if fails == 10 {
                                warn!("modem: PPP failed too much, rebooting modem.");
                                break;
                            }
                        }
                    }
                    last_start = Some(Instant::now());

                    {
                        let mut buf = [0u8; 64];

                        let mut at_client = SimpleClient::new(
                            &mut iface,
                            atat::AtDigester::<UbloxUrc>::new(),
                            &mut buf,
                            at_config,
                        );

                        // Send AT command `ATO3` to enter PPP mode
                        let res = at_client
                            .send(&ChangeMode {
                                mode: data_mode::types::Mode::PPPMode,
                            })
                            .await;

                        if let Err(e) = res {
                            warn!("ppp dial failed {:?}", e);
                            continue;
                        }

                        drop(at_client);

                        // Drain the UART
                        let _ = embassy_time::with_timeout(Duration::from_secs(2), async {
                            loop {
                                iface.read(&mut buf).await.ok();
                            }
                        })
                        .await;

                        Timer::after(Duration::from_millis(100)).await;
                    }

                    info!("RUNNING PPP");
                    let res = ppp_runner
                        .run(&mut iface, C::PPP_CONFIG, |ipv4| {
                            let Some(addr) = ipv4.address else {
                                warn!("PPP did not provide an IP address.");
                                return;
                            };
                            let mut dns_servers = heapless::Vec::new();
                            for s in ipv4.dns_servers.iter().flatten() {
                                let _ =
                                    dns_servers.push(embassy_net::Ipv4Address::from_bytes(&s.0));
                            }
                            let config =
                                embassy_net::ConfigV4::Static(embassy_net::StaticConfigV4 {
                                    address: embassy_net::Ipv4Cidr::new(
                                        embassy_net::Ipv4Address::from_bytes(&addr.0),
                                        0,
                                    ),
                                    gateway: None,
                                    dns_servers,
                                });

                            stack.set_config_v4(config);
                        })
                        .await;

                    info!("ppp failed: {:?}", res);
                }
            };

            let network_fut = async {
                stack.wait_config_up().await;

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

                let endpoint = stack.config_v4().unwrap();

                info!("Socket bound!");
                socket
                    .bind((Ipv4Address::new(172, 30, 0, 252), AtUdpSocket::PPP_AT_PORT))
                    .unwrap();

                let at_socket = AtUdpSocket(socket);

                let at_client =
                    atat::asynch::Client::new(&at_socket, self.res_slot, self.cmd_buf, at_config);

                let mut wifi_device =
                    NetDevice::new(&self.ch, &mut self.config, at_client, self.urc_channel);

                embassy_futures::join::join(self.ingress.read_from(&at_socket), wifi_device.run())
                    .await;
            };

            match embassy_futures::select::select(ppp_fut, network_fut).await {
                Either::First(_) => {
                    warn!("Breaking to reboot module from PPP");
                }
                Either::Second(_) => {
                    warn!("Breaking to reboot module from network runner");
                }
            }
        }
    }
}
