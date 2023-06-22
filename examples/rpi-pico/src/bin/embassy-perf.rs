#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]

use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{PIN_26, UART1};
use embassy_rp::uart::BufferedInterruptHandler;
use embassy_rp::{bind_interrupts, uart};
use embassy_time::{with_timeout, Duration};
use no_std_net::Ipv4Addr;
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
    uart::BufferedUartTx<'static, UART1>,
    RX_BUF_LEN,
>;

#[embassy_executor::task]
async fn wifi_task(runner: Runner<'static, AtClient, Output<'static, PIN_26>, 8>) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(stack: &'static UbloxStack<AtClient>) -> ! {
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
    defmt::info!("Hello World!");

    let p = embassy_rp::init(Default::default());

    let rst = Output::new(p.PIN_26, Level::High);

    let (tx_pin, rx_pin, rts_pin, cts_pin, uart) =
        (p.PIN_24, p.PIN_25, p.PIN_23, p.PIN_22, p.UART1);

    let tx_buf = &mut make_static!([0u8; 64])[..];
    let rx_buf = &mut make_static!([0u8; 64])[..];
    let uart = uart::BufferedUart::new_with_rtscts(
        uart,
        Irqs,
        tx_pin,
        rx_pin,
        rts_pin,
        cts_pin,
        tx_buf,
        rx_buf,
        uart::Config::default(),
    );
    let (rx, tx) = uart.split();

    let buffers = &*make_static!(atat::Buffers::new());
    let (ingress, client) = buffers.split(tx, EdmDigester::default(), atat::Config::new());
    defmt::unwrap!(spawner.spawn(ingress_task(ingress, rx)));

    let state = make_static!(State::new(client));
    let (net_device, mut control, runner) = new(state, &buffers.urc_channel, rst).await;

    defmt::unwrap!(spawner.spawn(wifi_task(runner)));

    // Init network stack
    let stack = &*make_static!(UbloxStack::new(
        net_device,
        make_static!(StackResources::<4>::new()),
    ));

    defmt::unwrap!(spawner.spawn(net_task(stack)));

    loop {
        match control.join_wpa2(WIFI_NETWORK, WIFI_PASSWORD).await {
            Ok(_) => break,
            Err(err) => {
                defmt::panic!("join failed with status={}", err);
            }
        }
    }

    // And now we can use it!
    defmt::info!("Device initialized!");

    let down = test_download(stack).await;
    let up = test_upload(stack).await;
    let updown = test_upload_download(stack).await;

    assert!(down > TEST_EXPECTED_DOWNLOAD_KBPS);
    assert!(up > TEST_EXPECTED_UPLOAD_KBPS);
    assert!(updown > TEST_EXPECTED_UPLOAD_DOWNLOAD_KBPS);

    defmt::info!("Test OK");
    cortex_m::asm::bkpt();
}

// Test-only wifi network, no internet access!
const WIFI_NETWORK: &str = "EmbassyTest";
const WIFI_PASSWORD: &str = "V8YxhKt5CdIAJFud";

const TEST_DURATION: usize = 10;
const TEST_EXPECTED_DOWNLOAD_KBPS: usize = 300;
const TEST_EXPECTED_UPLOAD_KBPS: usize = 300;
const TEST_EXPECTED_UPLOAD_DOWNLOAD_KBPS: usize = 300;
const RX_BUFFER_SIZE: usize = 4096;
const TX_BUFFER_SIZE: usize = 4096;
const SERVER_ADDRESS: Ipv4Addr = Ipv4Addr::new(192, 168, 2, 2);
const DOWNLOAD_PORT: u16 = 4321;
const UPLOAD_PORT: u16 = 4322;
const UPLOAD_DOWNLOAD_PORT: u16 = 4323;

async fn test_download(stack: &'static UbloxStack<AtClient>) -> usize {
    defmt::info!("Testing download...");

    let mut rx_buffer = [0; RX_BUFFER_SIZE];
    let mut tx_buffer = [0; TX_BUFFER_SIZE];
    let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
    // socket.set_timeout(Some(Duration::from_secs(10)));

    defmt::info!(
        "connecting to {:?}:{}...",
        defmt::Debug2Format(&SERVER_ADDRESS),
        DOWNLOAD_PORT
    );
    if let Err(e) = socket.connect((SERVER_ADDRESS, DOWNLOAD_PORT)).await {
        defmt::error!("connect error: {:?}", e);
        return 0;
    }
    defmt::info!("connected, testing...");

    let mut rx_buf = [0; 4096];
    let mut total: usize = 0;
    with_timeout(Duration::from_secs(TEST_DURATION as _), async {
        loop {
            match socket.read(&mut rx_buf).await {
                Ok(0) => {
                    defmt::error!("read EOF");
                    return 0;
                }
                Ok(n) => total += n,
                Err(e) => {
                    defmt::error!("read error: {:?}", e);
                    return 0;
                }
            }
        }
    })
    .await
    .ok();

    let kbps = (total + 512) / 1024 / TEST_DURATION;
    defmt::info!("download: {} kB/s", kbps);
    kbps
}

async fn test_upload(stack: &'static UbloxStack<AtClient>) -> usize {
    defmt::info!("Testing upload...");

    let mut rx_buffer = [0; RX_BUFFER_SIZE];
    let mut tx_buffer = [0; TX_BUFFER_SIZE];
    let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
    // socket.set_timeout(Some(Duration::from_secs(10)));

    defmt::info!(
        "connecting to {:?}:{}...",
        defmt::Debug2Format(&SERVER_ADDRESS),
        UPLOAD_PORT
    );
    if let Err(e) = socket.connect((SERVER_ADDRESS, UPLOAD_PORT)).await {
        defmt::error!("connect error: {:?}", e);
        return 0;
    }
    defmt::info!("connected, testing...");

    let buf = [0; 4096];
    let mut total: usize = 0;
    with_timeout(Duration::from_secs(TEST_DURATION as _), async {
        loop {
            match socket.write(&buf).await {
                Ok(0) => {
                    defmt::error!("write zero?!??!?!");
                    return 0;
                }
                Ok(n) => total += n,
                Err(e) => {
                    defmt::error!("write error: {:?}", e);
                    return 0;
                }
            }
        }
    })
    .await
    .ok();

    let kbps = (total + 512) / 1024 / TEST_DURATION;
    defmt::info!("upload: {} kB/s", kbps);
    kbps
}

async fn test_upload_download(stack: &'static UbloxStack<AtClient>) -> usize {
    defmt::info!("Testing upload+download...");

    let mut rx_buffer = [0; RX_BUFFER_SIZE];
    let mut tx_buffer = [0; TX_BUFFER_SIZE];
    let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
    // socket.set_timeout(Some(Duration::from_secs(10)));

    defmt::info!(
        "connecting to {:?}:{}...",
        defmt::Debug2Format(&SERVER_ADDRESS),
        UPLOAD_DOWNLOAD_PORT
    );
    if let Err(e) = socket.connect((SERVER_ADDRESS, UPLOAD_DOWNLOAD_PORT)).await {
        defmt::error!("connect error: {:?}", e);
        return 0;
    }
    defmt::info!("connected, testing...");

    let (mut reader, mut writer) = socket.split();

    let tx_buf = [0; 4096];
    let mut rx_buf = [0; 4096];
    let mut total: usize = 0;
    let tx_fut = async {
        loop {
            match writer.write(&tx_buf).await {
                Ok(0) => {
                    defmt::error!("write zero?!??!?!");
                    return 0;
                }
                Ok(_) => {}
                Err(e) => {
                    defmt::error!("write error: {:?}", e);
                    return 0;
                }
            }
        }
    };

    let rx_fut = async {
        loop {
            match reader.read(&mut rx_buf).await {
                Ok(0) => {
                    defmt::error!("read EOF");
                    return 0;
                }
                Ok(n) => total += n,
                Err(e) => {
                    defmt::error!("read error: {:?}", e);
                    return 0;
                }
            }
        }
    };

    with_timeout(
        Duration::from_secs(TEST_DURATION as _),
        join(tx_fut, rx_fut),
    )
    .await
    .ok();

    let kbps = (total + 512) / 1024 / TEST_DURATION;
    defmt::info!("upload+download: {} kB/s", kbps);
    kbps
}
