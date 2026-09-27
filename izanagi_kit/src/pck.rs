//! Godot Engine `.pck` pack-file header parsing.
//!
//! `GDPC` magic (u32LE `0x43504447`), then u32LE `pack_version`,
//! `ver_major`/`ver_minor`/`ver_rev`, `pack_flags`,
//! `file_base_ofs` u64LE, 16 × u64 reserved, `file_count` u32.
//!
//! ```
//! use izanagi_kit::pck;
//! let mut d = 0x43504447u32.to_le_bytes().to_vec();
//! d.extend_from_slice(&1u32.to_le_bytes()); // pack_version
//! d.extend_from_slice(&4u32.to_le_bytes()); // major
//! d.extend_from_slice(&0u32.to_le_bytes()); // minor
//! d.extend_from_slice(&0u32.to_le_bytes()); // rev
//! d.extend_from_slice(&0u32.to_le_bytes()); // flags
//! d.extend_from_slice(&0u64.to_le_bytes()); // file_base_ofs
//! d.extend_from_slice(&[0u8; 16 * 8]); // reserved
//! d.extend_from_slice(&3u32.to_le_bytes()); // file_count
//! let p = pck::parse(&d).unwrap();
//! assert_eq!(p.file_count, 3);
//! ```

/// `GDPC` magic as u32LE.
pub const MAGIC: u32 = 0x4350_4447;

/// Header length through `file_count` (4 + 5×4 + 8 + 128 + 4).
pub const HEADER_LEN: usize = 4 + 20 + 8 + 128 + 4;

/// A parsed `.pck` header.
#[derive(Clone, Debug, PartialEq)]
pub struct Pck {
    /// Pack format `version` (1 for Godot 3.x packs).
    pub version: u32,
    /// Engine version: major.
    pub ver_major: u32,
    /// Engine version: minor.
    pub ver_minor: u32,
    /// Engine version: revision.
    pub ver_rev: u32,
    /// `pack_flags` (encryption/dir bits).
    pub flags: u32,
    /// `file_base_ofs` — offset of the file-offset table.
    pub file_base_ofs: u64,
    /// `file_count` — entries in the file table.
    pub file_count: u32,
    /// Offset of the first file-entry record.
    pub table_offset: usize,
}

fn u32le(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at.checked_add(4)?)?;
    Some(s[0] as u32 | (s[1] as u32) << 8 | (s[2] as u32) << 16 | (s[3] as u32) << 24)
}

fn u64le(d: &[u8], at: usize) -> Option<u64> {
    Some(u32le(d, at)? as u64 | (u32le(d, at + 4)? as u64) << 32)
}

/// Parses a `.pck` header; requires magic and ≥ HEADER_LEN bytes.
pub fn parse(d: &[u8]) -> Option<Pck> {
    if d.len() < HEADER_LEN {
        return None;
    }
    if u32le(d, 0)? != MAGIC {
        return None;
    }
    Some(Pck {
        version: u32le(d, 4)?,
        ver_major: u32le(d, 8)?,
        ver_minor: u32le(d, 12)?,
        ver_rev: u32le(d, 16)?,
        flags: u32le(d, 20)?,
        file_base_ofs: u64le(d, 24)?,
        file_count: u32le(d, 160)?,
        table_offset: HEADER_LEN,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn fixture() -> Vec<u8> {
        let mut d = MAGIC.to_le_bytes().to_vec();
        d.extend_from_slice(&1u32.to_le_bytes());
        d.extend_from_slice(&4u32.to_le_bytes());
        d.extend_from_slice(&2u32.to_le_bytes());
        d.extend_from_slice(&0u32.to_le_bytes());
        d.extend_from_slice(&0u32.to_le_bytes());
        d.extend_from_slice(&0u64.to_le_bytes());
        d.extend_from_slice(&[0u8; 128]);
        d.extend_from_slice(&7u32.to_le_bytes());
        d
    }

    #[test]
    fn parses_header() {
        let p = parse(&fixture()).unwrap();
        assert_eq!(p.version, 1);
        assert_eq!(p.ver_major, 4);
        assert_eq!(p.file_count, 7);
        assert_eq!(p.table_offset, HEADER_LEN);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 40]).is_none());
        let mut d = fixture();
        d[3] = 0xFF;
        assert!(parse(&d).is_none());
    }
}
