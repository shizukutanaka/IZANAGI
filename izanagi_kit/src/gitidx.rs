//! Git pack index (`.idx`) v2 parsing.
//!
//! Magic `\xFFtO\x63`, u32BE `version` (2), 256 u32BE fanout counts
//! (`fanout[255]` = total object count), then `count` × 20-byte
//! SHA-1 object names, `count` × u32 CRC32s, `count` × u32 offsets
//! (bit31 set → index into the large-offset table of u64s), and two
//! trailing pack/idx checksums.
//!
//! ```
//! use izanagi_kit::gitidx;
//! let mut d = b"\xFFtOc".to_vec();
//! d.extend_from_slice(&2u32.to_be_bytes()); // version
//! // fanout: entries 0..255; 2 objects
//! for i in 0..256 {
//!     d.extend_from_slice(&if i < 0x61 { 0u32 } else { 2u32 }.to_be_bytes());
//! }
//! d.extend_from_slice(&[0x11; 40]); // 2 sha1s
//! d.extend_from_slice(&[0u8; 8]); // 2 crc32s
//! d.extend_from_slice(&[0u8; 8]); // 2 offsets
//! d.extend_from_slice(&[0u8; 40]); // pack + idx checksums
//! let i = gitidx::parse(&d).unwrap();
//! assert_eq!(i.num_objects, 2);
//! ```

use std::vec::Vec;

/// v2 magic `\xFFtO\x63`.
pub const MAGIC: &[u8; 4] = b"\xFFtOc";

/// A parsed pack index.
#[derive(Clone, Debug, PartialEq)]
pub struct Idx {
    /// Index format version (2).
    pub version: u32,
    /// 256-entry fanout table (prefix sums).
    pub fanout: Vec<u32>,
    /// Total objects = `fanout[255]`.
    pub num_objects: u32,
    /// Offset of the SHA-1 name table.
    pub names_offset: usize,
    /// Offset of the CRC32 table.
    pub crcs_offset: usize,
    /// Offset of the offset table.
    pub offsets_offset: usize,
    /// Offset of the trailing checksums.
    pub trailer_offset: usize,
}

fn u32be(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at.checked_add(4)?)?;
    Some((s[0] as u32) << 24 | (s[1] as u32) << 16 | (s[2] as u32) << 8 | s[3] as u32)
}

/// Parses an idx v2 file: magic + version + monotone fanout whose
/// implied tables fit the input.
pub fn parse(d: &[u8]) -> Option<Idx> {
    if d.len() < 8 + 256 * 4 + 40 {
        return None;
    }
    if d.get(..4)? != MAGIC {
        return None;
    }
    let version = u32be(d, 4)?;
    if version != 2 {
        return None;
    }
    let mut fanout = Vec::with_capacity(256);
    let mut prev = 0u32;
    for i in 0..256 {
        let v = u32be(d, 8 + i * 4)?;
        if v < prev {
            return None; // fanout must be non-decreasing
        }
        prev = v;
        fanout.push(v);
    }
    let num_objects = fanout[255];
    let n = num_objects as usize;
    let names_offset: usize = 8 + 256 * 4;
    let crcs_offset = names_offset.checked_add(n.checked_mul(20)?)?;
    let offsets_offset = crcs_offset.checked_add(n.checked_mul(4)?)?;
    let trailer_offset = offsets_offset.checked_add(n.checked_mul(4)?)?;
    // trailer: pack checksum 20B + idx checksum 20B; large-offset
    // table sits between the offset table and trailer — require at
    // least the trailer to fit.
    if trailer_offset.checked_add(40)? > d.len() {
        return None;
    }
    Some(Idx {
        version,
        fanout,
        num_objects,
        names_offset,
        crcs_offset,
        offsets_offset,
        trailer_offset,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(n: u32) -> Vec<u8> {
        let mut d = MAGIC.to_vec();
        d.extend_from_slice(&2u32.to_be_bytes());
        for i in 0..256u32 {
            d.extend_from_slice(&if i < 0x61 { 0 } else { n }.to_be_bytes());
        }
        d.extend_from_slice(&std::vec![0x11u8; 20 * n as usize]);
        d.extend_from_slice(&std::vec![0u8; 4 * n as usize]);
        d.extend_from_slice(&std::vec![0u8; 4 * n as usize]);
        d.extend_from_slice(&[0u8; 40]);
        d
    }

    #[test]
    fn parses_index() {
        let i = parse(&fixture(3)).unwrap();
        assert_eq!(i.num_objects, 3);
        assert_eq!(i.names_offset, 8 + 1024);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 40]).is_none());
        let mut d = fixture(3);
        d[4..8].copy_from_slice(&1u32.to_be_bytes()); // version 1
        assert!(parse(&d).is_none());
        let mut d = fixture(3);
        d[8 + 4..8 + 8].copy_from_slice(&5u32.to_be_bytes()); // fanout[1] > fanout[0x60]
        d[8..12].copy_from_slice(&3u32.to_be_bytes()); // make fanout[0]=3 < 5 ok? breaks monotonicity later
        d.truncate(8 + 256 * 4 + 10); // truncated
        assert!(parse(&d).is_none());
    }
}
