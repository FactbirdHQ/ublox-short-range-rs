#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{PIN_26, UART1};
use embassy_rp::{interrupt, uart};
use embassy_time::{Duration, Timer};
use static_cell::StaticCell;
use ublox_short_range::asynch::runner::Runner;
use ublox_short_range::asynch::ublox_stack::{StackResources, UbloxStack};
use ublox_short_range::asynch::{new, State};
use ublox_short_range::atat::{self, AtatIngress, AtatUrcChannel};
use ublox_short_range::command::custom_digest::EdmDigester;
use ublox_short_range::command::edm::urc::EdmEvent;
use ublox_short_range::command::gpio::types::{GPIOId, GPIOValue};
use {defmt_rtt as _, panic_probe as _};

const RX_BUF_LEN: usize = 1024;
const URC_CAPACITY: usize = 3;

macro_rules! singleton {
    ($val:expr) => {{
        type T = impl Sized;
        static STATIC_CELL: StaticCell<T> = StaticCell::new();
        let (x,) = STATIC_CELL.init(($val,));
        x
    }};
}

#[embassy_executor::task]
async fn wifi_task(
    runner: Runner<
        'static,
        ublox_short_range::atat::asynch::Client<
            'static,
            uart::BufferedUartTx<'static, UART1>,
            RX_BUF_LEN,
        >,
        Output<'static, PIN_26>,
    >,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(stack: &'static UbloxStack) -> ! {
    stack.run().await
}

#[embassy_executor::task]
async fn ingress_task(
    mut ingress: atat::Ingress<'static, EdmDigester, EdmEvent, RX_BUF_LEN, URC_CAPACITY, 1>,
    mut rx: uart::BufferedUartRx<'static, UART1>,
) -> ! {
    ingress.read_from(&mut rx).await
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Hello World!");

    let p = embassy_rp::init(Default::default());

    let rst = Output::new(p.PIN_26, Level::High);

    let (tx_pin, rx_pin, rts_pin, cts_pin, uart) =
        (p.PIN_24, p.PIN_25, p.PIN_23, p.PIN_22, p.UART1);

    let irq = interrupt::take!(UART1_IRQ);
    let tx_buf = &mut singleton!([0u8; 64])[..];
    let rx_buf = &mut singleton!([0u8; 64])[..];
    let uart = uart::BufferedUart::new_with_rtscts(
        uart,
        irq,
        tx_pin,
        rx_pin,
        rts_pin,
        cts_pin,
        tx_buf,
        rx_buf,
        uart::Config::default(),
    );
    let (rx, tx) = uart.split();

    let buffers = &*singleton!(atat::Buffers::new());
    let (ingress, client) = buffers.split(tx, EdmDigester::default(), atat::Config::new());

    unwrap!(spawner.spawn(ingress_task(ingress, rx)));

    let state = singleton!(State::new(client));
    let (net_device, mut control, runner) =
        new(state, buffers.urc_channel.subscribe().unwrap(), rst).await;

    unwrap!(spawner.spawn(wifi_task(runner)));

    // control.init(clm).await;
    // control
    //     .set_power_management(cyw43::PowerManagementMode::PowerSave)
    //     .await;

    // Init network stack
    let stack = &*singleton!(UbloxStack::new(
        net_device,
        singleton!(StackResources::<2>::new()),
    ));

    unwrap!(spawner.spawn(net_task(stack)));

    loop {
        match control.join_wpa2("test", "1234abcd").await {
            Ok(_) => break,
            Err(err) => {
                info!("join failed with error={:?}", err);
            }
        }
    }

    // And now we can use it!

    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];
    let mut buf = [0; 4096];

    defmt::info!("Device initialized!");

    loop {
        Timer::after(Duration::from_millis(1000)).await;
        // let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
        // // socket.set_timeout(Some(embassy_net::SmolDuration::from_secs(10)));

        // control.gpio_set(GPIOId::A10, GPIOValue::Low).await;
        // // info!("Listening on TCP:1234...");
        // // if let Err(e) = socket.accept(1234).await {
        // //     warn!("accept error: {:?}", e);
        // //     continue;
        // // }

        // // info!("Received connection from {:?}", socket.remote_endpoint());
        // control.gpio_set(GPIOId::A10, GPIOValue::High).await;

        // loop {
        //     let n = match socket.read(&mut buf).await {
        //         Ok(0) => {
        //             warn!("read EOF");
        //             break;
        //         }
        //         Ok(n) => n,
        //         Err(e) => {
        //             warn!("read error: {:?}", e);
        //             break;
        //         }
        //     };

        //     info!("rxd {}", from_utf8(&buf[..n]).unwrap());

        //     match socket.write_all(&buf[..n]).await {
        //         Ok(()) => {}
        //         Err(e) => {
        //             warn!("write error: {:?}", e);
        //             break;
        //         }
        //     };
        // }
    }
}
