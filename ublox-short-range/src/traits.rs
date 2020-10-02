use atat::AtatClient;
use super::error::*;
use super::wifi::{
    connection::WifiConnection,
    network::WifiNetwork,
    options::{ConnectionOptions, HotspotOptions},
};

use embedded_hal::timer::{Cancel, CountDown};
use heapless::{Vec, consts, ArrayLength};




