
macro_rules! setup_test_env {
  () => {
  extern crate env_logger;
  extern crate std;

  extern crate nb;

  // Note this useful idiom: importing names from outer (for mod tests) scope.
  use super::*;

  use heapless::{consts::*, spsc::Queue, String};
  #[allow(unused_imports)]
  use log::{error, info, warn};

  use env_logger::Env;
  use std::sync::Once;
  use crate::wifi;


  static INIT: Once = Once::new();

  struct Seconds(u32);
  trait U32Ext {
    fn s(self) -> Seconds;
  }
  impl U32Ext for u32 {
    fn s(self) -> Seconds {
      Seconds(self)
    }
  }

  struct Timer6;
  impl embedded_hal::timer::CountDown for Timer6 {
    type Time = Seconds;
    fn start<T>(&mut self, _: T)
    where
      T: Into<Seconds>,
    {
    }
    fn wait(&mut self) -> ::nb::Result<(), void::Void> {
      Ok(())
    }
  }

  };
}

macro_rules! setup_test_case {
  () => {{
    INIT.call_once(|| {
      env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .is_test(true)
        .init();
    });

    static mut WIFI_CMD_Q: Option<Queue<Command, U10, u8>> = None;
    static mut WIFI_RESP_Q: Option<Queue<Result<ResponseType, at::Error>, U10, u8>> = None;

    unsafe { WIFI_CMD_Q = Some(Queue::u8()) };
    unsafe { WIFI_RESP_Q = Some(Queue::u8()) };

    let (wifi_cmd_p, wifi_cmd_c) = unsafe { WIFI_CMD_Q.as_mut().unwrap().split() };
    let (wifi_resp_p, wifi_resp_c) = unsafe { WIFI_RESP_Q.as_mut().unwrap().split() };

    let wifi_client = at::client::ATClient::new((wifi_cmd_p, wifi_resp_c), 1000, Timer6);

    (wifi_client, (wifi_cmd_c, wifi_resp_p))
  }};
}

macro_rules! cleanup_test_case {
  ($connection: expr, $cmd_c: expr) => {
    let wifi_client = $connection.unwrap().disconnect();
    let (_, mut wifi_resp_c) = wifi_client.release();
    assert!(wifi_resp_c.dequeue().is_none());
    assert!($cmd_c.dequeue().is_none());
  };
}
