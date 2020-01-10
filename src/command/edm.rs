use core::convert::TryInto;
use core::iter::FromIterator;
use heapless::{consts, Vec};
use packed_struct::prelude::*;

#[derive(PackedStruct, Debug, Clone)]
#[packed_struct(bit_numbering = "msb0", endian = "msb")]
pub struct PacketStart {
    #[packed_field(bits = "0:7")]
    start: Integer<u8, packed_bits::Bits8>,
    #[packed_field(bits = "8:11")]
    _reserved: ReservedZeroes<packed_bits::Bits4>,
    #[packed_field(bits = "12:23")]
    payload_length: Integer<u16, packed_bits::Bits12>,
    /// The payload starts with two header bytes to identify exactly the kind of data that is included in the payload
    /// Identifier  |   Type     |   Event, Indication, Response, Request, Confirmation or Command
    /// (12 bits)   | (4 bits)   |     (Payload Length - 2 bytes)
    #[packed_field(bits = "24:35", ty = "enum")]
    identifier: EnumCatchAll<Identifier>,
    #[packed_field(bits = "36:39", ty = "enum")]
    payload_type: EnumCatchAll<Type>,
}

/// A packet starts with a start byte (0xAA) and ends with a stop byte (0x55) for easy parsing and packet
/// re-synchronization.
/// The length of the payload is defined by 12 bits. Four bits are reserved for future use. Hence, the total
/// packet length is the payload length plus four (start and stop bytes plus reserved and length bits)
///
/// |    Start             |   Reserved  |   Payload Length  |   Payload       |   Stop             |
/// | -------------------- | ----------- | ----------------- | --------------- | ------------------ |
/// |    (1 byte = 0xAA)   |   (4 bits)  |   (12 bits)       |  (Length bytes) | (1 byte = 0x55)    |
#[derive(Clone)]
pub struct Packet {
    packed_start: PacketStart,
    payload: Vec<u8, consts::U1018>,
    stop: u8,
}

impl Packet {
    pub fn new(
        identifier: Identifier,
        payload_type: Type,
        payload: Vec<u8, consts::U1018>,
    ) -> Self {
        let length: u16 = payload.len().try_into().unwrap();
        let packed_start = PacketStart {
            start: 0xAA.into(),
            _reserved: Default::default(),
            payload_length: length.into(),
            identifier: identifier.into(),
            payload_type: payload_type.into(),
        };

        Packet {
            packed_start,
            payload,
            stop: 0x55,
        }
    }

    pub fn r#type(&self) -> Type {
        self.packed_start.payload_type.to_primitive().into()
    }

    pub fn identifier(&self) -> Identifier {
        self.packed_start.identifier.to_primitive().into()
    }

    pub fn payload_len(&self) -> u16 {
        self.packed_start.payload_length.to_primitive()
    }

    pub fn pack(self) -> Vec<u8, consts::U1024> {
        let mut res: Vec<u8, consts::U1024> =
            Vec::from_iter(self.packed_start.pack().iter().cloned());
        res.extend(self.payload.iter());
        res.push(self.stop).unwrap();
        res
    }

    pub fn unpack(packet: Vec<u8, consts::U1024>) -> Self {
        let packed_start = PacketStart::unpack_from_slice(&packet[0..5]).unwrap();
        Packet {
            packed_start,
            payload: Vec::new(),
            stop: 0x55,
        }
    }
}

impl core::fmt::Debug for Packet {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "")
    }
}

/// The Type identifies if the data is an event, indication, response, request, confirmation or command
#[derive(PrimitiveEnum_u8, Debug, Clone, Copy)]
pub enum Type {
    /// Transmitted by the module as a notification. It does not require a response from the host
    Event = 0x1,
    /// Transmitted by the module as a notification. The module expects a Response back from the host
    Indication = 0x2,
    /// If an Indication is received from the module, the host must respond with a Response
    Response = 0x3,
    /// A request is transmitted to the module to execute some functionality. The module must respond with a Confirmation
    Request = 0x4,
    /// A response to an executed Request
    Confirmation = 0x5,
    /// A command is transmitted to the module to execute some functionality. No response is expected
    Command = 0x6,
}

impl From<u8> for Type {
    fn from(v: u8) -> Type {
        match v {
            0x1 => Type::Event,
            0x2 => Type::Indication,
            0x3 => Type::Response,
            0x4 => Type::Request,
            0x5 => Type::Confirmation,
            0x6 => Type::Command,
            _ => unreachable!(),
        }
    }
}

/// The Identifier identifies what event, indication, response, request, confirmation or command is
/// transmitted or received. Currently, the following packets are defined
#[derive(PrimitiveEnum_u16, Debug, Clone, Copy)]
pub enum Identifier {
    /// Sent by the module to inform the host about a new connection
    Connect = 0x0010,
    /// Sent by the module to inform the host about the loss of connection
    Disconnect = 0x0020,
    /// Sent by the module when data is received over air
    Data = 0x0030,
    /// Sent to the module to send data over air. No acknowledge is transmitted by the module
    AT = 0x0040,
    /// Special packet to execute an AT command. One or many AT Confirmation packets are transmitted back by the module
    Resend = 0x0050,
    /// The module sends one or many confirmations as a response to an AT Request. The number of confirmation packets depends on what AT command that is being executed
    Iphone = 0x0060,
    /// There are a number of AT events that can be sent by the module. See the u-connect AT Commands Manual for details
    Start = 0x0070,
}

impl From<u16> for Identifier {
    fn from(v: u16) -> Identifier {
        match v {
            0x0010 => Identifier::Connect,
            0x0020 => Identifier::Disconnect,
            0x0030 => Identifier::Data,
            0x0040 => Identifier::AT,
            0x0050 => Identifier::Resend,
            0x0060 => Identifier::Iphone,
            0x0070 => Identifier::Start,
            _ => unreachable!(),
        }
    }
}
