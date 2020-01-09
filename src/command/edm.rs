use core::convert::TryInto;
use packed_struct::prelude::*;
use heapless::{ArrayLength, Vec};

/// A packet starts with a start byte (0xAA) and ends with a stop byte (0x55) for easy parsing and packet
/// re-synchronization.
/// The length of the payload is defined by 12 bits. Four bits are reserved for future use. Hence, the total
/// packet length is the payload length plus four (start and stop bytes plus reserved and length bits)
///     Start             |   Reserved  |   Payload Length  |   Payload       |   Stop
///     (1 byte = 0xAA)   |   (4 bits)  |   (12 bits)       |  (Length bytes) | (1 byte = 0x55)
#[derive(PackedStruct, Debug, Clone)]
#[packed_struct(bit_numbering = "msb0", endian = "msb")]
pub struct Packet {
    #[packed_field(bits = "0:7")]
    start: Integer<u8, packed_bits::Bits8>,
    // #[packed_field(bits="8:11")]
    // reserved: Integer<u8, packed_bits::Bits4>,
    #[packed_field(bits = "12:23")]
    payload_length: Integer<u16, packed_bits::Bits12>,
    /// The payload starts with two header bytes to identify exactly the kind of data that is included in the payload
    /// Identifier  |   Type     |   Event, Indication, Response, Request, Confirmation or Command
    /// (12 bits)   | (4 bits)   |     (Payload Length - 2 bytes)
    #[packed_field(bits = "24:35", ty = "enum")]
    identifier: EnumCatchAll<Identifier>,
    #[packed_field(bits = "36:39", ty = "enum")]
    payload_type: EnumCatchAll<Type>,
    // #[packed_field(element_size_bits = "8")]
    // payload: [u8; 128],
    #[packed_field]
    stop: Integer<u8, packed_bits::Bits8>,
}

// impl PackedStructInfo for [u8; N] {
//     #[inline]
//     fn packed_bits() -> usize {
//         $N * 8
//     }
// }

// impl Packet {
//     pub fn new(identifier: Identifier, payload_type: Type, payload: [u8; 1024]) -> Self {
//         let length: u16 = payload.len().try_into().unwrap();
//         Packet {
//             start: 0xAA.into(),
//             payload_length: length.into(),
//             identifier: identifier.into(),
//             payload_type: payload_type.into(),
//             payload,
//             stop: 0x55.into(),
//         }
//     }
// }

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
