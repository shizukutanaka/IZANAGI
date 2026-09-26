//! Apple UDIF/DMG trailer (`koly` block).
//!
//! A compressed DMG ends in a 512-byte trailer starting with the
//! `"koly"` signature — all big-endian. It carries the format version
//! and header size, data/resource fork offsets and lengths, the
//! segment descriptor (number/count/UUID), a data checksum header,
//! the image variant and the virtual sector count.
//!
//! ```
//! use izanagi_kit::dmg::{parse, Kind, TRAILER};
//!
//! let mut d = vec![0u8; TRAILER];
//! d[..4].copy_from_slice(b"koly");
//! let put = |d: &mut [u8], at: usize, v: u64, n: usize| {
//!     for i in 0..n { d[at + i] = (v >> ((n - 1 - i) * 8)) as u8; }
//! };
//! put(&mut d, 4, 4, 4);           // version 4
//! put(&mut d, 8, 512, 4);         // header size
//! put(&mut d, 24, 2048, 8);       // data fork offset
//! put(&mut d, 32, 0x40_0000, 8);  // data fork length
//! put(&mut d, 216, 1, 4);         // variant
//! put(&mut d, 224, 20480, 8);     // sectors (10 MiB)
//! let t = parse(&d).unwrap();
//! assert_eq!(t.version, 4);
//! assert_eq!(t.kind, Kind::ReadWrite);
//! assert_eq!(t.sector_count, 20480);
//! ```

/// Trailer size in bytes.
pub const TRAILER: usize = 512;
/// Magic signature (`u32` big-endian).
pub const MAGIC: u32 = 0x6B6F_6C79; // "koly"

fn be32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(at)?) << 24
            | u32::from(*d.get(at + 1)?) << 16
            | u32::from(*d.get(at + 2)?) << 8
            | u32::from(*d.get(at + 3)?),
    )
}

fn be64(d: &[u8], at: usize) -> Option<u64> {
    Some(u64::from(be32(d, at)?) << 32 | u64::from(be32(d, at + 4)?))
}

/// Image variant (`udzo` compression flavor recorded in the trailer).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    /// `1` — read/write (`UDRW`-derived images).
    ReadWrite,
    /// `2` — `UDCO` ADC-compressed.
    Adc,
    /// `4` — `UDZO` zlib-compressed.
    Zlib,
    /// `5` — `UDZO` lzfse-compressed (later macOS).
    Lzfse,
    /// `6` — `ULMO` LZMA (rarely produced).
    Lzma,
    /// `8` — `UDBZ` bzip2-compressed.
    Bzip2,
    /// Any other variant id.
    Other(u32),
}

impl Kind {
    fn of(v: u32) -> Self {
        match v {
            1 => Kind::ReadWrite,
            2 => Kind::Adc,
            4 => Kind::Zlib,
            5 => Kind::Lzfse,
            6 => Kind::Lzma,
            8 => Kind::Bzip2,
            v => Kind::Other(v),
        }
    }
}

/// A parsed DMG trailer.
#[derive(Clone, Debug, PartialEq)]
pub struct Dmg {
    /// Format version (usually `4`).
    pub version: u32,
    /// Trailer header size (usually `512`).
    pub header_size: u32,
    /// Flags.
    pub flags: u32,
    /// Byte offset of the data fork (compressed blocks).
    pub data_fork_at: u64,
    /// Length of the data fork in bytes.
    pub data_fork_len: u64,
    /// Segment number.
    pub segment: u32,
    /// Total segment count.
    pub segment_count: u32,
    /// Segment UUID (16 raw bytes).
    pub segment_id: [u8; 16],
    /// Checksum algorithm (`1` none, `2` CRC32, `3` SHA-1...).
    pub checksum_type: u32,
    /// Checksum size in bytes.
    pub checksum_size: u32,
    /// Image variant.
    pub kind: Kind,
    /// Total sector count of the virtual device.
    pub sector_count: u64,
}

impl Dmg {
    /// Virtual image size in bytes (`sector_count * 512`).
    pub fn bytes(&self) -> u64 {
        self.sector_count.saturating_mul(512)
    }
}

/// Parse a DMG `koly` trailer. Returns `None` on a bad magic, a
/// non-512 `header_size`, or a buffer shorter than 232 bytes (the
/// last field read ends at offset 232).
pub fn parse(d: &[u8]) -> Option<Dmg> {
    if be32(d, 0)? != MAGIC {
        return None;
    }
    let header_size = be32(d, 8)?;
    if header_size != TRAILER as u32 {
        return None;
    }
    let mut segment_id = [0u8; 16];
    segment_id.copy_from_slice(d.get(64..80)?);
    Some(Dmg {
        version: be32(d, 4)?,
        header_size,
        flags: be32(d, 12)?,
        data_fork_at: be64(d, 24)?,
        data_fork_len: be64(d, 32)?,
        segment: be32(d, 56)?,
        segment_count: be32(d, 60)?,
        segment_id,
        checksum_type: be32(d, 80)?,
        checksum_size: be32(d, 84)?,
        kind: Kind::of(be32(d, 216)?),
        sector_count: be64(d, 224)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn put(d: &mut [u8], at: usize, v: u64, n: usize) {
        for i in 0..n {
            d[at + i] = (v >> ((n - 1 - i) * 8)) as u8;
        }
    }

    fn trailer() -> Vec<u8> {
        let mut d = vec![0u8; TRAILER];
        d[..4].copy_from_slice(b"koly");
        put(&mut d, 4, 4, 4);
        put(&mut d, 8, 512, 4);
        put(&mut d, 12, 1, 4);
        put(&mut d, 16, 0, 8);
        put(&mut d, 24, 2048, 8);
        put(&mut d, 32, 0x40_0000, 8);
        put(&mut d, 56, 1, 4);
        put(&mut d, 60, 1, 4);
        for i in 0..16 {
            d[64 + i] = 0xA0 + i as u8;
        }
        put(&mut d, 80, 2, 4);
        put(&mut d, 84, 4, 4);
        put(&mut d, 216, 1, 4);
        put(&mut d, 224, 20480, 8);
        d
    }

    #[test]
    fn parse_reads_every_field() {
        let d = trailer();
        let t = parse(&d).unwrap();
        assert_eq!(t.version, 4);
        assert_eq!(t.header_size, 512);
        assert_eq!(t.flags, 1);
        assert_eq!(t.data_fork_at, 2048);
        assert_eq!(t.data_fork_len, 0x40_0000);
        assert_eq!((t.segment, t.segment_count), (1, 1));
        assert_eq!(t.segment_id[0], 0xA0);
        assert_eq!(t.checksum_type, 2);
        assert_eq!(t.checksum_size, 4);
        assert_eq!(t.kind, Kind::ReadWrite);
        assert_eq!(t.sector_count, 20480);
        assert_eq!(t.bytes(), 20480 * 512);
    }

    #[test]
    fn kind_ids() {
        assert_eq!(Kind::of(2), Kind::Adc);
        assert_eq!(Kind::of(4), Kind::Zlib);
        assert_eq!(Kind::of(5), Kind::Lzfse);
        assert_eq!(Kind::of(6), Kind::Lzma);
        assert_eq!(Kind::of(8), Kind::Bzip2);
        assert_eq!(Kind::of(0), Kind::Other(0));
    }

    #[test]
    fn rejects() {
        assert_eq!(parse(&[]), None);
        assert_eq!(parse(&[0u8; 300]), None);
        let mut d = trailer();
        d[0] = b'x';
        assert_eq!(parse(&d), None);
        let mut d2 = trailer();
        d2[11] = 1; // header_size 513
        assert_eq!(parse(&d2), None);
    }

    #[test]
    fn constants() {
        assert_eq!(TRAILER, 512);
        assert_eq!(MAGIC, 0x6B6F_6C79);
    }
}
