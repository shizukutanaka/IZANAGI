//! Mercurial revlog index (`.i`) parsing — revlogNG 64-byte entries.
//!
//! The first u32BE doubles as the version field: `(flags << 16) |
//! version` (version 1 or 2). Each 64-byte entry holds
//! `offset_flags u64`, `compressed_len u32`, `uncompressed_len u32`,
//! `base_rev i32`, `link_rev i32`, `p1 i32`, `p2 i32`, 20-byte
//! `node_id`, 12-byte padding.
//!
//! ```
//! use izanagi_kit::revlog;
//! // version 1 header in entry0's first u32
//! let mut d = 0x0001u32.to_be_bytes().to_vec();
//! d.extend_from_slice(&0u32.to_be_bytes()); // rest of offset_flags
//! d.extend_from_slice(&4u32.to_be_bytes()); // compressed_len
//! d.extend_from_slice(&10u32.to_be_bytes()); // uncompressed_len
//! d.extend_from_slice(&0i32.to_be_bytes()); // base_rev
//! d.extend_from_slice(&7i32.to_be_bytes()); // link_rev
//! d.extend_from_slice(&(-1i32).to_be_bytes()); // p1
//! d.extend_from_slice(&(-1i32).to_be_bytes()); // p2
//! d.extend_from_slice(&[0xAA; 20]); // node id
//! d.extend_from_slice(&[0u8; 12]); // padding
//! let r = revlog::parse(&d).unwrap();
//! assert_eq!(r.entries.len(), 1);
//! ```

use std::vec::Vec;

/// RevlogNG entry size.
pub const ENTRY_LEN: usize = 64;

/// One revlog index entry.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    /// `compressed_len`.
    pub compressed_len: u32,
    /// `uncompressed_len`.
    pub uncompressed_len: u32,
    /// `base_rev` — revision this delta is based on.
    pub base_rev: i32,
    /// `link_rev` — changelog revision.
    pub link_rev: i32,
    /// Parent revisions (`-1` = null).
    pub p1: i32,
    /// Second parent.
    pub p2: i32,
    /// 20-byte node id (SHA-1).
    pub node_id: [u8; 20],
}

/// A parsed revlog index.
#[derive(Clone, Debug, PartialEq)]
pub struct Revlog {
    /// Index format `version` (1 or 2).
    pub version: u32,
    /// Flags from the version word.
    pub flags: u32,
    /// Entries.
    pub entries: Vec<Entry>,
}

fn u32be(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at.checked_add(4)?)?;
    Some((s[0] as u32) << 24 | (s[1] as u32) << 16 | (s[2] as u32) << 8 | s[3] as u32)
}

fn i32be(d: &[u8], at: usize) -> Option<i32> {
    Some(u32be(d, at)? as i32)
}

/// Parses a revlogNG index: whole input must tile into 64B entries;
/// entry 0's offset field carries `flags << 16 | version`.
pub fn parse(d: &[u8]) -> Option<Revlog> {
    if d.is_empty() || d.len() % ENTRY_LEN != 0 {
        return None;
    }
    let word = u32be(d, 0)?;
    let version = word & 0xFFFF;
    let flags = word >> 16;
    if !(1..=2).contains(&version) {
        return None;
    }
    let mut entries = Vec::with_capacity(d.len() / ENTRY_LEN);
    for at in (0..d.len()).step_by(ENTRY_LEN) {
        let compressed_len = u32be(d, at + 8)?;
        let uncompressed_len = u32be(d, at + 12)?;
        let base_rev = i32be(d, at + 16)?;
        let link_rev = i32be(d, at + 20)?;
        let p1 = i32be(d, at + 24)?;
        let p2 = i32be(d, at + 28)?;
        let mut node_id = [0u8; 20];
        node_id.copy_from_slice(d.get(at + 32..at + 52)?);
        entries.push(Entry {
            compressed_len,
            uncompressed_len,
            base_rev,
            link_rev,
            p1,
            p2,
            node_id,
        });
    }
    Some(Revlog {
        version,
        flags,
        entries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(link: i32) -> Vec<u8> {
        let mut e = Vec::new();
        e.extend_from_slice(&0u64.to_be_bytes());
        e.extend_from_slice(&8u32.to_be_bytes());
        e.extend_from_slice(&32u32.to_be_bytes());
        e.extend_from_slice(&0i32.to_be_bytes());
        e.extend_from_slice(&link.to_be_bytes());
        e.extend_from_slice(&(-1i32).to_be_bytes());
        e.extend_from_slice(&(-1i32).to_be_bytes());
        e.extend_from_slice(&[0x55; 20]);
        e.extend_from_slice(&[0u8; 12]);
        e
    }

    #[test]
    fn parses_entries() {
        let mut d = entry(1);
        // set version 1 in entry0 offset_flags word
        d[0..4].copy_from_slice(&1u32.to_be_bytes());
        d.extend_from_slice(&entry(2));
        let r = parse(&d).unwrap();
        assert_eq!(r.version, 1);
        assert_eq!(r.entries.len(), 2);
        assert_eq!(r.entries[1].link_rev, 2);
        assert_eq!(r.entries[0].node_id, [0x55; 20]);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 30]).is_none()); // not multiple of 64
        let mut d = entry(1);
        d[0..4].copy_from_slice(&9u32.to_be_bytes()); // version 9
        assert!(parse(&d).is_none());
    }
}
