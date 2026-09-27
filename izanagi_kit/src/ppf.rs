//! PPF — PlayStation Patch Format. A file opens with `PPF` plus a
//! version digit (`PPF10`/`PPF20`/`PPF30`), an encoding byte for v3
//! (`0` = bin offsets, `1` = Gi/disk-image offsets), a description,
//! then records `{offset, u8 len, data}` — u32LE offsets for bin
//! encoding, u64LE for image files — optionally followed by an
//! undo block when the v3 "undo" variant is in play.
//!
//! ```
//! use izanagi_kit::ppf::{parse, records, Encoding};
//! let mut d = b"PPF30".to_vec();
//! d.push(0);                            // bin encoding
//! d.extend_from_slice(&[0; 60]);        // description
//! d.extend_from_slice(&[4, 0, 0, 0]);   // u32LE offset 4
//! d.extend_from_slice(&[2, 0xAA, 0xBB]);// len 2 + data
//! let p = parse(&d).unwrap();
//! assert_eq!(p.version, 3);
//! assert_eq!(p.encoding, Encoding::Bin);
//! assert_eq!(records(&p, &d).next().unwrap().offset, 4);
//! ```

/// v3 description field length.
pub const DESC_LEN_V3: usize = 60;
/// v1/v2 description field length.
pub const DESC_LEN_V1: usize = 50;

/// Offset encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    /// 32-bit little-endian offsets (raw binary image).
    Bin,
    /// 64-bit little-endian offsets (image files >4 GiB — "GI").
    Gi,
}

/// A parsed header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ppf {
    /// Version digit (1..=3).
    pub version: u8,
    /// Offset encoding (v3 only meaningful; v1/v2 report `Bin`).
    pub encoding: Encoding,
    /// Description, NUL-trimmed.
    pub description: Vec<u8>,
    /// Byte offset of the first record.
    pub records_at: usize,
}

/// One patch record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// Target offset.
    pub offset: u64,
    /// Bytes to write.
    pub data: Vec<u8>,
}

fn u32l(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

fn u64l(d: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(d.get(at..at + 8)?.try_into().ok()?))
}

/// Parse the header; `None` without `PPF` + version digit.
pub fn parse(d: &[u8]) -> Option<Ppf> {
    if d.get(..3)? != b"PPF" {
        return None;
    }
    let version = d.get(3)?.checked_sub(b'0')?;
    if !(1..=3).contains(&version) || d.get(4)? != &b'0' {
        return None;
    }
    let (encoding, desc_len, desc_at) = if version == 3 {
        let enc = match *d.get(5)? {
            0 => Encoding::Bin,
            1 => Encoding::Gi,
            _ => return None,
        };
        (enc, DESC_LEN_V3, 6)
    } else {
        (Encoding::Bin, DESC_LEN_V1, 5)
    };
    let raw = d.get(desc_at..desc_at + desc_len)?;
    let end = raw.iter().position(|&c| c == 0).unwrap_or(raw.len());
    Some(Ppf {
        version,
        encoding,
        description: raw[..end].to_vec(),
        records_at: desc_at + desc_len,
    })
}

/// Iterate records until EOF or a truncated record.
pub fn records<'d>(p: &Ppf, d: &'d [u8]) -> impl Iterator<Item = Record> + 'd {
    let mut at = p.records_at;
    let wide = p.encoding == Encoding::Gi;
    core::iter::from_fn(move || {
        let offset = if wide {
            u64l(d, at)?
        } else {
            u64::from(u32l(d, at)?)
        };
        at += if wide { 8 } else { 4 };
        let len = usize::from(*d.get(at)?);
        at += 1;
        let data = d.get(at..at.checked_add(len)?)?.to_vec();
        at += len;
        Some(Record { offset, data })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = b"PPF30".to_vec();
        d.push(0);
        let mut desc = b"level select".to_vec();
        desc.resize(DESC_LEN_V3, 0);
        d.extend_from_slice(&desc);
        d.extend_from_slice(&4u32.to_le_bytes());
        d.extend_from_slice(&[2, 0xAA, 0xBB]);
        d
    }

    #[test]
    fn parses_header() {
        let d = fixture();
        let p = parse(&d).unwrap();
        assert_eq!(p.version, 3);
        assert_eq!(p.encoding, Encoding::Bin);
        assert_eq!(p.description, b"level select");
        assert_eq!(p.records_at, 66);
    }

    #[test]
    fn record_walk() {
        let d = fixture();
        let p = parse(&d).unwrap();
        let rs: Vec<_> = records(&p, &d).collect();
        assert_eq!(rs.len(), 1);
        assert_eq!(rs[0].offset, 4);
        assert_eq!(rs[0].data, &[0xAA, 0xBB]);
    }

    #[test]
    fn gi_offsets() {
        let mut d = b"PPF30".to_vec();
        d.push(1);
        d.extend_from_slice(&[0; DESC_LEN_V3]);
        d.extend_from_slice(&0x1_0000_0005u64.to_le_bytes());
        d.extend_from_slice(&[1, 0xCC]);
        let p = parse(&d).unwrap();
        assert_eq!(p.encoding, Encoding::Gi);
        assert_eq!(records(&p, &d).next().unwrap().offset, 0x1_0000_0005);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"PPF40").is_none());
        assert!(parse(b"PPF00").is_none());
        assert!(parse(b"PPF30\x02").is_none()); // bad encoding
    }
}
