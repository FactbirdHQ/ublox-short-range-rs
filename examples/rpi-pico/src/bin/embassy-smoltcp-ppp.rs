#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]

#[cfg(not(feature = "ppp"))]
compile_error!("You must enable the `ppp` feature flag to build this example");

use defmt::*;
use embassy_executor::Spawner;
use embassy_net::tcp::TcpSocket;
use embassy_net::{Ipv4Address, Stack, StackResources};
use embassy_rp::gpio::{AnyPin, Level, Output, OutputOpenDrain, Pin};
use embassy_rp::peripherals::UART1;
use embassy_rp::uart::{BufferedInterruptHandler, BufferedUart, BufferedUartRx, BufferedUartTx};
use embassy_rp::{bind_interrupts, uart};
use embassy_time::{Duration, Timer};
use embedded_tls::TlsConfig;
use embedded_tls::TlsConnection;
use embedded_tls::TlsContext;
use embedded_tls::UnsecureProvider;
use embedded_tls::{Aes128GcmSha256, MaxFragmentLength};
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;
use reqwless::headers::ContentType;
use reqwless::request::Request;
use reqwless::request::RequestBuilder as _;
use reqwless::response::Response;
use static_cell::StaticCell;
use ublox_short_range::asynch::control::ControlResources;
use ublox_short_range::asynch::{Resources, Runner};
use {defmt_rtt as _, panic_probe as _};

const CMD_BUF_SIZE: usize = 128;
const INGRESS_BUF_SIZE: usize = 512;
const URC_CAPACITY: usize = 2;

pub struct WifiConfig {
    pub rst_pin: OutputOpenDrain<'static>,
}

impl<'a> ublox_short_range::WifiConfig<'a> for WifiConfig {
    type ResetPin = OutputOpenDrain<'static>;

    const PPP_CONFIG: embassy_net_ppp::Config<'a> = embassy_net_ppp::Config {
        username: b"",
        password: b"",
    };

    fn reset_pin(&mut self) -> Option<&mut Self::ResetPin> {
        Some(&mut self.rst_pin)
    }
}

#[embassy_executor::task]
async fn net_task(stack: &'static Stack<embassy_net_ppp::Device<'static>>) -> ! {
    stack.run().await
}

#[embassy_executor::task]
async fn ppp_task(
    mut runner: Runner<
        'static,
        BufferedUartRx<'static, UART1>,
        BufferedUartTx<'static, UART1>,
        WifiConfig,
        INGRESS_BUF_SIZE,
        URC_CAPACITY,
    >,
    stack: &'static embassy_net::Stack<embassy_net_ppp::Device<'static>>,
) -> ! {
    runner.run(stack).await
}

bind_interrupts!(struct Irqs {
    UART1_IRQ => BufferedInterruptHandler<UART1>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let rst_pin = OutputOpenDrain::new(p.PIN_26.degrade(), Level::High);

    static TX_BUF: StaticCell<[u8; 32]> = StaticCell::new();
    static RX_BUF: StaticCell<[u8; 32]> = StaticCell::new();
    let wifi_uart = uart::BufferedUart::new_with_rtscts(
        p.UART1,
        Irqs,
        p.PIN_24,
        p.PIN_25,
        p.PIN_23,
        p.PIN_22,
        TX_BUF.init([0; 32]),
        RX_BUF.init([0; 32]),
        uart::Config::default(),
    );

    static RESOURCES: StaticCell<Resources<CMD_BUF_SIZE, INGRESS_BUF_SIZE, URC_CAPACITY>> =
        StaticCell::new();

    let mut runner = Runner::new(
        wifi_uart.split(),
        RESOURCES.init(Resources::new()),
        WifiConfig { rst_pin },
    );

    static PPP_STATE: StaticCell<embassy_net_ppp::State<2, 2>> = StaticCell::new();
    let net_device = runner.ppp_stack(PPP_STATE.init(embassy_net_ppp::State::new()));

    // Generate random seed
    let seed = 0x0123_4567_89ab_cdef; // chosen by fair dice roll. guarenteed to be random.

    // Init network stack
    static STACK: StaticCell<Stack<embassy_net_ppp::Device<'static>>> = StaticCell::new();
    static STACK_RESOURCES: StaticCell<StackResources<6>> = StaticCell::new();

    let stack = &*STACK.init(Stack::new(
        net_device,
        embassy_net::Config::default(),
        STACK_RESOURCES.init(StackResources::new()),
        seed,
    ));

    static CONTROL_RESOURCES: StaticCell<ControlResources> = StaticCell::new();
    let mut control = runner.control(CONTROL_RESOURCES.init(ControlResources::new()), &stack);

    spawner.spawn(net_task(stack)).unwrap();
    spawner.spawn(ppp_task(runner, &stack)).unwrap();

    stack.wait_config_up().await;

    Timer::after(Duration::from_secs(1)).await;

    control.set_hostname("Ublox-wifi-test").await.ok();

    control.join_wpa2("MyAccessPoint", "12345678").await;

    info!("We have network!");

    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];
    let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
    socket.set_timeout(Some(Duration::from_secs(10)));

    let hostname = "ecdsa-test.germancoding.com";

    let mut remote = stack
        .dns_query(hostname, smoltcp::wire::DnsQueryType::A)
        .await
        .unwrap();
    let remote_endpoint = (remote.pop().unwrap(), 443);
    info!("connecting to {:?}...", remote_endpoint);
    let r = socket.connect(remote_endpoint).await;
    if let Err(e) = r {
        warn!("connect error: {:?}", e);
        return;
    }
    info!("TCP connected!");

    let mut read_record_buffer = [0; 16384];
    let mut write_record_buffer = [0; 16384];
    let config = TlsConfig::new()
        // .with_max_fragment_length(MaxFragmentLength::Bits11)
        .with_server_name(hostname);
    let mut tls = TlsConnection::new(socket, &mut read_record_buffer, &mut write_record_buffer);

    tls.open(TlsContext::new(
        &config,
        UnsecureProvider::new::<Aes128GcmSha256>(ChaCha8Rng::seed_from_u64(seed)),
    ))
    .await
    .expect("error establishing TLS connection");

    info!("TLS Established!");

    let request = Request::get("/")
        .host(hostname)
        .content_type(ContentType::TextPlain)
        .build();
    request.write(&mut tls).await.unwrap();

    let mut rx_buf = [0; 1024];
    let mut body_buf = [0; 8192];
    let response = Response::read(&mut tls, reqwless::request::Method::GET, &mut rx_buf)
        .await
        .unwrap();
    let len = response
        .body()
        .reader()
        .read_to_end(&mut body_buf)
        .await
        .unwrap();

    info!("{=[u8]:a}", &body_buf[..len]);
}
