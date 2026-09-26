//! UPS — byuu's UPS patch format (companion to `crate::ips`). A
//! file opens with `UPS1`, then two varints (source size, target
//! size), then records of `{varint relative_offset, bytes…}` each
//! ending on a `0x00` byte, and closes with three u32LE CRCs
//! (source, target, patch). Varints use byuu's encoding: 7 bits per
//! byte, MSB *set* on the last byte, plus a shift bias.
//!
//! ```
//! use izanagi_kit::ups::{parse, records};
//! let mut d = b"UPS1".to_vec();
//! d.extend_from_slice(&[0x8A, 0x94]);  // src=10, dst=20 (varints)
//! d.push(0x80);                        // record at offset 0
//! d.extend_from_slice(&[0xAA, 0x00]);  // data then 0x00 terminator
//! d.extend_from_slice(&[0; 12]);       // 3 CRCs
//! let u = parse(&d).unwrap();
//! assert_eq!(u.source_size, 10);
//! let r = records(&u, &d).next().unwrap();
//! assert_eq!(r.offset, 0);
//! assert_eq!(r.data, &[0xAA]);
//! ```

/// File magic.
pub const MAGIC: &[u8; 4] = b"UPS1";
/// Trailing CRC block length.
pub const TRAILER: usize = 12;

/// A parsed header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ups {
    /// Declared input size.
    pub source_size: u64,
    /// Declared output size.
    pub target_size: u64,
    /// Byte offset of the first record.
    pub records_at: usize,
}

/// One XOR record: patch bytes starting at `offset`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// Absolute target offset (running sum of relative offsets).
    pub offset: u64,
    /// Bytes to XOR into the target.
    pub data: Vec<u8>,
}

/// byuu's varint: 7 data bits per byte, MSB **set** on the last byte,
/// plus a `shift` bias after each continuation so encodings are dense
/// (no two spellings for one value). `v` encodes as `v | 0x80` when
/// `v < 128`; `129` becomes `[0x01, 0x80]`.
pub fn varint(d: &[u8], at: usize) -> Option<(u64, usize)> {
    let mut data = 0u64;
    let mut shift = 1u64;
    let mut i = at;
    loop {
        let x = *d.get(i)?;
        i += 1;
        data = data.checked_add(u64::from(x & 0x7F).checked_mul(shift)?)?;
        if x & 0x80 != 0 {
            return Some((data, i));
        }
        shift = shift.checked_shl(7)?;
        data = data.checked_add(shift)?;
    }
}

/// Parse the header; `None` without `UPS1` or with a truncated trailer.
pub fn parse(d: &[u8]) -> Option<Ups> {
    if d.get(..4)? != MAGIC {
        return None;
    }
    let (source_size, at) = varint(d, 4)?;
    let (target_size, at) = varint(d, at)?;
    d.get(..at.checked_add(TRAILER)?)?; // room for the trailer
    Some(Ups {
        source_size,
        target_size,
        records_at: at,
    })
}

/// The three trailing CRCs (source, target, patch), little-endian.
pub fn crcs(d: &[u8]) -> Option<[u32; 3]> {
    let t = d.get(d.len().checked_sub(TRAILER)?..)?;
    let mut out = [0u32; 3];
    for (i, w) in out.iter_mut().enumerate() {
        *w = u32::from_le_bytes(t[i * 4..i * 4 + 4].try_into().ok()?);
    }
    Some(out)
}

/// Iterate records until the trailer; each is `{varint rel_offset,
/// bytes…, 0x00}`. Stops early on a malformed record.
pub fn records<'d>(u: &Ups, d: &'d [u8]) -> impl Iterator<Item = Record> + 'd {
    let mut at = u.records_at;
    let mut offset = 0u64;
    let end = d.len().saturating_sub(TRAILER);
    core::iter::from_fn(move || {
        if at >= end {
            return None;
        }
        let (rel, next) = varint(d, at)?;
        at = next;
        offset = offset.checked_add(rel)?;
        let mut data = Vec::new();
        loop {
            if at >= end {
                return None; // unterminated record
            }
            let c = d[at];
            at += 1;
            if c == 0 {
                break;
            }
            data.push(c);
        }
        Some(Record { offset, data })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = b"UPS1".to_vec();
        d.extend_from_slice(&[0xE4, 0x48, 0x80]); // 100, 200
                                                  // record: rel offset 5, two XOR bytes
        d.push(0x85);
        d.extend_from_slice(&[0x11, 0x22, 0x00]);
        // record: rel offset 129 → varint [0x01, 0x80]
        d.extend_from_slice(&[0x01, 0x80]);
        d.extend_from_slice(&[0x33, 0x00]);
        d.extend_from_slice(&[0x01, 0x02, 0x03, 0x04]); // src crc
        d.extend_from_slice(&[0x05, 0x06, 0x07, 0x08]); // dst crc
        d.extend_from_slice(&[0x09, 0x0A, 0x0B, 0x0C]); // patch crc
        d
    }

    #[test]
    fn parses_header() {
        let d = fixture();
        let u = parse(&d).unwrap();
        assert_eq!((u.source_size, u.target_size), (100, 200));
        assert_eq!(u.records_at, 7);
        assert_eq!(crcs(&d).unwrap(), [0x0403_0201, 0x0807_0605, 0x0C0B_0A09]);
    }

    #[test]
    fn record_walk() {
        let d = fixture();
        let u = parse(&d).unwrap();
        let rs: Vec<_> = records(&u, &d).collect();
        assert_eq!(rs.len(), 2);
        assert_eq!(rs[0].offset, 5);
        assert_eq!(rs[0].data, &[0x11, 0x22]);
        assert_eq!(rs[1].offset, 5 + 129); // relative
        assert_eq!(rs[1].data, &[0x33]);
    }

    #[test]
    fn varints_and_rejects() {
        assert_eq!(varint(&[0xFF], 0), Some((127, 1)));
        assert_eq!(varint(&[0x01, 0x80], 0), Some((129, 2)));
        assert_eq!(varint(&[0x48, 0x80], 0), Some((200, 2)));
        assert!(varint(&[], 0).is_none());
        assert!(parse(b"").is_none());
        assert!(parse(b"UPS2\0\0").is_none());
        // no trailer room
        assert!(parse(b"UPS1\x81\x82").is_none());
    }
}
