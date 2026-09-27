//! Unity `UnityFS` asset-bundle header parsing.
//!
//! `UnityFS\0` signature, then **big-endian**: `version` u32,
//! `unity_version` C-string, `unity_revision` C-string,
//! `size` i64 (total bundle size), `compressed_block_size` u32,
//! `uncompressed_block_size` u32, `flags` u32 (bit6 = has directory
//! info; low bits select the codec).
//!
//! ```
//! use izanagi_kit::unityfs;
//! let mut d = b"UnityFS\0".to_vec();
//! d.extend_from_slice(&6u32.to_be_bytes()); // version
//! d.extend_from_slice(b"2021.3\0");
//! d.extend_from_slice(b"abc123\0");
//! d.extend_from_slice(&512i64.to_be_bytes()); // size
//! d.extend_from_slice(&64u32.to_be_bytes()); // comp block size
//! d.extend_from_slice(&64u32.to_be_bytes()); // uncomp block size
//! d.extend_from_slice(&0x43u32.to_be_bytes()); // flags
//! d.extend_from_slice(&[0u8; 64]); // block data
//! let u = unityfs::parse(&d).unwrap();
//! assert_eq!(u.version, 6);
//! ```

use std::string::String;

/// Signature `UnityFS\0`.
pub const MAGIC: &[u8; 8] = b"UnityFS\0";

/// A parsed UnityFS header.
#[derive(Clone, Debug, PartialEq)]
pub struct UnityFs {
    /// Format `version` (6 or 7 in practice).
    pub version: u32,
    /// Engine version string (`unity_version`).
    pub unity_version: String,
    /// Engine revision string (`unity_revision`).
    pub unity_revision: String,
    /// Total bundle `size` in bytes.
    pub size: u64,
    /// `compressed_block_size`.
    pub compressed_block_size: u32,
    /// `uncompressed_block_size`.
    pub uncompressed_block_size: u32,
    /// `flags` (codec in low bits, directory bit 0x40).
    pub flags: u32,
    /// Offset just past the header.
    pub data_offset: usize,
}

fn u32be(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at.checked_add(4)?)?;
    Some((s[0] as u32) << 24 | (s[1] as u32) << 16 | (s[2] as u32) << 8 | s[3] as u32)
}

fn u64be(d: &[u8], at: usize) -> Option<u64> {
    Some((u32be(d, at)? as u64) << 32 | u32be(d, at + 4)? as u64)
}

fn cstr(d: &[u8], at: &mut usize) -> Option<String> {
    let rest = d.get(*at..)?;
    let n = rest.iter().position(|&b| b == 0)?;
    let s = String::from_utf8_lossy(&rest[..n]).into_owned();
    *at += n + 1;
    Some(s)
}

/// Parses a UnityFS header; `size` may exceed the input for
/// streaming reads, so only the fixed fields are bounds-checked.
pub fn parse(d: &[u8]) -> Option<UnityFs> {
    if d.len() < 8 + 4 {
        return None;
    }
    if d.get(..8)? != MAGIC {
        return None;
    }
    let version = u32be(d, 8)?;
    if !(6..=7).contains(&version) {
        return None;
    }
    let mut at = 12;
    let unity_version = cstr(d, &mut at)?;
    let unity_revision = cstr(d, &mut at)?;
    let size = u64be(d, at)?;
    at += 8;
    let compressed_block_size = u32be(d, at)?;
    at += 4;
    let uncompressed_block_size = u32be(d, at)?;
    at += 4;
    let flags = u32be(d, at)?;
    at += 4;
    Some(UnityFs {
        version,
        unity_version,
        unity_revision,
        size,
        compressed_block_size,
        uncompressed_block_size,
        flags,
        data_offset: at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn fixture() -> Vec<u8> {
        let mut d = MAGIC.to_vec();
        d.extend_from_slice(&7u32.to_be_bytes());
        d.extend_from_slice(b"2022.3\0");
        d.extend_from_slice(b"rev9\0");
        d.extend_from_slice(&1024u64.to_be_bytes());
        d.extend_from_slice(&128u32.to_be_bytes());
        d.extend_from_slice(&128u32.to_be_bytes());
        d.extend_from_slice(&0u32.to_be_bytes());
        d.extend_from_slice(&[0u8; 32]);
        d
    }

    #[test]
    fn parses_header() {
        let u = parse(&fixture()).unwrap();
        assert_eq!(u.version, 7);
        assert_eq!(u.unity_version, "2022.3");
        assert_eq!(u.size, 1024);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 10]).is_none());
        let mut d = fixture();
        d[0] = b'X';
        assert!(parse(&d).is_none());
        let mut d = fixture();
        d[8..12].copy_from_slice(&3u32.to_be_bytes()); // version 3
        assert!(parse(&d).is_none());
    }
}
