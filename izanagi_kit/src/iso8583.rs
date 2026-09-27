//! ISO 8583 financial message header parsing (1987/1993 style).
//!
//! Layout: `MTI` (4 ASCII digits: version | class | function |
//! origin), primary bitmap (8 bytes, hex or raw), optional
//! secondary bitmap (8 more bytes when bit 1 set). This module
//! decodes the MTI and the bitmap into a set of present field
//! numbers — field bodies are length-coded per implementation and
//! left to the caller.
//!
//! ```
//! use izanagi_kit::iso8583;
//! let mut d = b"0200".to_vec(); // v0 auth request
//! d.extend_from_slice(&[0x30, 0x20, 0, 0, 0, 0, 0, 0x80]); // fields 3,4,11,57
//! let m = iso8583::parse(&d).unwrap();
//! assert_eq!(m.mti, "0200");
//! assert_eq!(m.fields, vec![3, 4, 11, 57]);
//! ```

use std::string::String;
use std::vec::Vec;

/// A parsed ISO 8583 header.
#[derive(Clone, Debug, PartialEq)]
pub struct Iso8583 {
    /// MTI as 4 ASCII digits.
    pub mti: String,
    /// Version digit (0 or 1 in practice).
    pub version: u8,
    /// Message class (2 = financial, 4 = reversal, ...).
    pub class: u8,
    /// Field numbers (1..=128) marked present by the bitmap(s).
    pub fields: Vec<u8>,
    /// Byte offset of the first data element.
    pub data_at: usize,
}

/// Parses MTI + bitmaps. `hex_bitmap=true` interprets each bitmap as
/// 16 ASCII hex chars instead of 8 raw bytes.
pub fn parse_hex(d: &[u8], hex_bitmap: bool) -> Option<Iso8583> {
    if d.len() < 4 {
        return None;
    }
    let mti_b = d.get(..4)?;
    if !mti_b.iter().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let mti = String::from_utf8_lossy(mti_b).into_owned();
    let mut at = 4usize;
    let read8 = |d: &[u8], at: usize| -> Option<[u8; 8]> {
        if hex_bitmap {
            let s = d.get(at..at.checked_add(16)?)?;
            if !s.iter().all(|b| b.is_ascii_hexdigit()) {
                return None;
            }
            let mut out = [0u8; 8];
            for (i, w) in s.chunks(2).enumerate() {
                let hi = (w[0] as char).to_digit(16)? as u8;
                let lo = (w[1] as char).to_digit(16)? as u8;
                out[i] = hi << 4 | lo;
            }
            Some(out)
        } else {
            let s = d.get(at..at.checked_add(8)?)?;
            let mut out = [0u8; 8];
            out.copy_from_slice(s);
            Some(out)
        }
    };
    let primary = read8(d, at)?;
    at += if hex_bitmap { 16 } else { 8 };
    let mut fields = Vec::new();
    let push_bits = |bm: &[u8; 8], base: u8, skip_first: bool, fields: &mut Vec<u8>| {
        for (i, byte) in bm.iter().enumerate() {
            for bit in 0..8u8 {
                let n = base + (i as u8) * 8 + bit + 1;
                if byte & (0x80 >> bit) != 0 && !(skip_first && n == base + 1) {
                    fields.push(n);
                }
            }
        }
    };
    let secondary = primary[0] & 0x80 != 0;
    push_bits(&primary, 0, secondary, &mut fields);
    if secondary {
        let sec = read8(d, at)?;
        at += if hex_bitmap { 16 } else { 8 };
        push_bits(&sec, 64, false, &mut fields);
    }
    Some(Iso8583 {
        version: mti_b[0] - b'0',
        class: mti_b[1] - b'0',
        mti,
        fields,
        data_at: at,
    })
}

/// Parses an ISO 8583 message with raw (8-byte binary) bitmaps.
pub fn parse(d: &[u8]) -> Option<Iso8583> {
    parse_hex(d, false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;

    #[test]
    fn parses_primary_only() {
        // byte0 0x20 → field 3; byte1 0x20 → field 11
        let mut d = b"0200".to_vec();
        d.extend_from_slice(&[0x20, 0x20, 0, 0, 0, 0, 0, 0]);
        let m = parse(&d).unwrap();
        assert_eq!(m.mti, "0200");
        assert_eq!(m.fields, vec![3, 11]);
        assert_eq!(m.data_at, 12);
    }

    #[test]
    fn parses_secondary() {
        let mut d = b"1200".to_vec();
        d.extend_from_slice(&[0x80, 0, 0, 0, 0, 0, 0, 0]); // bit1 only → secondary
        d.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0x40]); // field 128-6=... bit: last byte 0x40 → bit 122? byte7 bit1 → field 64+56+2=122
        let m = parse(&d).unwrap();
        assert_eq!(m.version, 1);
        assert_eq!(m.fields, vec![122]);
        assert_eq!(m.data_at, 20);
    }

    #[test]
    fn hex_mode_and_rejects() {
        let mut d = b"0200".to_vec();
        d.extend_from_slice(b"2000000000000000");
        let m = parse_hex(&d, true).unwrap();
        assert_eq!(m.fields, vec![3]);
        assert!(parse(b"xxxx").is_none());
        assert!(parse(b"0200").is_none());
    }
}
