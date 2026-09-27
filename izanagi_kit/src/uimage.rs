//! U-Boot legacy image (`uImage`) header parsing + header-CRC check.
//!
//! 64-byte big-endian header: magic `0x27051956`, `hcrc`, `time`,
//! `size`, `load`, `ep`, `dcrc`, `os`, `arch`, `type`, `comp`,
//! `name[32]`. `hcrc` covers the header with the `hcrc` field zeroed;
//! `size` must fit the trailing payload (payload CRC is verified by
//! U-Boot, kept verbatim here).
//!
//! ```
//! use izanagi_kit::{crc, uimage};
//! let mut d = vec![0u8; 68];
//! d[0..4].copy_from_slice(&0x27051956u32.to_be_bytes());
//! d[12..16].copy_from_slice(&4u32.to_be_bytes()); // size
//! d[28] = 5; // ih_os = linux
//! d[20..24].copy_from_slice(&0x80008000u32.to_be_bytes()); // entry
//! let mut h = d[..64].to_vec();
//! let c = crc::crc32(&h[..64]);
//! h[4..8].copy_from_slice(&c.to_be_bytes());
//! d[..64].copy_from_slice(&h);
//! let u = uimage::parse(&d).unwrap();
//! assert_eq!(u.size, 4);
//! ```

use crate::crc;
use std::vec::Vec;

/// Header length in bytes.
pub const HEADER_LEN: usize = 64;

/// The `ih_magic` value.
pub const MAGIC: u32 = 0x2705_1956;

/// A parsed U-Boot legacy header.
#[derive(Clone, Debug, PartialEq)]
pub struct UImage {
    /// `ih_time` build timestamp (unix).
    pub time: u32,
    /// `ih_size` payload byte size.
    pub size: u32,
    /// `ih_load` payload load address.
    pub load: u32,
    /// `ih_ep` entry point.
    pub entry: u32,
    /// `ih_dcrc` payload CRC32.
    pub data_crc: u32,
    /// `ih_os` operating-system id.
    pub os: u8,
    /// `ih_arch` CPU architecture id.
    pub arch: u8,
    /// `ih_type` image type id.
    pub image_type: u8,
    /// `ih_comp` compression id (0=none).
    pub comp: u8,
    /// `ih_name` (NUL-trimmed).
    pub name: Vec<u8>,
    /// Byte offset where the payload starts (always 64).
    pub data_offset: usize,
}

fn u32be(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at.checked_add(4)?)?;
    Some((s[0] as u32) << 24 | (s[1] as u32) << 16 | (s[2] as u32) << 8 | s[3] as u32)
}

/// Parses the header: magic, `hcrc` and `size` must all validate.
pub fn parse(d: &[u8]) -> Option<UImage> {
    if d.len() < HEADER_LEN {
        return None;
    }
    if u32be(d, 0)? != MAGIC {
        return None;
    }
    let hcrc = u32be(d, 4)?;
    let mut hdr = d[..HEADER_LEN].to_vec();
    hdr[4..8].copy_from_slice(&[0; 4]);
    if crc::crc32(&hdr) != hcrc {
        return None;
    }
    let size = u32be(d, 12)?;
    if HEADER_LEN.checked_add(size as usize)? > d.len() {
        return None;
    }
    let mut name: Vec<u8> = d.get(32..64)?.to_vec();
    while name.last() == Some(&0) {
        name.pop();
    }
    Some(UImage {
        time: u32be(d, 8)?,
        size,
        load: u32be(d, 16)?,
        entry: u32be(d, 20)?,
        data_crc: u32be(d, 24)?,
        os: *d.get(28)?,
        arch: *d.get(29)?,
        image_type: *d.get(30)?,
        comp: *d.get(31)?,
        name,
        data_offset: HEADER_LEN,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(size: u32) -> Vec<u8> {
        let mut d = vec![0u8; HEADER_LEN + size as usize];
        d[0..4].copy_from_slice(&MAGIC.to_be_bytes());
        d[12..16].copy_from_slice(&size.to_be_bytes());
        d[28] = 5; // IH_OS_LINUX
        d[29] = 2; // ARM
        d[30] = 2; // kernel
        d[32..39].copy_from_slice(b"uImage\0");
        let c = crc::crc32(&d[..HEADER_LEN]);
        d[4..8].copy_from_slice(&c.to_be_bytes());
        d
    }

    #[test]
    fn parses_header() {
        let u = parse(&fixture(8)).unwrap();
        assert_eq!(u.size, 8);
        assert_eq!(u.os, 5);
        assert_eq!(u.name, b"uImage".to_vec());
        assert_eq!(u.data_offset, 64);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 40]).is_none());
        let mut d = fixture(4);
        d[0] = 0x00; // bad magic
        assert!(parse(&d).is_none());
        let mut d = fixture(4);
        d[4] ^= 1; // hcrc mismatch
        assert!(parse(&d).is_none());
        let d = fixture(4);
        assert!(parse(&d[..HEADER_LEN]).is_none()); // payload missing
    }
}
