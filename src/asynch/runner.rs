use super::{
    control::Control,
    network::NetDevice,
    state::{self, LinkState},
    Resources, UbloxUrc,
};
use crate::{
    asynch::control::ProxyClient,
    command::data_mode::{self, ChangeMode},
    WifiConfig,
};
use atat::{
    asynch::{AtatClient, SimpleClient},
    AtatIngress as _, UrcChannel,
};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Channel};
use embassy_time::{Duration, Timer};
use embedded_io_async::{BufRead, Read, Write};

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
    mut sink: impl Write,
    source: impl Read,
    req_slot: &Channel<NoopRawMutex, heapless::Vec<u8, MAX_CMD_LEN>, 1>,
    ingress: &mut atat::Ingress<
        'a,
        Digester,
        UbloxUrc,
        INGRESS_BUF_SIZE,
        URC_CAPACITY,
        URC_SUBSCRIBERS,
    >,
) -> ! {
    ingress.clear();

    let tx_fut = async {
        loop {
            let msg = req_slot.receive().await;
            let _ = sink.write_all(&msg).await;
        }
    };

    embassy_futures::join::join(tx_fut, ingress.read_from(source)).await;

    unreachable!()
}

/// Background runner for the Ublox Module.
///
/// You must call `.run()` in a background task for the Ublox Module to operate.
pub struct Runner<'a, R, W, C, const INGRESS_BUF_SIZE: usize, const URC_CAPACITY: usize> {
    iface: (R, W),

    ch: state::Runner<'a>,
    config: C,

    pub urc_channel: &'a UrcChannel<UbloxUrc, URC_CAPACITY, URC_SUBSCRIBERS>,

    pub ingress:
        atat::Ingress<'a, Digester, UbloxUrc, INGRESS_BUF_SIZE, URC_CAPACITY, URC_SUBSCRIBERS>,
    pub res_slot: &'a atat::ResponseSlot<INGRESS_BUF_SIZE>,
    pub req_slot: &'a Channel<NoopRawMutex, heapless::Vec<u8, MAX_CMD_LEN>, 1>,

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
    pub fn new(
        iface: (R, W),
        resources: &'a mut Resources<INGRESS_BUF_SIZE, URC_CAPACITY>,
        config: C,
    ) -> Self {
        let ch_runner = state::Runner::new(&mut resources.ch);

        let ingress = atat::Ingress::new(
            Digester::new(),
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
            res_slot: &resources.res_slot,
            req_slot: &resources.req_slot,

            #[cfg(feature = "ppp")]
            ppp_runner: None,
        }
    }

    pub fn control(&self) -> Control<'a, INGRESS_BUF_SIZE, URC_CAPACITY> {
        Control::new(
            self.ch.clone(),
            &self.urc_channel,
            self.req_slot.sender(),
            &self.res_slot,
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

    #[cfg(feature = "internal-network-stack")]
    pub async fn run(&mut self) -> ! {
        let device_fut = async {
            loop {
                let mut device = NetDevice::new(
                    &self.ch,
                    &mut self.config,
                    ProxyClient::new(self.req_slot.sender(), &self.res_slot),
                    self.urc_channel,
                );

                if let Err(e) = device.init().await {
                    error!("WiFi init failed {:?}", e);
                    continue;
                };

                let _ = device.run().await;
            }
        };

        embassy_futures::join::join(
            device_fut,
            at_bridge(
                &mut self.iface.1,
                &mut self.iface.0,
                &self.req_slot,
                &mut self.ingress,
            ),
        )
        .await;

        unreachable!()
    }

    #[cfg(feature = "ppp")]
    pub async fn run<D: embassy_net::driver::Driver>(
        &mut self,
        stack: &embassy_net::Stack<D>,
    ) -> ! {
        let at_config = atat::Config::default();

        loop {
            let network_fut = async {
                // Allow control to send/receive AT commands directly on the
                // UART, until we are ready to establish connection using PPP

                // Send "+++" to escape data mode, and enter command mode
                warn!("Escaping to command mode!");
                Timer::after_secs(1).await;
                self.iface.1.write_all(b"+++").await.ok();
                Timer::after_secs(1).await;

                let _ = embassy_futures::select::select(
                    at_bridge(
                        &mut self.iface.1,
                        &mut self.iface.0,
                        &self.req_slot,
                        &mut self.ingress,
                    ),
                    self.ch.wait_for_link_state(LinkState::Up),
                )
                .await;

                #[cfg(feature = "ppp")]
                let ppp_fut = async {
                    let mut iface = super::ReadWriteAdapter(&mut self.iface.0, &mut self.iface.1);

                    self.ch.wait_for_link_state(LinkState::Up).await;

                    {
                        let mut buf = [0u8; 8];
                        let mut at_client = SimpleClient::new(
                            &mut iface,
                            atat::AtDigester::<UbloxUrc>::new(),
                            &mut buf,
                            at_config,
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
                                iface.read(&mut buf).await.ok();
                            }
                        })
                        .await;
                    }

                    info!("RUNNING PPP");
                    let res = self
                        .ppp_runner
                        .as_mut()
                        .unwrap()
                        .run(&mut iface, C::PPP_CONFIG, |ipv4| {
                            debug!("Running on_ipv4_up for wifi!");
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
                    let at_socket = AtUdpSocket(socket);

                    at_bridge(&at_socket, &at_socket, &self.req_slot, &mut self.ingress).await;
                };

                embassy_futures::select::select(ppp_fut, at_fut).await;
            };

            let device_fut = async {
                let at_client = ProxyClient::new(self.req_slot.sender(), &self.res_slot);
                let mut device =
                    NetDevice::new(&self.ch, &mut self.config, &at_client, self.urc_channel);

                if let Err(e) = device.init().await {
                    error!("WiFi init failed {:?}", e);
                    return;
                };

                let _ = device.run().await;

                warn!("Breaking to reboot device");
            };

            embassy_futures::select::select(device_fut, network_fut).await;
        }
    }
}
