use embedded_hal::digital::OutputPin;

pub trait WifiConfig<'a> {
    type ResetPin: OutputPin;

    const FLOW_CONTROL: bool = false;

    const TLS_IN_BUFFER_SIZE: Option<u16> = None;
    const TLS_OUT_BUFFER_SIZE: Option<u16> = None;

    #[cfg(feature = "ppp")]
    const PPP_CONFIG: embassy_net_ppp::Config<'a>;

    fn reset_pin(&mut self) -> Option<&mut Self::ResetPin> {
        None
    }
}
