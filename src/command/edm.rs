
use crate::command::*;


const EDM_START = "\xAA";
const EDM_STOP = "\x55";

const CONNECT_EVENT = "\x00\x11";
const DISCONNECT_EVENT = "\x00\x21";
const DATA_EVENT = "\x00\x31";
const DATA_COMMAND = "\x00\x36";
const AT_EVENT = "\x00\x41";
const AT_REQUEST = "\x00\x44";
const AT_CONFIRMATION = "\x00\x45";
const RESEND_CONNECT_EVENT_COMMAND = "\x00\x56";
const IPHONE_EVENT = "\x00\x61";
const START_EVENT = "\x00\x71";

const BLUETOOTH = "\x01";
const IPv4 = "\x02";
const IPv6 = "\x03";

const SPP = "\x00";
const DUN = "\x01";
const SPS = "\x0E";

pub fn send_at_command(cmd: Command) {

}


pub fn check_for_incoming_edm_packet() {

}

pub fn send_edm_packet() {

}

pub fn generate_edm_at_request_payload() {

}

pub fn generate_edm_data_payload() {

}

pub fn generate_edm_resend_connect_events_payload() {

}
