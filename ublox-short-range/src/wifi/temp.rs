// use no_std_net::{IpAddr, Ipv4Addr, Ipv6Addr};
// use ufmt::{uDisplay, uDebug, Formatter, uWrite, uwrite};




// //Wrapper type for IpAddr for ufmt
// // pub struct IpAddrFmt(IpAddrFmt);

// pub enum IpAddrFmt{
//     V4(Ipv4Fmt),
//     V6(Ipv6Fmt),
// }

// pub struct Ipv4Fmt(Ipv4Addr);
// pub struct Ipv6Fmt(Ipv6Addr);



// impl uDebug for IpAddrFmt {

//     fn fmt<W>(&self, f: &mut Formatter<'_, W>) -> Result<(), W::Error>
//     where
//         W: uWrite + ?Sized
//     {
//         match *self {
//             IpAddrFmt::V4(ref a) => uDebug::fmt(a, f),
//             IpAddrFmt::V6(ref a) => uDebug::fmt(a, f),
//         }
//     }
// }

// impl uDisplay for Ipv4Fmt {
//     fn fmt<W>(&self, fmt: &mut Formatter<'_, W>) -> Result<(), W::Error>
//     where
//         W: uWrite + ?Sized
//     {
//         self.0.
//         uwrite!(
//             fmt,
//             "{}.{}.{}.{}",
//             self.0.inner[0], self.0.inner[1], self.0.inner[2], self.0.inner[3]
//         )
//     }
// }

// impl uDebug for Ipv4Fmt {
//     fn fmt<W>(&self, f: &mut Formatter<'_, W>) -> Result<(), W::Error>
//     where
//         W: uWrite + ?Sized
//     {
//         uDisplay::fmt(self, f)
//     }
// }

// impl uDisplay for Ipv6Fmt {
//     fn fmt<W>(&self, fmt: &mut Formatter<'_, W>) -> Result<(), W::Error>
//     where
//         W: uWrite + ?Sized
//     {
//         Ok(())
//         // match self.0.segments() {
//         //     // We need special cases for :: and ::1, otherwise they're formatted
//         //     // as ::0.0.0.[01]
//         //     [0, 0, 0, 0, 0, 0, 0, 0] => uwrite!(fmt, "::"),
//         //     [0, 0, 0, 0, 0, 0, 0, 1] => uwrite!(fmt, "::1"),
//         //     // Ipv4 Compatible address
//         //     [0, 0, 0, 0, 0, 0, g, h] => uwrite!(
//         //         fmt,
//         //         "::{}.{}.{}.{}",
//         //         (g >> 8) as u8,
//         //         g as u8,
//         //         (h >> 8) as u8,
//         //         h as u8
//         //     ),
//         //     // Ipv4-Mapped address
//         //     [0, 0, 0, 0, 0, 0xffff, g, h] => uwrite!(
//         //         fmt,
//         //         "::ffff:{}.{}.{}.{}",
//         //         (g >> 8) as u8,
//         //         g as u8,
//         //         (h >> 8) as u8,
//         //         h as u8
//         //     ),
//         //     _ => {
//         //         fn find_zero_slice(segments: &[u16; 8]) -> (usize, usize) {
//         //             let mut longest_span_len = 0;
//         //             let mut longest_span_at = 0;
//         //             let mut cur_span_len = 0;
//         //             let mut cur_span_at = 0;

//         //             for i in 0..8 {
//         //                 if segments[i] == 0 {
//         //                     if cur_span_len == 0 {
//         //                         cur_span_at = i;
//         //                     }

//         //                     cur_span_len += 1;

//         //                     if cur_span_len > longest_span_len {
//         //                         longest_span_len = cur_span_len;
//         //                         longest_span_at = cur_span_at;
//         //                     }
//         //                 } else {
//         //                     cur_span_len = 0;
//         //                     cur_span_at = 0;
//         //                 }
//         //             }

//         //             (longest_span_at, longest_span_len)
//         //         }

//         //         let (zeros_at, zeros_len) = find_zero_slice(&self.0.segments());

//         //         if zeros_len > 1 {
//         //             fn fmt_subslice(segments: &[u16], fmt: &mut Formatter) -> Result {
//         //                 if !segments.is_empty() {
//         //                     uwrite!(fmt, "{:x}", segments[0])?;
//         //                     for &seg in &segments[1..] {
//         //                         uwrite!(fmt, ":{:x}", seg)?;
//         //                     }
//         //                 }
//         //                 Ok(())
//         //             }

//         //             fmt_subslice(&self.0.segments()[..zeros_at], fmt)?;
//         //             fmt.write_str("::")?;
//         //             fmt_subslice(&self.0.segments()[zeros_at + zeros_len..], fmt)
//         //         } else {
//         //             let &[a, b, c, d, e, f, g, h] = &self.0.segments();
//         //             uwrite!(
//         //                 fmt,
//         //                 "{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}",
//         //                 a, b, c, d, e, f, g, h
//         //             )
//         //         }
//         //     }
//         // }
//     }
// }

// impl uDebug for Ipv6Fmt {
//     fn fmt<W>(&self, fmt: &mut Formatter<'_, W>) -> Result<(), W::Error>
//     where
//         W: uWrite + ?Sized
//     {
//         uDisplay::fmt(self, fmt)
//     }
// }