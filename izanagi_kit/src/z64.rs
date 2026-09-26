//! Nintendo 64 ROM header (`.z64` / `.v64` / `.n64`).
//!
//! Three byte orders exist in the wild: big-endian z64 (`8037 1240`),
//! byte-swapped v64 (`3780 4012`), and little-endian n64 (`4012 3780`).
//! `parse` detects the order from the magic word and decodes the header
//! accordingly.
//!
//! ```
//! use izanagi_kit::z64::{parse, ByteOrder};
//!
//! let mut d = vec![0u8; 0x1000];
//! d[0..4].copy_from_slice(&[0x80, 0x37, 0x12, 0x40]);
//! d[4..8].copy_from_slice(&60u32.to_be_bytes());   // clock
//! d[8..12].copy_from_slice(&0x8000_0400u32.to_be_bytes()); // PC
//! d[0x20..0x33].copy_from_slice(b"TEST ROM\0\0\0\0\0\0\0\0\0\0\0");
//! d[0x3B..0x3F].copy_from_slice(b"NSME");
//! d[0x3F] = 0;
//! let r = parse(&d).unwrap();
//! assert_eq!(r.order, ByteOrder::Z64);
//! assert_eq!(r.name, "TEST ROM");
//! assert_eq!(r.serial, "NSME");
//! assert_eq!(r.country, 'E');
//! ```

use std::string::String;

/// Parsed header length on disk (the identifiable region).
pub const HEADER: usize = 0x40;

/// Detected byte order of the image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ByteOrder {
    /// Standard big-endian image.
    Z64,
    /// Byte-swapped image (words' halves exchanged).
    V64,
    /// Little-endian image.
    N64,
}

fn be32(d: &[u8], at: usize) -> u32 {
    u32::from(d[at]) << 24
        | u32::from(d[at + 1]) << 16
        | u32::from(d[at + 2]) << 8
        | u32::from(d[at + 3])
}

/// A decoded N64 ROM header.
#[derive(Clone, Debug, PartialEq)]
pub struct Z64 {
    /// Detected byte order.
    pub order: ByteOrder,
    /// PI BSD domain1 clock rate override.
    pub clock: u32,
    /// Program counter / boot address.
    pub pc: u32,
    /// Release address.
    pub release: u32,
    /// CRC1 checksum.
    pub crc1: u32,
    /// CRC2 checksum.
    pub crc2: u32,
    /// Image name (20 bytes, NUL/space trimmed).
    pub name: String,
    /// 4-char cartridge serial at `0x3B..0x3F` (e.g. `NSME`).
    pub serial: String,
    /// Country code = last char of the serial ('E' US, 'J' JP, 'P' PAL…).
    pub country: char,
    /// ROM version byte at 0x3F.
    pub version: u8,
}

/// Normalise `d[0..4]` into the canonical big-endian magic value,
/// returning the detected order.
fn magic_of(d: &[u8]) -> Option<ByteOrder> {
    match [d[0], d[1], d[2], d[3]] {
        [0x80, 0x37, 0x12, 0x40] => Some(ByteOrder::Z64),
        [0x37, 0x80, 0x40, 0x12] => Some(ByteOrder::V64),
        [0x40, 0x12, 0x37, 0x80] => Some(ByteOrder::N64),
        _ => None,
    }
}

/// Read the byte at logical address `a` for the given byte order:
/// v64 swaps u16 halves, n64 reverses each u32 word.
fn byte_at(order: ByteOrder, d: &[u8], a: usize) -> u8 {
    match order {
        ByteOrder::Z64 => d[a],
        ByteOrder::V64 => d[a ^ 1],
        ByteOrder::N64 => d[(a & !3) + (3 - (a & 3))],
    }
}

fn rd32(order: ByteOrder, d: &[u8], at: usize) -> u32 {
    match order {
        ByteOrder::Z64 => be32(d, at),
        ByteOrder::V64 => {
            u32::from(d[at + 1]) << 24
                | u32::from(d[at]) << 16
                | u32::from(d[at + 3]) << 8
                | u32::from(d[at + 2])
        }
        ByteOrder::N64 => {
            u32::from(d[at])
                | u32::from(d[at + 1]) << 8
                | u32::from(d[at + 2]) << 16
                | u32::from(d[at + 3]) << 24
        }
    }
}

fn name_of(order: ByteOrder, d: &[u8]) -> String {
    let mut raw = [0u8; 20];
    for (i, b) in raw.iter_mut().enumerate() {
        *b = byte_at(order, d, 0x20 + i);
    }
    let end = raw.iter().position(|&b| b == 0).unwrap_or(20);
    String::from_utf8_lossy(&raw[..end]).trim_end().to_string()
}

fn serial_of(order: ByteOrder, d: &[u8]) -> [u8; 4] {
    let mut s = [0u8; 4];
    for (i, b) in s.iter_mut().enumerate() {
        *b = byte_at(order, d, 0x3B + i);
    }
    s
}

/// Parse the ROM header. `None` when too short or the magic word is not
/// one of the three known encodings.
pub fn parse(d: &[u8]) -> Option<Z64> {
    if d.len() < HEADER {
        return None;
    }
    let order = magic_of(d)?;
    let serial = serial_of(order, d);
    let country = serial[3] as char;
    Some(Z64 {
        order,
        clock: rd32(order, d, 0x04),
        pc: rd32(order, d, 0x08),
        release: rd32(order, d, 0x0C),
        crc1: rd32(order, d, 0x10),
        crc2: rd32(order, d, 0x14),
        name: name_of(order, d),
        serial: String::from_utf8_lossy(&serial).into_owned(),
        country,
        version: byte_at(order, d, 0x3F),
    })
}

/// Byte-swap `d` in place from v64 into z64 ordering (pairs of bytes).
pub fn unswap_v64(d: &mut [u8]) {
    for pair in d.chunks_exact_mut(2) {
        pair.swap(0, 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn rom() -> Vec<u8> {
        let mut d = vec![0u8; 0x1000];
        d[0..4].copy_from_slice(&[0x80, 0x37, 0x12, 0x40]);
        d[4..8].copy_from_slice(&60u32.to_be_bytes());
        d[8..12].copy_from_slice(&0x8000_0400u32.to_be_bytes());
        d[0x10..0x14].copy_from_slice(&0xDEAD_BEEFu32.to_be_bytes());
        d[0x20..0x33].copy_from_slice(b"TEST ROM\0\0\0\0\0\0\0\0\0\0\0");
        d[0x3B..0x3F].copy_from_slice(b"NSME");
        d
    }

    #[test]
    fn z64_fields() {
        let r = parse(&rom()).unwrap();
        assert_eq!(r.order, ByteOrder::Z64);
        assert_eq!(r.clock, 60);
        assert_eq!(r.pc, 0x8000_0400);
        assert_eq!(r.crc1, 0xDEAD_BEEF);
        assert_eq!(r.name, "TEST ROM");
        assert_eq!(r.serial, "NSME");
        assert_eq!(r.country, 'E');
    }

    #[test]
    fn v64_and_n64_orders() {
        let mut v = rom();
        unswap_v64(&mut v);
        let r = parse(&v).unwrap();
        assert_eq!(r.order, ByteOrder::V64);
        assert_eq!(r.pc, 0x8000_0400);
        assert_eq!(r.name, "TEST ROM");
        assert_eq!(r.serial, "NSME");
        // n64 = little-endian 32-bit words
        let mut n = rom();
        for w in n.chunks_exact_mut(4) {
            w.swap(0, 3);
            w.swap(1, 2);
        }
        let r = parse(&n).unwrap();
        assert_eq!(r.order, ByteOrder::N64);
        assert_eq!(r.pc, 0x8000_0400);
        assert_eq!(r.serial, "NSME");
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        assert!(parse(&[0u8; 0x40]).is_none());
    }
}
