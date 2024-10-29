#![cfg(feature = "internal-network-stack")]
#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]

use core::fmt::Write as _;
use core::net::{Ipv4Addr, SocketAddr};
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_rp::gpio::{AnyPin, Input, Level, Output, Pull};
use embassy_rp::peripherals::{PIN_26, UART1};
use embassy_rp::uart::{BufferedInterruptHandler, BufferedUartTx};
use embassy_rp::{bind_interrupts, uart};
use embassy_time::{Duration, Timer};
use embedded_io_async::Write;
use static_cell::make_static;
use ublox_short_range::asynch::runner::Runner;
use ublox_short_range::asynch::ublox_stack::dns::DnsSocket;
use ublox_short_range::asynch::ublox_stack::tcp::TcpSocket;
use ublox_short_range::asynch::ublox_stack::{StackResources, UbloxStack};
use ublox_short_range::asynch::{new, Resources, State};
use ublox_short_range::atat::{self, AtatIngress};
use ublox_short_range::command::custom_digest::EdmDigester;
use ublox_short_range::command::edm::urc::EdmEvent;
use ublox_short_range::embedded_nal_async::AddrType;
use {defmt_rtt as _, panic_probe as _};

const CMD_BUF_SIZE: usize = 128;
const INGRESS_BUF_SIZE: usize = 1024;
const URC_CAPACITY: usize = 2;

type AtClient = ublox_short_range::atat::asynch::Client<
    'static,
    uart::BufferedUartTx<'static, UART1>,
    INGRESS_BUF_SIZE,
>;

#[embassy_executor::task]
async fn wifi_task(
    runner: InternalRunner<
        'a,
        BufferedUartRx<'static, UART1>,
        BufferedUartTx<'static, UART1>,
        Output<'static, AnyPin>,
        INGRESS_BUF_SIZE,
        URC_CAPACITY,
    >,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(stack: &'static UbloxStack<AtClient, URC_CAPACITY>) -> ! {
    stack.run().await
}

#[embassy_executor::task(pool_size = 2)]
async fn echo_task(
    stack: &'static UbloxStack<AtClient, URC_CAPACITY>,
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
            error!("[{}] Failed to resolve IP addr", hostname);
            return;
        }
    };

    let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);

    info!("[{}] Connecting... {}", hostname, debug2Format(&ip_addr));
    if let Err(e) = socket.connect((ip_addr, port)).await {
        warn!("[{}] connect error: {:?}", hostname, e);
        return;
    }
    info!(
        "[{}] Connected to {:?}",
        hostname,
        debug2Format(&socket.remote_endpoint())
    );

    loop {
        match select(Timer::after(write_interval), socket.read(&mut buf)).await {
            Either::First(_) => {
                msg.clear();
                write!(msg, "Hello {}! {}\n", ip_addr, cnt).unwrap();
                cnt = cnt.wrapping_add(1);
                if let Err(e) = socket.write_all(msg.as_bytes()).await {
                    warn!("[{}] write error: {:?}", hostname, e);
                    break;
                }
                info!("[{}] txd: {}", hostname, msg);
                Timer::after(Duration::from_millis(400)).await;
            }
            Either::Second(res) => {
                let n = match res {
                    Ok(0) => {
                        warn!("[{}] read EOF", hostname);
                        break;
                    }
                    Ok(n) => n,
                    Err(e) => {
                        warn!("[{}] {:?}", hostname, e);
                        break;
                    }
                };
                info!(
                    "[{}] rxd {}",
                    hostname,
                    core::str::from_utf8(&buf[..n]).unwrap()
                );
            }
        }
    }
}

bind_interrupts!(struct Irqs {
    UART1_IRQ => BufferedInterruptHandler<UART1>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Hello World!");

    let p = embassy_rp::init(Default::default());

    let rst = Output::new(p.PIN_26, Level::High);
    let mut btn = Input::new(p.PIN_27, Pull::Up);

    static TX_BUF: StaticCell<[u8; 16]> = StaticCell::new();
    static RX_BUF: StaticCell<[u8; 16]> = StaticCell::new();

    let uart = uart::BufferedUart::new_with_rtscts(
        p.UART1,
        Irqs,
        p.PIN_24,
        p.PIN_25,
        p.PIN_23,
        p.PIN_22,
        TX_BUF.init([0; 16]),
        RX_BUF.init([0; 16]),
        uart::Config::default(),
    );
    let (uart_rx, uart_tx) = uart.split();

    static RESOURCES: StaticCell<
        Resources<BufferedUartTx<UART1>, CMD_BUF_SIZE, INGRESS_BUF_SIZE, URC_CAPACITY>,
    > = StaticCell::new();

    let (net_device, mut control, runner) = ublox_short_range::asynch::new_internal(
        uart_rx,
        uart_tx,
        RESOURCES.init(Resources::new()),
        rst,
    );

    // Init network stack
    static STACK: StaticCell<Stack<embassy_net_ppp::Device<'static>>> = StaticCell::new();
    static STACK_RESOURCES: StaticCell<StackResources<2>> = StaticCell::new();

    let stack = &*STACK.init(UbloxStack::new(
        net_device,
        STACK_RESOURCES.init(StackResources::new()),
    ));

    spawner.spawn(net_task(stack)).unwrap();
    spawner.spawn(wifi_task(runner)).unwrap();

    control
        .set_hostname("Factbird-duo-wifi-test")
        .await
        .unwrap();

    // And now we can use it!
    info!("Device initialized!");

    let mut rx_buffer = [0; 256];
    let mut tx_buffer = [0; 256];
    let mut buf = [0; 256];
    let mut cnt = 0u32;
    let mut msg = heapless::String::<64>::new();

    loop {
        loop {
            match control.join_wpa2("test", "1234abcd").await {
                Ok(_) => {
                    info!("Network connected!");
                    spawner
                        .spawn(echo_task(
                            &stack,
                            // "echo.u-blox.com",
                            // 7,
                            "tcpbin.com",
                            4242,
                            Duration::from_secs(1),
                        ))
                        .unwrap();

                    spawner
                        .spawn(echo_task(
                            &stack,
                            "tcpbin.com",
                            4242,
                            Duration::from_millis(500),
                        ))
                        .unwrap();
                    break;
                }
                Err(err) => {
                    info!("join failed with error={:?}. Retrying in 1 second", err);
                    Timer::after(Duration::from_secs(1)).await;
                }
            }
        }
        'outer: loop {
            Timer::after(Duration::from_secs(1)).await;

            let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
            // socket.set_timeout(Some(Duration::from_secs(10)));

            let remote: SocketAddr = (Ipv4Addr::new(192, 168, 1, 183), 4444).into();
            info!("Connecting... {}", debug2Format(&remote));
            if let Err(e) = socket.connect(remote).await {
                warn!("connect error: {:?}", e);
                continue;
            }
            info!("Connected to {:?}", debug2Format(&socket.remote_endpoint()));

            'inner: loop {
                match select(Timer::after(Duration::from_secs(3)), socket.read(&mut buf)).await {
                    Either::First(_) => {
                        msg.clear();
                        write!(msg, "Hello world! {}\n", cnt).unwrap();
                        cnt = cnt.wrapping_add(1);
                        if let Err(e) = socket.write_all(msg.as_bytes()).await {
                            warn!("write error: {:?}", e);
                            break;
                        }
                        info!("txd: {}", msg);
                        Timer::after(Duration::from_millis(400)).await;
                    }
                    Either::Second(res) => {
                        let n = match res {
                            Ok(0) => {
                                warn!("read EOF");
                                break;
                            }
                            Ok(n) => n,
                            Err(e) => {
                                warn!("{:?}", e);
                                break;
                            }
                        };
                        info!("rxd [{}] {}", n, core::str::from_utf8(&buf[..n]).unwrap());

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
            info!("Press USER button to reconnect socket!");
            btn.wait_for_any_edge().await;
            continue;
        }
        info!("Press USER button to reconnect to WiFi!");
        btn.wait_for_any_edge().await;
    }
}
