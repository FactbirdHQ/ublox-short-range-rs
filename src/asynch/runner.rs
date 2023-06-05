use core::{future::poll_fn, task::Poll};

use crate::{
    command::{
        data_mode::{types::PeerConfigParameter, SetPeerConfiguration},
        edm::{urc::EdmEvent, EdmAtCmdWrapper, EdmDataCommand, SwitchToEdmCommand},
        network::SetNetworkHostName,
        system::{
            types::{BaudRate, ChangeAfterConfirm, FlowControl, Parity, StopBits},
            RebootDCE, SetRS232Settings, StoreCurrentConfig,
        },
        wifi::{
            types::{StatusId, WifiConfig, WifiStatus, WifiStatusVal},
            GetWifiStatus, SetWifiConfig,
        },
        Urc,
    },
    config::Config,
    error::Error,
};
use atat::{asynch::AtatClient, UrcSubscription};
use ch::driver::LinkState;
use embassy_futures::select::{select, Either};
use embassy_net_driver_channel as ch;
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};
use embassy_time::{with_timeout, Duration, Timer};
use embedded_hal::digital::OutputPin;

use super::{
    ublox_stack::{DataPacket, SocketEvent},
    MTU,
};

/// Background runner for the Ublox Module.
///
/// You must call `.run()` in a background task for the Ublox Module to operate.
pub struct Runner<'d, AT: AtatClient, RST: OutputPin> {
    ch: ch::Runner<'d, MTU>,
    at_handle: &'d Mutex<NoopRawMutex, AT>,
    reset: RST,
    config: Config,
    urc_subscription: UrcSubscription<'d, EdmEvent>,
}

impl<'d, AT: AtatClient, RST: OutputPin> Runner<'d, AT, RST> {
    pub(crate) fn new(
        ch: ch::Runner<'d, MTU>,
        at_handle: &'d Mutex<NoopRawMutex, AT>,
        reset: RST,
        urc_subscription: UrcSubscription<'d, EdmEvent>,
    ) -> Self {
        Self {
            ch,
            at_handle,
            reset,
            config: Config::default(),
            urc_subscription,
        }
    }

    pub(crate) async fn init(&mut self) -> Result<(), Error> {
        // Initilize a new ublox device to a known state (set RS232 settings)
        defmt::debug!("Initializing module");
        // Hard reset module
        self.reset().await?;

        // Switch to EDM on Init. If in EDM, fail and check with autosense
        self.send(SwitchToEdmCommand).await.ok();

        loop {
            let urc = self.urc_subscription.next_message_pure().await;
            if matches!(urc, EdmEvent::StartUp) {
                break;
            }
            // Ignore AT results until we are successful in EDM mode
            self.send(SwitchToEdmCommand).await.ok();
            Timer::after(Duration::from_millis(100)).await;
        }

        // TODO: handle EDM settings quirk see EDM datasheet: 2.2.5.1 AT Request Serial settings
        self.send_edm(SetRS232Settings {
            baud_rate: BaudRate::B115200,
            flow_control: FlowControl::On,
            data_bits: 8,
            stop_bits: StopBits::One,
            parity: Parity::None,
            change_after_confirm: ChangeAfterConfirm::ChangeAfterOK,
        })
        .await?;

        if let Some(hostname) = self.config.hostname.clone() {
            self.send_edm(SetNetworkHostName {
                host_name: hostname.as_str(),
            })
            .await?;
        }

        // self.send_edm(SetWifiConfig {
        //     config_param: WifiConfig::RemainOnChannel(0),
        // })
        // .await?;

        self.send_edm(StoreCurrentConfig).await?;

        // self.software_reset().await?;

        // // FIXME: Prevent infinite loop
        // loop {
        //     let urc = self.urc_subscription.next_message_pure().await;
        //     if matches!(urc, EdmEvent::StartUp) {
        //         break;
        //     }
        //     self.send(SwitchToEdmCommand).await.ok();
        //     Timer::after(Duration::from_millis(100)).await;
        // }

        if let Some(size) = self.config.tls_in_buffer_size {
            self.send_edm(SetPeerConfiguration {
                parameter: PeerConfigParameter::TlsInBuffer(size),
            })
            .await?;
        }

        if let Some(size) = self.config.tls_out_buffer_size {
            self.send_edm(SetPeerConfiguration {
                parameter: PeerConfigParameter::TlsOutBuffer(size),
            })
            .await?;
        }

        Ok(())
    }

    async fn send_edm<Cmd: atat::AtatCmd<LEN>, const LEN: usize>(
        &mut self,
        cmd: Cmd,
    ) -> Result<Cmd::Response, atat::Error> {
        self.send(EdmAtCmdWrapper(cmd)).await
    }

    async fn send<Cmd: atat::AtatCmd<LEN>, const LEN: usize>(
        &mut self,
        cmd: Cmd,
    ) -> Result<Cmd::Response, atat::Error> {
        self.at_handle
            .lock()
            .await
            .send_retry::<Cmd, LEN>(&cmd)
            .await
    }

    async fn wait_startup(&mut self, timeout: Duration) -> Result<(), Error> {
        let fut = poll_fn(|_cx| {
            if let Some(EdmEvent::ATEvent(Urc::StartUp)) | Some(EdmEvent::StartUp) =
                self.urc_subscription.try_next_message_pure()
            {
                return Poll::Ready(());
            }
            Poll::Pending
        });

        with_timeout(timeout, fut).await.map_err(|_| Error::Timeout)
    }

    pub async fn reset(&mut self) -> Result<(), Error> {
        defmt::warn!("Hard resetting Ublox Short Range");
        self.reset.set_low().ok();
        Timer::after(Duration::from_millis(100)).await;
        self.reset.set_high().ok();

        self.wait_startup(Duration::from_secs(4)).await?;

        Ok(())
    }

    pub async fn software_reset(&mut self) -> Result<(), Error> {
        defmt::warn!("Soft resetting Ublox Short Range");
        self.send_edm(RebootDCE).await?;

        self.wait_startup(Duration::from_secs(10)).await?;

        Ok(())
    }

    pub async fn is_link_up(&mut self) -> Result<bool, Error> {
        let status = self
            .send_edm(GetWifiStatus {
                status_id: StatusId::Status,
            })
            .await?;

        Ok(matches!(
            status.status_id,
            WifiStatus::Status(WifiStatusVal::Connected)
        ))
    }

    pub async fn run(mut self) -> ! {
        loop {
            let tx = self.ch.tx_buf();
            let urc = self.urc_subscription.next_message_pure();

            match select(tx, urc).await {
                Either::First(p) => {
                    if let Ok(packet) = postcard::from_bytes::<DataPacket>(p) {
                        self.at_handle
                            .lock()
                            .await
                            .send_retry(&EdmDataCommand {
                                channel: packet.edm_channel,
                                data: packet.payload,
                            })
                            .await
                            .ok();
                    }
                    self.ch.tx_done();
                }
                Either::Second(p) => match p {
                    EdmEvent::BluetoothConnectEvent(_) => {}
                    EdmEvent::ATEvent(urc) => self.handle_urc(urc),
                    EdmEvent::StartUp => todo!(),

                    // All below events needs to be conveyed to `self.ch.rx`
                    EdmEvent::IPv4ConnectEvent(ev) => self.rx(ev),
                    EdmEvent::IPv6ConnectEvent(ev) => self.rx(ev),
                    EdmEvent::DisconnectEvent(channel_id) => self.rx(channel_id),
                    EdmEvent::DataEvent(ev) => {
                        let packet = DataPacket {
                            edm_channel: ev.channel_id,
                            payload: ev.data.as_slice(),
                        };

                        self.rx(packet);
                    }
                },
            }
        }
    }

    fn rx<'a>(&mut self, packet: impl Into<SocketEvent<'a>>) {
        match self.ch.try_rx_buf() {
            Some(buf) => {
                let event: SocketEvent = packet.into();
                let used = postcard::to_slice(&event, buf).unwrap();
                let len = used.len();
                self.ch.rx_done(len);
            }
            None => {
                defmt::warn!("failed to push rxd packet to the channel.")
            }
        }
    }

    fn handle_urc(&mut self, urc: Urc) {
        match urc {
            Urc::StartUp => {}
            Urc::PeerConnected(_) => todo!(),
            Urc::PeerDisconnected(_) => todo!(),
            Urc::WifiLinkConnected(_) => {
                self.ch.set_link_state(LinkState::Up);
            }
            Urc::WifiLinkDisconnected(_) => {
                self.ch.set_link_state(LinkState::Down);
            }
            Urc::WifiAPUp(_) => todo!(),
            Urc::WifiAPDown(_) => todo!(),
            Urc::WifiAPStationConnected(_) => todo!(),
            Urc::WifiAPStationDisconnected(_) => todo!(),
            Urc::EthernetLinkUp(_) => todo!(),
            Urc::EthernetLinkDown(_) => todo!(),
            Urc::NetworkUp(_) => {}
            Urc::NetworkDown(_) => {}
            Urc::NetworkError(_) => todo!(),
            Urc::PingResponse(_) => todo!(),
            Urc::PingErrorResponse(_) => todo!(),
        }
    }
}
