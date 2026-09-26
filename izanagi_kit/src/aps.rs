//! APS — the N64 APS patch format (often seen on romhacking sites;
//! companion to `crate::ips`/`crate::ups`/`crate::bps`). A file opens
//! with `APS10`, a 50-byte description (NUL-padded), a one-byte patch
//! type, then records of `{u32BE offset, u8 len, len data}` — a `0`
//! length selects the RLE form `{u8 rle_len, u8 value}`.
//!
//! ```
//! use izanagi_kit::aps::{parse, records, Patch};
//! let mut d = b"APS10".to_vec();
//! d.extend_from_slice(&[b'A'; 50]);
//! d.push(0);
//! d.extend_from_slice(&[0, 0, 0, 4, 2, 0xAA, 0xBB]); // len=2 record
//! d.extend_from_slice(&[0, 0, 0, 9, 0, 5, 0x77]);     // RLE: 5×0x77
//! let a = parse(&d).unwrap();
//! assert_eq!(a.description, vec![b'A'; 50]);
//! let rs: Vec<_> = records(&d).collect();
//! assert_eq!(rs[0], Patch::Bytes { offset: 4, data: vec![0xAA, 0xBB] });
//! assert_eq!(rs[1], Patch::Rle { offset: 9, len: 5, value: 0x77 });
//! ```

/// File magic.
pub const MAGIC: &[u8; 5] = b"APS10";
/// Fixed description field length.
pub const DESC_LEN: usize = 50;
/// Byte offset of the first record.
pub const RECORDS_AT: usize = 5 + DESC_LEN + 1;

/// A parsed header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Aps {
    /// The 50-byte description, NUL-trimmed.
    pub description: Vec<u8>,
    /// Patch type byte (0/1 = simple, 2 = image-per-block in some tools).
    pub patch_type: u8,
}

/// One record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Patch {
    /// Literal `data` written at `offset`.
    Bytes {
        /// Target offset.
        offset: u32,
        /// Bytes to write.
        data: Vec<u8>,
    },
    /// `value` repeated `len` times at `offset`.
    Rle {
        /// Target offset.
        offset: u32,
        /// Repeat count.
        len: u8,
        /// Byte value.
        value: u8,
    },
}

fn u32b(d: &[u8], at: usize) -> Option<u32> {
    let r = d.get(at..at + 4)?;
    Some(
        (u32::from(r[0]) << 24)
            | (u32::from(r[1]) << 16)
            | (u32::from(r[2]) << 8)
            | u32::from(r[3]),
    )
}

/// Parse the header; `None` without `APS10`.
pub fn parse(d: &[u8]) -> Option<Aps> {
    if d.get(..5)? != MAGIC {
        return None;
    }
    let raw = d.get(5..5 + DESC_LEN)?;
    let end = raw.iter().position(|&c| c == 0).unwrap_or(raw.len());
    let patch_type = *d.get(5 + DESC_LEN)?;
    Some(Aps {
        description: raw[..end].to_vec(),
        patch_type,
    })
}

/// Iterate records until EOF or a truncated record.
pub fn records(d: &[u8]) -> impl Iterator<Item = Patch> + '_ {
    let mut at = RECORDS_AT;
    core::iter::from_fn(move || {
        let offset = u32b(d, at)?;
        let len = *d.get(at + 4)?;
        at += 5;
        if len == 0 {
            let rle_len = *d.get(at)?;
            let value = *d.get(at + 1)?;
            at += 2;
            Some(Patch::Rle {
                offset,
                len: rle_len,
                value,
            })
        } else {
            let data = d.get(at..at + usize::from(len))?.to_vec();
            at += usize::from(len);
            Some(Patch::Bytes { offset, data })
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = b"APS10".to_vec();
        let mut desc = b"fix for level 3".to_vec();
        desc.resize(DESC_LEN, 0);
        d.extend_from_slice(&desc);
        d.push(0);
        d.extend_from_slice(&[0, 0, 0, 4, 2, 0xAA, 0xBB]);
        d.extend_from_slice(&[0, 0, 0, 9, 0, 5, 0x77]);
        d
    }

    #[test]
    fn parses_header() {
        let a = parse(&fixture()).unwrap();
        assert_eq!(a.description, b"fix for level 3");
        assert_eq!(a.patch_type, 0);
        assert_eq!(RECORDS_AT, 56);
    }

    #[test]
    fn record_walk() {
        let rs: Vec<_> = records(&fixture()).collect();
        assert_eq!(rs.len(), 2);
        assert_eq!(
            rs[0],
            Patch::Bytes {
                offset: 4,
                data: vec![0xAA, 0xBB]
            }
        );
        assert_eq!(
            rs[1],
            Patch::Rle {
                offset: 9,
                len: 5,
                value: 0x77
            }
        );
    }

    #[test]
    fn rejects_and_truncation() {
        assert!(parse(b"").is_none());
        assert!(parse(b"APS11").is_none());
        // truncated record payload stops iteration
        let mut d = fixture();
        d.pop();
        let rs: Vec<_> = records(&d).collect();
        assert_eq!(rs.len(), 1);
    }
}
