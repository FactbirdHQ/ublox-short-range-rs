use core::mem::MaybeUninit;

use atat::{
    asynch::{AtatClient, Client, SimpleClient},
    AtatIngress,
};
use embassy_futures::select::Either;
use embassy_net::{
    udp::{PacketMetadata, UdpSocket},
    IpEndpoint, Ipv4Address,
};
use embassy_sync::{
    blocking_mutex::raw::NoopRawMutex,
    mutex::Mutex,
    pipe::{Reader, Writer},
};
use embassy_time::{Duration, Instant, Timer};
use embedded_hal::digital::OutputPin;
use embedded_io_async::{BufRead, Read, Write};

use crate::command::{
    data_mode::{self, ChangeMode},
    system::{self, SetEcho},
};

use super::{control::Control, resources::UbxResources, runner::Runner, state, AtHandle, UbloxUrc};

const PPP_AT_PORT: u16 = 23;
pub const SOCKET_BUF_SIZE: usize = 128;

pub type Resources<
    'a,
    const CMD_BUF_SIZE: usize,
    const INGRESS_BUF_SIZE: usize,
    const URC_CAPACITY: usize,
> = UbxResources<
    Writer<'a, NoopRawMutex, SOCKET_BUF_SIZE>,
    CMD_BUF_SIZE,
    INGRESS_BUF_SIZE,
    URC_CAPACITY,
>;

pub fn new_ppp<
    'a,
    RST: OutputPin,
    const CMD_BUF_SIZE: usize,
    const INGRESS_BUF_SIZE: usize,
    const URC_CAPACITY: usize,
>(
    resources: &'a mut Resources<'a, CMD_BUF_SIZE, INGRESS_BUF_SIZE, URC_CAPACITY>,
    reset: RST,
) -> (
    embassy_net_ppp::Device<'a>,
    Control<'a, Client<'a, Writer<'a, NoopRawMutex, SOCKET_BUF_SIZE>, INGRESS_BUF_SIZE>>,
    PPPRunner<'a, RST, INGRESS_BUF_SIZE, URC_CAPACITY>,
) {
    let ch_runner = state::new_ppp(&mut resources.ch);
    let state_ch = ch_runner.state_runner();

    let (control_rx_reader, control_rx_writer) = resources.control_rx.split();
    let (control_tx_reader, control_tx_writer) = resources.control_tx.split();

    // safety: this is a self-referential struct, however:
    // - it can't move while the `'a` borrow is active.
    // - when the borrow ends, the dangling references inside the MaybeUninit will never be used again.
    let at_client_uninit: *mut MaybeUninit<
        Mutex<
            NoopRawMutex,
            Client<'a, Writer<'a, NoopRawMutex, SOCKET_BUF_SIZE>, INGRESS_BUF_SIZE>,
        >,
    > = (&mut resources.at_client
        as *mut MaybeUninit<
            Mutex<
                NoopRawMutex,
                Client<'static, Writer<'a, NoopRawMutex, SOCKET_BUF_SIZE>, INGRESS_BUF_SIZE>,
            >,
        >)
        .cast();

    unsafe { &mut *at_client_uninit }.write(Mutex::new(Client::new(
        control_tx_writer,
        &resources.res_slot,
        &mut resources.cmd_buf,
        atat::Config::default(),
    )));

    let at_client = unsafe { (&*at_client_uninit).assume_init_ref() };

    let wifi_runner = Runner::new(
        ch_runner,
        AtHandle(at_client),
        reset,
        resources.urc_channel.subscribe().unwrap(),
    );

    let ingress = atat::Ingress::new(
        atat::AtDigester::<UbloxUrc>::new(),
        &mut resources.ingress_buf,
        &resources.res_slot,
        &resources.urc_channel,
    );

    let control = Control::new(state_ch, AtHandle(at_client));

    let (net_device, ppp_runner) = embassy_net_ppp::new(&mut resources.ppp_state);

    let runner = PPPRunner {
        ppp_runner,
        wifi_runner,
        ingress,
        control_rx_reader,
        control_rx_writer,
        control_tx_reader,
    };

    (net_device, control, runner)
}

pub struct PPPRunner<'a, RST: OutputPin, const INGRESS_BUF_SIZE: usize, const URC_CAPACITY: usize> {
    pub ppp_runner: embassy_net_ppp::Runner<'a>,
    pub wifi_runner: Runner<
        'a,
        Client<'a, Writer<'a, NoopRawMutex, SOCKET_BUF_SIZE>, INGRESS_BUF_SIZE>,
        RST,
        URC_CAPACITY,
    >,
    pub ingress:
        atat::Ingress<'a, atat::AtDigester<UbloxUrc>, UbloxUrc, INGRESS_BUF_SIZE, URC_CAPACITY, 2>,
    pub control_rx_reader: Reader<'a, NoopRawMutex, SOCKET_BUF_SIZE>,
    pub control_rx_writer: Writer<'a, NoopRawMutex, SOCKET_BUF_SIZE>,
    pub control_tx_reader: Reader<'a, NoopRawMutex, SOCKET_BUF_SIZE>,
}

impl<'a, RST: OutputPin, const INGRESS_BUF_SIZE: usize, const URC_CAPACITY: usize>
    PPPRunner<'a, RST, INGRESS_BUF_SIZE, URC_CAPACITY>
{
    async fn configure<A: AtatClient>(at_client: &mut A) -> Result<(), atat::Error> {
        let _ = at_client
            .send(&ChangeMode {
                mode: data_mode::types::Mode::CommandMode,
            })
            .await;

        at_client
            .send(&SetEcho {
                on: system::types::EchoOn::Off,
            })
            .await?;

        // Initialize `ublox` module to desired baudrate
        at_client
            .send(&system::SetRS232Settings {
                baud_rate: system::types::BaudRate::B115200,
                flow_control: system::types::FlowControl::On,
                data_bits: 8,
                stop_bits: system::types::StopBits::One,
                parity: system::types::Parity::None,
                change_after_confirm: system::types::ChangeAfterConfirm::ChangeAfterOK,
            })
            .await?;

        Ok(())
    }

    pub async fn run<RW: BufRead + Read + Write>(
        &mut self,
        mut iface: RW,
        stack: &embassy_net::Stack<embassy_net_ppp::Device<'a>>,
    ) -> ! {
        // self.wifi_runner.init().await.unwrap();
        // Timer::after(Duration::from_secs(4)).await;

        loop {
            // Reset modem
            self.wifi_runner.reset().await;

            Timer::after(Duration::from_secs(1)).await;

            let control_fut = async {
                stack.wait_config_up().await;

                let mut rx_meta = [PacketMetadata::EMPTY; 1];
                let mut tx_meta = [PacketMetadata::EMPTY; 1];
                let mut socket_rx_buf = [0u8; 32];
                let mut socket_tx_buf = [0u8; 32];
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
                    .bind((endpoint.address.address(), PPP_AT_PORT))
                    .unwrap();

                let mut tx_buf = [0u8; 32];
                let mut rx_buf = [0u8; 32];

                loop {
                    match embassy_futures::select::select(
                        self.control_tx_reader.read(&mut tx_buf),
                        socket.recv_from(&mut rx_buf),
                    )
                    .await
                    {
                        Either::First(n) => {
                            socket
                                .send_to(
                                    &tx_buf[..n],
                                    (Ipv4Address::new(172, 30, 0, 251), PPP_AT_PORT),
                                )
                                .await
                                .unwrap();
                        }
                        Either::Second(Ok((n, _))) => {
                            self.control_rx_writer
                                .write_all(&rx_buf[..n])
                                .await
                                .unwrap();
                        }
                        Either::Second(_) => {}
                    }
                }
            };

            let ppp_fut = async {
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

                    let mut buf = [0u8; 64];
                    let mut at_client = SimpleClient::new(
                        &mut iface,
                        atat::AtDigester::<UbloxUrc>::new(),
                        &mut buf,
                        atat::Config::default(),
                    );

                    if let Err(e) = Self::configure(&mut at_client).await {
                        warn!("modem: configure failed {:?}", e);
                        continue;
                    }

                    Timer::after(Duration::from_secs(2)).await;

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

                    info!("RUNNING PPP");
                    let config = embassy_net_ppp::Config {
                        username: b"",
                        password: b"",
                    };
                    let res = self
                        .ppp_runner
                        .run(&mut iface, config, |ipv4| {
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

                    info!("ppp failed");
                }
            };

            let ingress_fut = async {
                self.ingress.read_from(&mut self.control_rx_reader).await;
            };

            embassy_futures::select::select4(
                ppp_fut,
                ingress_fut,
                control_fut,
                self.wifi_runner.run(),
            )
            .await;
        }
    }
}
