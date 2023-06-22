#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]

use core::fmt::Write as _;
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_rp::gpio::{Input, Level, Output, Pull};
use embassy_rp::peripherals::{PIN_26, UART1};
use embassy_rp::uart::BufferedInterruptHandler;
use embassy_rp::{bind_interrupts, uart};
use embassy_time::{Duration, Timer};
use embedded_io::asynch::Write;
use no_std_net::{Ipv4Addr, SocketAddr};
use static_cell::make_static;
use ublox_short_range::asynch::runner::Runner;
use ublox_short_range::asynch::ublox_stack::dns::DnsSocket;
use ublox_short_range::asynch::ublox_stack::tcp::TcpSocket;
use ublox_short_range::asynch::ublox_stack::{StackResources, UbloxStack};
use ublox_short_range::asynch::{new, State};
use ublox_short_range::atat::{self, AtatIngress};
use ublox_short_range::command::custom_digest::EdmDigester;
use ublox_short_range::command::edm::urc::EdmEvent;
use ublox_short_range::embedded_nal_async::AddrType;
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

#[embassy_executor::task(pool_size = 2)]
async fn echo_task(
    stack: &'static UbloxStack<AtClient>,
    hostname: &'static str,
    port: u16,
    write_interval: Duration,
) {
    let mut rx_buffer = [0; 128];
    let mut tx_buffer = [0; 128];
    let mut buf = [0; 128];
    let mut cnt = 0u32;
    let mut msg = heapless::String::<64>::new();
    Timer::after(Duration::from_secs(1)).await;

    let ip_addr = match DnsSocket::new(stack).query(hostname, AddrType::IPv4).await {
        Ok(ip) => ip,
        Err(_) => {
            defmt::error!("[{}] Failed to resolve IP addr", hostname);
            return;
        }
    };

    let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);

    defmt::info!(
        "[{}] Connecting... {}",
        hostname,
        defmt::Debug2Format(&ip_addr)
    );
    if let Err(e) = socket.connect((ip_addr, port)).await {
        defmt::warn!("[{}] connect error: {:?}", hostname, e);
        return;
    }
    defmt::info!(
        "[{}] Connected to {:?}",
        hostname,
        defmt::Debug2Format(&socket.remote_endpoint())
    );

    loop {
        match select(Timer::after(write_interval), socket.read(&mut buf)).await {
            Either::First(_) => {
                msg.clear();
                write!(msg, "Hello {}! {}\n", ip_addr, cnt).unwrap();
                cnt = cnt.wrapping_add(1);
                if let Err(e) = socket.write_all(msg.as_bytes()).await {
                    defmt::warn!("[{}] write error: {:?}", hostname, e);
                    break;
                }
                defmt::info!("[{}] txd: {}", hostname, msg);
                Timer::after(Duration::from_millis(400)).await;
            }
            Either::Second(res) => {
                let n = match res {
                    Ok(0) => {
                        defmt::warn!("[{}] read EOF", hostname);
                        break;
                    }
                    Ok(n) => n,
                    Err(e) => {
                        defmt::warn!("[{}] {:?}", hostname, e);
                        break;
                    }
                };
                defmt::info!(
                    "[{}] rxd {}",
                    hostname,
                    core::str::from_utf8(&buf[..n]).unwrap()
                );
            }
        }
    }
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
    let mut btn = Input::new(p.PIN_27, Pull::Up);

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

    control
        .set_hostname("Factbird-duo-wifi-test")
        .await
        .unwrap();

    // Init network stack
    let stack = &*make_static!(UbloxStack::new(
        net_device,
        make_static!(StackResources::<4>::new()),
    ));

    defmt::unwrap!(spawner.spawn(net_task(stack)));

    // And now we can use it!
    defmt::info!("Device initialized!");

    // spawner
    //     .spawn(echo_task(
    //         &stack,
    //         "tcpbin.com",
    //         4242,
    //         Duration::from_millis(500),
    //     ))
    //     .unwrap();

    let mut rx_buffer = [0; 256];
    let mut tx_buffer = [0; 256];
    let mut buf = [0; 256];
    let mut cnt = 0u32;
    let mut msg = heapless::String::<64>::new();

    loop {
        loop {
            match control.join_wpa2("test", "1234abcd").await {
                Ok(_) => {
                    defmt::info!("Network connected!");
                    // spawner
                    //     .spawn(echo_task(
                    //         &stack,
                    //         "echo.u-blox.com",
                    //         7,
                    //         Duration::from_secs(1),
                    //     ))
                    //     .unwrap();
                    break;
                }
                Err(err) => {
                    defmt::info!("join failed with error={:?}. Retrying in 1 second", err);
                    Timer::after(Duration::from_secs(1)).await;
                }
            }
        }
        'outer: loop {
            Timer::after(Duration::from_secs(1)).await;

            let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
            // // socket.set_timeout(Some(Duration::from_secs(10)));

            let remote: SocketAddr = (Ipv4Addr::new(192, 168, 73, 183), 4444).into();
            defmt::info!("Connecting... {}", defmt::Debug2Format(&remote));
            if let Err(e) = socket.connect(remote).await {
                defmt::warn!("connect error: {:?}", e);
                continue;
            }
            defmt::info!(
                "Connected to {:?}",
                defmt::Debug2Format(&socket.remote_endpoint())
            );

            'inner: loop {
                match select(Timer::after(Duration::from_secs(3)), socket.read(&mut buf)).await {
                    Either::First(_) => {
                        msg.clear();
                        write!(msg, "Hello world! {}\n", cnt).unwrap();
                        cnt = cnt.wrapping_add(1);
                        if let Err(e) = socket.write_all(msg.as_bytes()).await {
                            defmt::warn!("write error: {:?}", e);
                            break;
                        }
                        defmt::info!("txd: {}", msg);
                        Timer::after(Duration::from_millis(400)).await;
                    }
                    Either::Second(res) => {
                        let n = match res {
                            Ok(0) => {
                                defmt::warn!("read EOF");
                                break;
                            }
                            Ok(n) => n,
                            Err(e) => {
                                defmt::warn!("{:?}", e);
                                break;
                            }
                        };
                        defmt::info!("rxd [{}] {}", n, core::str::from_utf8(&buf[..n]).unwrap());

                        match &buf[..n] {
                            b"c\n" => {
                                socket.close();
                                break 'inner;
                            }
                            b"a\n" => {
                                socket.abort();
                                break 'inner;
                            }
                            b"d\n" => {
                                drop(socket);
                                break 'inner;
                            }
                            b"f\n" => {
                                control.disconnect().await.unwrap();
                                break 'outer;
                            }
                            _ => {}
                        }
                    }
                }
            }
            defmt::info!("Press USER button to reconnect socket!");
            btn.wait_for_any_edge().await;
            continue;
        }
        defmt::info!("Press USER button to reconnect to WiFi!");
        btn.wait_for_any_edge().await;
    }
}
