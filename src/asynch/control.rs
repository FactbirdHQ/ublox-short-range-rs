use atat::asynch::AtatClient;
use ch::driver::LinkState;
use embassy_net_driver_channel as ch;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{with_timeout, Duration, Timer};

use crate::command::wifi::types::{
    Authentication, StatusId, WifiStationAction, WifiStationConfig, WifiStatus, WifiStatusVal,
};
use crate::command::wifi::{
    ExecWifiStationAction, GetWifiMac, GetWifiStatus, SetWifiStationConfig,
};
use crate::command::{
    edm::EdmAtCmdWrapper,
    gpio::{
        types::{GPIOId, GPIOValue},
        WriteGPIO,
    },
};
use crate::error::Error;

pub struct Control<'a, AT: AtatClient> {
    state_ch: ch::StateRunner<'a>,
    at_client: &'a Mutex<NoopRawMutex, AT>,
}

impl<'a, AT: AtatClient> Control<'a, AT> {
    pub(crate) fn new(
        state_ch: ch::StateRunner<'a>,
        at_client: &'a Mutex<NoopRawMutex, AT>,
    ) -> Self {
        Self {
            state_ch,
            at_client,
        }
    }

    pub(crate) async fn init(&mut self) -> Result<(), Error> {
        defmt::debug!("Initalizing ublox control");
        // read MAC addr.
        let resp = self.send_edm(GetWifiMac).await?;
        // FIXME: MAC length here?
        // self.state_ch
        //     .set_ethernet_address(resp.mac_addr.as_slice().try_into().unwrap());

        // let country = countries::WORLD_WIDE_XX;
        // let country_info = CountryInfo {
        //     country_abbrev: [country.code[0], country.code[1], 0, 0],
        //     country_code: [country.code[0], country.code[1], 0, 0],
        //     rev: if country.rev == 0 {
        //         -1
        //     } else {
        //         country.rev as _
        //     },
        // };
        // self.set_iovar("country", &country_info.to_bytes()).await;

        // // set country takes some time, next ioctls fail if we don't wait.
        // Timer::after(Duration::from_millis(100)).await;

        // // Set antenna to chip antenna
        // self.ioctl_set_u32(IOCTL_CMD_ANTDIV, 0, 0).await;

        // self.set_iovar_u32("bus:txglom", 0).await;
        // Timer::after(Duration::from_millis(100)).await;
        // //self.set_iovar_u32("apsta", 1).await; // this crashes, also we already did it before...??
        // //Timer::after(Duration::from_millis(100)).await;
        // self.set_iovar_u32("ampdu_ba_wsize", 8).await;
        // Timer::after(Duration::from_millis(100)).await;
        // self.set_iovar_u32("ampdu_mpdu", 4).await;
        // Timer::after(Duration::from_millis(100)).await;
        // //self.set_iovar_u32("ampdu_rx_factor", 0).await; // this crashes

        // // set wifi up
        // self.ioctl(ControlType::Set, IOCTL_CMD_UP, 0, &mut []).await;

        // Timer::after(Duration::from_millis(100)).await;

        // self.ioctl_set_u32(110, 0, 1).await; // SET_GMODE = auto
        // self.ioctl_set_u32(142, 0, 0).await; // SET_BAND = any

        Ok(())
    }

    pub async fn join_open(&mut self, ssid: &str) -> Result<(), Error> {
        let config_id = 0;

        self.send_edm(ExecWifiStationAction {
            config_id,
            action: WifiStationAction::Reset,
        })
        .await?;

        self.send_edm(SetWifiStationConfig {
            config_id,
            config_param: WifiStationConfig::SSID(heapless::String::from(ssid)),
        })
        .await?;

        self.send_edm(SetWifiStationConfig {
            config_id,
            config_param: WifiStationConfig::Authentication(Authentication::Open),
        })
        .await?;

        self.send_edm(ExecWifiStationAction {
            config_id,
            action: WifiStationAction::Activate,
        })
        .await?;

        with_timeout(Duration::from_secs(10), self.wait_for_join(ssid))
            .await
            .map_err(|_| Error::Timeout)??;

        Ok(())
    }

    pub async fn join_wpa2(&mut self, ssid: &str, passphrase: &str) -> Result<(), Error> {
        let config_id = 0;

        self.send_edm(ExecWifiStationAction {
            config_id,
            action: WifiStationAction::Reset,
        })
        .await?;

        self.send_edm(SetWifiStationConfig {
            config_id,
            config_param: WifiStationConfig::SSID(heapless::String::from(ssid)),
        })
        .await?;

        self.send_edm(SetWifiStationConfig {
            config_id,
            config_param: WifiStationConfig::Authentication(Authentication::WpaWpa2Psk),
        })
        .await?;

        self.send_edm(SetWifiStationConfig {
            config_id,
            config_param: WifiStationConfig::WpaPskOrPassphrase(heapless::String::from(passphrase)),
        })
        .await?;

        self.send_edm(ExecWifiStationAction {
            config_id,
            action: WifiStationAction::Activate,
        })
        .await?;

        with_timeout(Duration::from_secs(10), self.wait_for_join(ssid))
            .await
            .map_err(|_| Error::Timeout)??;

        Ok(())
    }

    async fn wait_for_join(&mut self, ssid: &str) -> Result<(), Error> {
        loop {
            let status = self
                .send_edm(GetWifiStatus {
                    status_id: StatusId::Status,
                })
                .await?;

            if matches!(
                status.status_id,
                WifiStatus::Status(WifiStatusVal::Connected)
            ) {
                let connected_ssid = self
                    .send_edm(GetWifiStatus {
                        status_id: StatusId::SSID,
                    })
                    .await?;

                match connected_ssid.status_id {
                    WifiStatus::SSID(s) if s.as_str() == ssid => {
                        self.state_ch.set_link_state(LinkState::Up);
                        defmt::debug!("JOINED");
                        return Ok(());
                    }
                    _ => return Err(Error::Network),
                }
            }

            Timer::after(Duration::from_millis(500)).await;
        }
    }

    pub async fn gpio_set(&mut self, id: GPIOId, value: GPIOValue) -> Result<(), Error> {
        self.send_edm(WriteGPIO { id, value }).await?;
        Ok(())
    }

    // pub async fn start_ap_open(&mut self, ssid: &str, channel: u8) {
    //     self.start_ap(ssid, "", Security::OPEN, channel).await;
    // }

    // pub async fn start_ap_wpa2(&mut self, ssid: &str, passphrase: &str, channel: u8) {
    //     self.start_ap(ssid, passphrase, Security::WPA2_AES_PSK, channel)
    //         .await;
    // }

    // async fn start_ap(&mut self, ssid: &str, passphrase: &str, security: Security, channel: u8) {
    //     if security != Security::OPEN
    //         && (passphrase.as_bytes().len() < MIN_PSK_LEN
    //             || passphrase.as_bytes().len() > MAX_PSK_LEN)
    //     {
    //         panic!("Passphrase is too short or too long");
    //     }

    //     // Temporarily set wifi down
    //     self.ioctl(ControlType::Set, IOCTL_CMD_DOWN, 0, &mut [])
    //         .await;

    //     // Turn off APSTA mode
    //     self.set_iovar_u32("apsta", 0).await;

    //     // Set wifi up again
    //     self.ioctl(ControlType::Set, IOCTL_CMD_UP, 0, &mut []).await;

    //     // Turn on AP mode
    //     self.ioctl_set_u32(IOCTL_CMD_SET_AP, 0, 1).await;

    //     // Set SSID
    //     let mut i = SsidInfoWithIndex {
    //         index: 0,
    //         ssid_info: SsidInfo {
    //             len: ssid.as_bytes().len() as _,
    //             ssid: [0; 32],
    //         },
    //     };
    //     i.ssid_info.ssid[..ssid.as_bytes().len()].copy_from_slice(ssid.as_bytes());
    //     self.set_iovar("bsscfg:ssid", &i.to_bytes()).await;

    //     // Set channel number
    //     self.ioctl_set_u32(IOCTL_CMD_SET_CHANNEL, 0, channel as u32)
    //         .await;

    //     // Set security
    //     self.set_iovar_u32x2("bsscfg:wsec", 0, (security as u32) & 0xFF)
    //         .await;

    //     if security != Security::OPEN {
    //         self.set_iovar_u32x2("bsscfg:wpa_auth", 0, 0x0084).await; // wpa_auth = WPA2_AUTH_PSK | WPA_AUTH_PSK

    //         Timer::after(Duration::from_millis(100)).await;

    //         // Set passphrase
    //         let mut pfi = PassphraseInfo {
    //             len: passphrase.as_bytes().len() as _,
    //             flags: 1, // WSEC_PASSPHRASE
    //             passphrase: [0; 64],
    //         };
    //         pfi.passphrase[..passphrase.as_bytes().len()].copy_from_slice(passphrase.as_bytes());
    //         self.ioctl(
    //             ControlType::Set,
    //             IOCTL_CMD_SET_PASSPHRASE,
    //             0,
    //             &mut pfi.to_bytes(),
    //         )
    //         .await;
    //     }

    //     // Change mutlicast rate from 1 Mbps to 11 Mbps
    //     self.set_iovar_u32("2g_mrate", 11000000 / 500000).await;

    //     // Start AP
    //     self.set_iovar_u32x2("bss", 0, 1).await; // bss = BSS_UP
    // }

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
        self.at_client
            .lock()
            .await
            .send_retry::<Cmd, LEN>(&cmd)
            .await
    }
}
