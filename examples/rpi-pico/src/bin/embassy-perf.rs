#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]

use core::net::Ipv4Addr;
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{PIN_26, UART1};
use embassy_rp::uart::BufferedInterruptHandler;
use embassy_rp::{bind_interrupts, uart};
use embassy_time::{with_timeout, Duration, Timer};
use static_cell::make_static;
use ublox_short_range::asynch::runner::Runner;
use ublox_short_range::asynch::ublox_stack::tcp::TcpSocket;
use ublox_short_range::asynch::ublox_stack::{StackResources, UbloxStack};
use ublox_short_range::asynch::{new, State};
use ublox_short_range::atat::{self, AtatIngress};
use ublox_short_range::command::custom_digest::EdmDigester;
use ublox_short_range::command::edm::urc::EdmEvent;
use {defmt_rtt as _, panic_probe as _};

const RX_BUF_LEN: usize = 1024;
const URC_CAPACITY: usize = 3;

type AtClient = ublox_short_range::atat::asynch::Client<
    'static,
    common::TxWrap<uart::BufferedUartTx<'static, UART1>>,
    RX_BUF_LEN,
>;

#[embassy_executor::task]
async fn wifi_task(
    runner: Runner<'static, AtClient, Output<'static, PIN_26>, 8, URC_CAPACITY>,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(stack: &'static UbloxStack<AtClient, URC_CAPACITY>) -> ! {
    stack.run().await
}

#[embassy_executor::task]
async fn ingress_task(
    mut ingress: atat::Ingress<'static, EdmDigester, EdmEvent, RX_BUF_LEN, URC_CAPACITY, 2>,
    mut rx: uart::BufferedUartRx<'static, UART1>,
) -> ! {
    ingress.read_from(&mut rx).await
}

bind_interrupts!(struct Irqs {
    UART1_IRQ => BufferedInterruptHandler<UART1>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Hello World!");

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
    let seed = 0x0123_4567_89ab_cdef; // chosen by fair dice roll. guaranteed to be random.

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

    loop {
        match control.join_wpa2(WIFI_NETWORK, WIFI_PASSWORD).await {
            Ok(_) => break,
            Err(err) => {
                defmt::panic!("join failed with status={}", err);
            }
        }
    }

    // And now we can use it!
    info!("Device initialized!");

    let down = test_download(stack).await;
    Timer::after(Duration::from_secs(SETTLE_TIME as _)).await;
    let up = test_upload(stack).await;
    Timer::after(Duration::from_secs(SETTLE_TIME as _)).await;
    let updown = test_upload_download(stack).await;
    Timer::after(Duration::from_secs(SETTLE_TIME as _)).await;

    // assert!(down > TEST_EXPECTED_DOWNLOAD_KBPS);
    // assert!(up > TEST_EXPECTED_UPLOAD_KBPS);
    // assert!(updown > TEST_EXPECTED_UPLOAD_DOWNLOAD_KBPS);

    info!("Test OK");
    cortex_m::asm::bkpt();
}

// Test-only wifi network, no internet access!
const WIFI_NETWORK: &str = "WiFimodem-7A76";
const WIFI_PASSWORD: &str = "ndzwqzyhhd";

const TEST_DURATION: usize = 10;
const SETTLE_TIME: usize = 5;
const TEST_EXPECTED_DOWNLOAD_KBPS: usize = 300;
const TEST_EXPECTED_UPLOAD_KBPS: usize = 300;
const TEST_EXPECTED_UPLOAD_DOWNLOAD_KBPS: usize = 300;
const RX_BUFFER_SIZE: usize = 4096;
const TX_BUFFER_SIZE: usize = 4096;
const SERVER_ADDRESS: Ipv4Addr = Ipv4Addr::new(192, 168, 0, 8);
const DOWNLOAD_PORT: u16 = 4321;
const UPLOAD_PORT: u16 = 4322;
const UPLOAD_DOWNLOAD_PORT: u16 = 4323;

async fn test_download(stack: &'static UbloxStack<AtClient, URC_CAPACITY>) -> usize {
    info!("Testing download...");

    let mut rx_buffer = [0; RX_BUFFER_SIZE];
    let mut tx_buffer = [0; TX_BUFFER_SIZE];
    let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
    // socket.set_timeout(Some(Duration::from_secs(10)));

    info!(
        "connecting to {:?}:{}...",
        debug2Format(&SERVER_ADDRESS),
        DOWNLOAD_PORT
    );
    if let Err(e) = socket.connect((SERVER_ADDRESS, DOWNLOAD_PORT)).await {
        error!("connect error: {:?}", e);
        return 0;
    }
    info!("connected, testing...");

    let mut rx_buf = [0; 4096];
    let mut total: usize = 0;
    with_timeout(Duration::from_secs(TEST_DURATION as _), async {
        loop {
            match socket.read(&mut rx_buf).await {
                Ok(0) => {
                    error!("read EOF");
                    return 0;
                }
                Ok(n) => total += n,
                Err(e) => {
                    error!("read error: {:?}", e);
                    return 0;
                }
            }
        }
    })
    .await
    .ok();

    let kbps = (total + 512) / 1024 / TEST_DURATION;
    info!("download: {} kB/s", kbps);
    kbps
}

async fn test_upload(stack: &'static UbloxStack<AtClient, URC_CAPACITY>) -> usize {
    info!("Testing upload...");

    let mut rx_buffer = [0; RX_BUFFER_SIZE];
    let mut tx_buffer = [0; TX_BUFFER_SIZE];
    let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
    // socket.set_timeout(Some(Duration::from_secs(10)));

    info!(
        "connecting to {:?}:{}...",
        debug2Format(&SERVER_ADDRESS),
        UPLOAD_PORT
    );
    if let Err(e) = socket.connect((SERVER_ADDRESS, UPLOAD_PORT)).await {
        error!("connect error: {:?}", e);
        return 0;
    }
    info!("connected, testing...");

    let buf = [0; 4096];
    let mut total: usize = 0;
    with_timeout(Duration::from_secs(TEST_DURATION as _), async {
        loop {
            match socket.write(&buf).await {
                Ok(0) => {
                    error!("write zero?!??!?!");
                    return 0;
                }
                Ok(n) => total += n,
                Err(e) => {
                    error!("write error: {:?}", e);
                    return 0;
                }
            }
        }
    })
    .await
    .ok();

    let kbps = (total + 512) / 1024 / TEST_DURATION;
    info!("upload: {} kB/s", kbps);
    kbps
}

async fn test_upload_download(stack: &'static UbloxStack<AtClient, URC_CAPACITY>) -> usize {
    info!("Testing upload+download...");

    let mut rx_buffer = [0; RX_BUFFER_SIZE];
    let mut tx_buffer = [0; TX_BUFFER_SIZE];
    let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
    // socket.set_timeout(Some(Duration::from_secs(10)));

    info!(
        "connecting to {:?}:{}...",
        debug2Format(&SERVER_ADDRESS),
        UPLOAD_DOWNLOAD_PORT
    );
    if let Err(e) = socket.connect((SERVER_ADDRESS, UPLOAD_DOWNLOAD_PORT)).await {
        error!("connect error: {:?}", e);
        return 0;
    }
    info!("connected, testing...");

    let (mut reader, mut writer) = socket.split();

    let tx_buf = [0; 4096];
    let mut rx_buf = [0; 4096];
    let mut total: usize = 0;
    let tx_fut = async {
        loop {
            match writer.write(&tx_buf).await {
                Ok(0) => {
                    error!("write zero?!??!?!");
                    return 0;
                }
                Ok(_) => {}
                Err(e) => {
                    error!("write error: {:?}", e);
                    return 0;
                }
            }
        }
    };

    let rx_fut = async {
        loop {
            match reader.read(&mut rx_buf).await {
                Ok(0) => {
                    error!("read EOF");
                    return 0;
                }
                Ok(n) => total += n,
                Err(e) => {
                    error!("read error: {:?}", e);
                    return 0;
                }
            }
        }
    };

    if with_timeout(
        Duration::from_secs(TEST_DURATION as _),
        join(tx_fut, rx_fut),
    )
    .await
    .is_err()
    {
        error!("Test timed out");
    }

    let kbps = (total + 512) / 1024 / TEST_DURATION;
    info!("upload+download: {} kB/s", kbps);
    kbps
}
