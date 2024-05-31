#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

#[cfg(not(feature = "ppp"))]
compile_error!("You must enable the `ppp` feature flag to build this example");

use defmt::*;
use embassy_executor::Spawner;
use embassy_net::tcp::TcpSocket;
use embassy_net::{Ipv4Address, Stack, StackResources};
use embassy_rp::gpio::{AnyPin, Level, Output, Pin};
use embassy_rp::peripherals::UART1;
use embassy_rp::uart::{BufferedInterruptHandler, BufferedUart};
use embassy_rp::{bind_interrupts, uart};
use embassy_time::{Duration, Timer};
use embedded_tls::Aes128GcmSha256;
use embedded_tls::TlsConfig;
use embedded_tls::TlsConnection;
use embedded_tls::TlsContext;
use embedded_tls::UnsecureProvider;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;
use reqwless::headers::ContentType;
use reqwless::request::Request;
use reqwless::request::RequestBuilder as _;
use reqwless::response::Response;
use static_cell::StaticCell;
use ublox_short_range::asynch::{PPPRunner, Resources};
use {defmt_rtt as _, panic_probe as _};

const CMD_BUF_SIZE: usize = 128;
const INGRESS_BUF_SIZE: usize = 512;
const URC_CAPACITY: usize = 2;

#[embassy_executor::task]
async fn net_task(stack: &'static Stack<embassy_net_ppp::Device<'static>>) -> ! {
    stack.run().await
}

#[embassy_executor::task]
async fn ppp_task(
    mut runner: PPPRunner<'static, Output<'static>, INGRESS_BUF_SIZE, URC_CAPACITY>,
    interface: BufferedUart<'static, UART1>,
    stack: &'static embassy_net::Stack<embassy_net_ppp::Device<'static>>,
) -> ! {
    runner.run(interface, stack).await
}

bind_interrupts!(struct Irqs {
    UART1_IRQ => BufferedInterruptHandler<UART1>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let rst = Output::new(p.PIN_26.degrade(), Level::High);

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

    let (net_device, mut control, runner) =
        ublox_short_range::asynch::new_ppp(RESOURCES.init(Resources::new()), rst);

    // Generate random seed
    let seed = 0x0123_4567_89ab_cdef; // chosen by fair dice roll. guarenteed to be random.

    // Init network stack
    static STACK: StaticCell<Stack<embassy_net_ppp::Device<'static>>> = StaticCell::new();
    static STACK_RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();

    let stack = &*STACK.init(Stack::new(
        net_device,
        embassy_net::Config::default(),
        STACK_RESOURCES.init(StackResources::new()),
        seed,
    ));

    spawner.spawn(net_task(stack)).unwrap();
    spawner.spawn(ppp_task(runner, wifi_uart, &stack)).unwrap();

    stack.wait_config_up().await;

    Timer::after(Duration::from_secs(1)).await;

    control
        .set_hostname("Factbird-duo-wifi-test")
        .await
        .unwrap();

    control.join_open("Test").await;

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
    let config = TlsConfig::new().with_server_name(hostname);
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

    let mut rx_buf = [0; 4096];
    let response = Response::read(&mut tls, reqwless::request::Method::GET, &mut rx_buf)
        .await
        .unwrap();
    info!("{=[u8]:a}", rx_buf);
}
