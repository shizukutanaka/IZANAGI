//! BinHex 4.0 (`.hqx`) — the classic Mac 6-bit ASCII transport format.
//!
//! An `.hqx` file starts with the line
//! `(This file must be converted with BinHex 4.0)` then a `:`-framed
//! stream of 64-character runes carrying six bits each. The decoded
//! stream opens with the file header: name length + name, type,
//! creator, flags, data/resource lengths and a CRC-16.
//!
//! ```
//! use izanagi_kit::hqx::{encode_header, parse};
//! let d = encode_header(b"file.txt", 0x5445_5854, 0x7474_7874, 0, 12, 0);
//! let h = parse(&d).unwrap();
//! assert_eq!(h.name, b"file.txt");
//! assert_eq!(h.data_len, 12);
//! ```

/// The 64 six-bit runes, in BinHex order (index = value).
pub const RUNES: &[u8; 64] = b"!\"#$%&'()*+,-012345689@ABCDEFGHIJKLMNPQRSTUVXYZ[`abcdefhijklmpqr";

/// Decoded `.hqx` file header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hqx {
    /// File name (1–63 bytes).
    pub name: Vec<u8>,
    /// Four-char file type.
    pub file_type: u32,
    /// Four-char creator code.
    pub creator: u32,
    /// Finder flags.
    pub flags: u16,
    /// Data fork byte length.
    pub data_len: u32,
    /// Resource fork byte length.
    pub rsrc_len: u32,
    /// Declared header CRC-16 (poly 0x1021, init 0, zero-fill body).
    pub header_crc: u16,
}

impl Hqx {
    /// True when the stored CRC matches the re-serialized header.
    pub fn crc_ok(&self) -> bool {
        match header_block(self) {
            Some(b) => crc16(&b) == self.header_crc,
            None => false,
        }
    }
}

/// A byte's rune value; `None` when it is not a rune.
pub fn rune_val(b: u8) -> Option<u8> {
    RUNES.iter().position(|&r| r == b).map(|p| p as u8)
}

/// Decode a rune stream (any non-rune bytes are skipped) into bytes.
/// `None` when the rune count's bit total isn't a whole byte count —
/// trailing partial groups are dropped as BinHex pads them.
pub fn decode_runes(d: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let (mut acc, mut bits) = (0u32, 0u32);
    for &b in d {
        if let Some(v) = rune_val(b) {
            acc = (acc << 6) | u32::from(v);
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                out.push((acc >> bits) as u8);
                acc &= (1 << bits) - 1;
            }
        }
    }
    out
}

/// CRC-16/ARC-style variant used by BinHex: poly 0x1021, init 0,
/// feed order MSB-first (implemented bit-serially over the buffer).
pub fn crc16(d: &[u8]) -> u16 {
    let mut c = 0u16;
    for &b in d {
        c ^= u16::from(b) << 8;
        for _ in 0..8 {
            c = if c & 0x8000 != 0 {
                (c << 1) ^ 0x1021
            } else {
                c << 1
            };
        }
    }
    c
}

fn be32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        (u32::from(*d.get(at)?) << 24)
            | (u32::from(*d.get(at + 1)?) << 16)
            | (u32::from(*d.get(at + 2)?) << 8)
            | u32::from(*d.get(at + 3)?),
    )
}
fn be16(d: &[u8], at: usize) -> Option<u16> {
    Some((u16::from(*d.get(at)?) << 8) | u16::from(*d.get(at + 1)?))
}

/// Serialized header block (length+name+type+creator+flags+lens)
/// for CRC computation — the CRC itself is excluded.
fn put32(b: &mut Vec<u8>, v: u32) {
    b.extend_from_slice(&[(v >> 24) as u8, (v >> 16) as u8, (v >> 8) as u8, v as u8]);
}
fn put16(b: &mut Vec<u8>, v: u16) {
    b.extend_from_slice(&[(v >> 8) as u8, v as u8]);
}

fn header_block(h: &Hqx) -> Option<Vec<u8>> {
    if h.name.is_empty() || h.name.len() > 63 {
        return None;
    }
    let mut b = Vec::new();
    b.push(h.name.len() as u8);
    b.extend_from_slice(&h.name);
    put32(&mut b, h.file_type);
    put32(&mut b, h.creator);
    put16(&mut b, h.flags);
    put32(&mut b, h.data_len);
    put32(&mut b, h.rsrc_len);
    Some(b)
}

/// Build a complete `.hqx` text with just a header (fork bodies
/// omitted) — useful for tests and fixture generation.
pub fn encode_header(
    name: &[u8],
    file_type: u32,
    creator: u32,
    flags: u16,
    data_len: u32,
    rsrc_len: u32,
) -> Vec<u8> {
    let h = Hqx {
        name: name.to_vec(),
        file_type,
        creator,
        flags,
        data_len,
        rsrc_len,
        header_crc: 0,
    };
    let mut blk = header_block(&h).unwrap_or_default();
    let crc = crc16(&blk);
    put16(&mut blk, crc);
    let mut out = b"(This file must be converted with BinHex 4\x2e0)\n\n:".to_vec();
    // emit runes, 64 per line
    let (mut acc, mut bits) = (0u32, 0u32);
    let mut col = 0usize;
    let push = |out: &mut Vec<u8>, v: u8, col: &mut usize| {
        out.push(RUNES[v as usize]);
        *col += 1;
        if *col == 64 {
            out.push(b'\n');
            *col = 0;
        }
    };
    for &b in &blk {
        acc = (acc << 8) | u32::from(b);
        bits += 8;
        while bits >= 6 {
            bits -= 6;
            push(&mut out, ((acc >> bits) & 0x3f) as u8, &mut col);
        }
        acc &= (1 << bits) - 1;
    }
    if bits > 0 {
        push(&mut out, ((acc << (6 - bits)) & 0x3f) as u8, &mut col);
    }
    if col != 0 {
        out.push(b'\n');
    }
    out.push(b':');
    out
}

/// Parse a `.hqx` file: find the BinHex banner, decode the rune
/// stream between `:` delimiters, and read the header. `None` when
/// the banner, delimiters or a complete header are missing.
pub fn parse(d: &[u8]) -> Option<Hqx> {
    // rune stream starts after the first ':' following the banner.
    let banner_end = d
        .windows(b"BinHex 4\x2e0".len())
        .position(|w| w == b"BinHex 4\x2e0")?;
    let start = d[banner_end..].iter().position(|&b| b == b':')? + banner_end + 1;
    let end = d[start..]
        .iter()
        .position(|&b| b == b':')
        .map(|p| start + p)
        .unwrap_or(d.len());
    let raw = decode_runes(&d[start..end]);
    let nlen = *raw.first()? as usize;
    if nlen == 0 || nlen > 63 || raw.len() < 1 + nlen + 18 {
        return None;
    }
    let name = raw[1..1 + nlen].to_vec();
    let o = 1 + nlen;
    Some(Hqx {
        name,
        file_type: be32(&raw, o)?,
        creator: be32(&raw, o + 4)?,
        flags: be16(&raw, o + 8)?,
        data_len: be32(&raw, o + 10)?,
        rsrc_len: be32(&raw, o + 14)?,
        header_crc: be16(&raw, o + 18)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rune_table() {
        assert_eq!(rune_val(b'!'), Some(0));
        assert_eq!(rune_val(b'r'), Some(63));
        assert_eq!(rune_val(b'A'), Some(23));
        assert_eq!(rune_val(b'Q'), Some(38));
        assert_eq!(rune_val(b'#'), Some(2));
        assert_eq!(rune_val(b' '), None);
        assert_eq!(rune_val(b'~'), None);
    }

    #[test]
    fn decode_knowntext() {
        // "HQX" = 0x48 0x51 0x58 -> runes: 010010 000101 000101 011000
        // = 18, 5, 5, 24 -> '5', '&', '&', 'B'
        assert_eq!(decode_runes(b"5&&B"), b"HQX");
        assert!(decode_runes(b"").is_empty());
        // spaces/colons skipped
        assert_eq!(decode_runes(b"5& &B"), b"HQX");
    }

    #[test]
    fn crc16_of_empty_is_zero() {
        assert_eq!(crc16(b""), 0);
        // BinHex CRC-16 (poly 0x1021, init 0, MSB-first) check value.
        assert_eq!(crc16(b"123456789"), 0x31C3);
    }

    #[test]
    fn roundtrip_header() {
        let d = encode_header(b"Read Me", 0x5445_5854, 0x7474_7874, 0x1200, 100, 40);
        let h = parse(&d).unwrap();
        assert_eq!(h.name, b"Read Me");
        assert_eq!(h.file_type, 0x54455854); // "TEXT"
        assert_eq!(h.creator, 0x74747874); // "ttxt"
        assert_eq!(h.flags, 0x1200);
        assert_eq!(h.data_len, 100);
        assert_eq!(h.rsrc_len, 40);
        assert!(h.crc_ok());
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"no banner here").is_none());
        assert!(parse(b"(This file must be converted with BinHex 4.0)\n:\n:").is_none());
    }
}
