#![no_std]
#![no_main]

#[cfg(not(feature = "ppp"))]
compile_error!("You must enable the `ppp` feature flag to build this example");

use defmt::*;
use embassy_executor::Spawner;
use embassy_net::tcp::TcpSocket;
use embassy_net::StackResources;
use embassy_rp::gpio::{Level, OutputOpenDrain};
use embassy_rp::uart::{self, BufferedInterruptHandler, BufferedUart};
use embassy_rp::{bind_interrupts, peripherals::UART1};
use embassy_time::{Duration, Timer};
use embedded_io_async::{BufRead, Read, Write};
use static_cell::StaticCell;
use ublox_short_range::asynch::{Resources, Runner};
use ublox_short_range::options::ConnectionOptions;
use ublox_short_range::Transport;
use {defmt_rtt as _, panic_probe as _};

const INGRESS_BUF_SIZE: usize = 512;
const URC_CAPACITY: usize = 2;

/// Wrapper around BufferedUart that implements the Transport trait
struct UartTransport {
    inner: BufferedUart,
}

impl embedded_io_async::ErrorType for UartTransport {
    type Error = embassy_rp::uart::Error;
}

impl Read for UartTransport {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.inner.read(buf).await
    }
}

impl BufRead for UartTransport {
    async fn fill_buf(&mut self) -> Result<&[u8], Self::Error> {
        self.inner.fill_buf().await
    }

    fn consume(&mut self, amt: usize) {
        self.inner.consume(amt)
    }
}

impl Write for UartTransport {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.inner.write(buf).await
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.inner.flush().await
    }
}

impl Transport for UartTransport {
    fn set_baudrate(&mut self, baudrate: u32) {
        self.inner.set_baudrate(baudrate);
    }

    fn split_ref(&mut self) -> (impl Write, impl Read) {
        self.inner.split_ref()
    }
}

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
async fn net_task(mut runner: embassy_net::Runner<'static, embassy_net_ppp::Device<'static>>) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn ppp_task(
    mut runner: Runner<'static, UartTransport, WifiConfig, INGRESS_BUF_SIZE, URC_CAPACITY>,
    stack: embassy_net::Stack<'static>,
) -> ! {
    runner.run(stack).await
}

bind_interrupts!(struct Irqs {
    UART1_IRQ => BufferedInterruptHandler<UART1>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let rst_pin = OutputOpenDrain::new(p.PIN_26, Level::High);

    static TX_BUF: StaticCell<[u8; 32]> = StaticCell::new();
    static RX_BUF: StaticCell<[u8; 32]> = StaticCell::new();
    let wifi_uart = uart::BufferedUart::new_with_rtscts(
        p.UART1,
        p.PIN_24,
        p.PIN_25,
        p.PIN_23,
        p.PIN_22,
        Irqs,
        TX_BUF.init([0; 32]),
        RX_BUF.init([0; 32]),
        uart::Config::default(),
    );

    let transport = UartTransport { inner: wifi_uart };

    static RESOURCES: StaticCell<Resources<INGRESS_BUF_SIZE, URC_CAPACITY>> = StaticCell::new();

    let (mut runner, control) = Runner::new(
        transport,
        RESOURCES.init(Resources::new()),
        WifiConfig { rst_pin },
    );

    static PPP_STATE: StaticCell<embassy_net_ppp::State<2, 2>> = StaticCell::new();
    let net_device = runner.ppp_stack(PPP_STATE.init(embassy_net_ppp::State::new()));

    // Generate random seed
    let seed = 0x0123_4567_89ab_cdef; // chosen by fair dice roll. guaranteed to be random.

    // Init network stack
    static STACK_RESOURCES: StaticCell<StackResources<6>> = StaticCell::new();

    let (stack, net_runner) = embassy_net::new(
        net_device,
        embassy_net::Config::default(),
        STACK_RESOURCES.init(StackResources::new()),
        seed,
    );

    spawner.spawn(net_task(net_runner).unwrap());
    spawner.spawn(ppp_task(runner, stack).unwrap());

    stack.wait_config_up().await;

    Timer::after(Duration::from_secs(1)).await;

    control.set_hostname("Ublox-wifi-test").await.ok();

    let options = ConnectionOptions::new("MyAccessPoint").wpa_psk("12345678");
    control.join_sta(options).await.unwrap();

    info!("We have network!");

    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];
    let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
    socket.set_timeout(Some(Duration::from_secs(10)));

    let remote_endpoint =
        embassy_net::IpEndpoint::new(embassy_net::IpAddress::v4(93, 184, 216, 34), 80);
    info!("connecting to {:?}...", remote_endpoint);
    let r = socket.connect(remote_endpoint).await;
    if let Err(e) = r {
        warn!("connect error: {:?}", e);
        return;
    }
    info!("TCP connected!");

    let request = b"GET / HTTP/1.1\r\nHost: example.com\r\n\r\n";
    socket.write_all(request).await.unwrap();
    info!("Request sent");

    let mut buf = [0; 1024];
    loop {
        match socket.read(&mut buf).await {
            Ok(0) => {
                info!("Connection closed");
                break;
            }
            Ok(n) => {
                info!("Received {} bytes", n);
            }
            Err(e) => {
                warn!("Read error: {:?}", e);
                break;
            }
        }
    }
}
