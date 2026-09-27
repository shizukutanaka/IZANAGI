//! OpenWrt/Broadcom TRX firmware image parsing.
//!
//! Little-endian `struct trx_header`: magic `HDR0` (`0x30524448`),
//! `len` @4 (total image size), `crc32` @8 (over bytes 12..len),
//! `flag_ver` @12 (`flags<<16 | version`), then three partition
//! `offsets` @16..28. Offset 0 = absent.
//!
//! ```
//! use izanagi_kit::{crc, trx};
//! let mut d = vec![0u8; 28 + 8];
//! d[0..4].copy_from_slice(b"HDR0");
//! d[4..8].copy_from_slice(&36u32.to_le_bytes()); // len
//! d[16..20].copy_from_slice(&28u32.to_le_bytes()); // offset[0] -> partition 1
//! let c = crc::crc32(&d[12..36]);
//! d[8..12].copy_from_slice(&c.to_le_bytes());
//! let t = trx::parse(&d).unwrap();
//! assert_eq!(t.offsets[0], Some(28));
//! ```

use crate::crc;

/// `HDR0` magic (little-endian).
pub const MAGIC: u32 = 0x3052_4448;

/// TRX header size.
pub const HEADER_LEN: usize = 28;

/// A parsed TRX header.
#[derive(Clone, Debug, PartialEq)]
pub struct Trx {
    /// `len` — declared total image length (≤ input).
    pub len: u32,
    /// `flag_ver >> 16` — flags field.
    pub flags: u16,
    /// `flag_ver & 0xFFFF` — format version.
    pub version: u16,
    /// Up-to-three partition offsets within the file (0 = absent).
    pub offsets: [Option<u32>; 3],
}

fn u32le(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at.checked_add(4)?)?;
    Some(s[0] as u32 | (s[1] as u32) << 8 | (s[2] as u32) << 16 | (s[3] as u32) << 24)
}

/// Parses a TRX header: magic, `len` ≤ input size, and CRC over
/// bytes 12..len must validate.
pub fn parse(d: &[u8]) -> Option<Trx> {
    if d.len() < HEADER_LEN {
        return None;
    }
    if u32le(d, 0)? != MAGIC {
        return None;
    }
    let len = u32le(d, 4)? as usize;
    if len < HEADER_LEN || len > d.len() {
        return None;
    }
    let crc32 = u32le(d, 8)?;
    if crc::crc32(d.get(12..len)?) != crc32 {
        return None;
    }
    let flag_ver = u32le(d, 12)?;
    let mut offsets = [None; 3];
    for (i, slot) in offsets.iter_mut().enumerate() {
        let v = u32le(d, 16 + i * 4)?;
        if v != 0 {
            if v as usize >= len {
                return None;
            }
            *slot = Some(v);
        }
    }
    Some(Trx {
        len: len as u32,
        flags: (flag_ver >> 16) as u16,
        version: flag_ver as u16,
        offsets,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0u8; 28 + 16];
        d[0..4].copy_from_slice(b"HDR0");
        d[4..8].copy_from_slice(&44u32.to_le_bytes());
        d[12..16].copy_from_slice(&((1u32 << 16) | 2).to_le_bytes()); // flags=1 ver=2
        d[16..20].copy_from_slice(&28u32.to_le_bytes());
        d[20..24].copy_from_slice(&36u32.to_le_bytes());
        let c = crc::crc32(&d[12..44]);
        d[8..12].copy_from_slice(&c.to_le_bytes());
        d
    }

    #[test]
    fn parses_header() {
        let t = parse(&fixture()).unwrap();
        assert_eq!(t.len, 44);
        assert_eq!(t.flags, 1);
        assert_eq!(t.version, 2);
        assert_eq!(t.offsets, [Some(28), Some(36), None]);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 20]).is_none());
        let mut d = fixture();
        d[0] = b'X';
        assert!(parse(&d).is_none());
        let mut d = fixture();
        d[8] ^= 1; // crc
        assert!(parse(&d).is_none());
        let d = fixture();
        assert!(parse(&d[..30]).is_none()); // len past input
    }
}
