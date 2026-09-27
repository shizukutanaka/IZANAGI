//! Windows `$Recycle.Bin` `$I` metadata file parsing.
//!
//! Version 1 (Vista/7): `version u64LE` = 1, `file_size u64`,
//! `deletion_time FILETIME u64`, then a fixed 520-byte UTF-16LE
//! path — total 544 bytes. Version 2 (8+): same 24-byte header,
//! then `name_len u32LE` and `name_len` UTF-16 code units.
//!
//! ```
//! use izanagi_kit::recbin;
//! let mut d = vec![0u8; 544];
//! d[0..8].copy_from_slice(&1u64.to_le_bytes());
//! d[8..16].copy_from_slice(&4096u64.to_le_bytes()); // file size
//! d[16..24].copy_from_slice(&0x11223344u64.to_le_bytes()); // del time
//! d[24..28].copy_from_slice(&[b'C', 0, b':', 0]);
//! let r = recbin::parse(&d).unwrap();
//! assert_eq!(r.version, 1);
//! assert_eq!(r.path, "C:");
//! ```

use std::string::String;

/// Version-1 fixed record size.
pub const V1_LEN: usize = 544;

/// A parsed `$I` record.
#[derive(Clone, Debug, PartialEq)]
pub struct RecBin {
    /// `version` (1 or 2).
    pub version: u64,
    /// Original `file_size`.
    pub file_size: u64,
    /// `deletion_time` FILETIME.
    pub deletion_time: u64,
    /// Original path (UTF-16, lossy, NUL-trimmed).
    pub path: String,
    /// Total record length consumed.
    pub len: usize,
}

fn u64le(d: &[u8], at: usize) -> Option<u64> {
    let s = d.get(at..at.checked_add(8)?)?;
    let mut v = 0u64;
    for (i, &b) in s.iter().enumerate() {
        v |= (b as u64) << (8 * i);
    }
    Some(v)
}

fn u32le(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at.checked_add(4)?)?;
    Some((s[0] as u32) | (s[1] as u32) << 8 | (s[2] as u32) << 16 | (s[3] as u32) << 24)
}

fn utf16(d: &[u8]) -> String {
    let mut s = String::new();
    for c in d.chunks(2) {
        let v = (c[0] as u16) | ((c.get(1).copied().unwrap_or(0) as u16) << 8);
        if v == 0 {
            break;
        }
        s.push(char::from_u32(v as u32).unwrap_or('\u{FFFD}'));
    }
    s
}

/// Parses a `$I` file: version 1 fixed 544B record or version 2
/// length-prefixed record.
pub fn parse(d: &[u8]) -> Option<RecBin> {
    let version = u64le(d, 0)?;
    let file_size = u64le(d, 8)?;
    let deletion_time = u64le(d, 16)?;
    match version {
        1 => {
            if d.len() < V1_LEN {
                return None;
            }
            let path = utf16(d.get(24..V1_LEN)?);
            Some(RecBin {
                version,
                file_size,
                deletion_time,
                path,
                len: V1_LEN,
            })
        }
        2 => {
            let name_len = u32le(d, 24)? as usize;
            let bytes = name_len.checked_mul(2)?;
            if 28usize.checked_add(bytes)? > d.len() || name_len == 0 {
                return None;
            }
            let path = utf16(d.get(28..28 + bytes)?);
            Some(RecBin {
                version,
                file_size,
                deletion_time,
                path,
                len: 28 + bytes,
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;

    #[test]
    fn parses_v1() {
        let mut d = vec![0u8; 544];
        d[0..8].copy_from_slice(&1u64.to_le_bytes());
        d[8..16].copy_from_slice(&100u64.to_le_bytes());
        d[16..24].copy_from_slice(&9u64.to_le_bytes());
        d[24..28].copy_from_slice(&[b'D', 0, b':', 0]);
        let r = parse(&d).unwrap();
        assert_eq!(r.file_size, 100);
        assert_eq!(r.path, "D:");
    }

    #[test]
    fn parses_v2() {
        let mut d = vec![0u8; 28 + 8];
        d[0..8].copy_from_slice(&2u64.to_le_bytes());
        d[24..28].copy_from_slice(&4u32.to_le_bytes()); // 4 chars
        d[28..36].copy_from_slice(&[b'a', 0, b'.', 0, b't', 0, b'x', 0]);
        let r = parse(&d).unwrap();
        assert_eq!(r.version, 2);
        assert_eq!(r.path, "a.tx");
        assert_eq!(r.len, 36);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 16]).is_none());
        let mut d = vec![0u8; 544];
        d[0..8].copy_from_slice(&7u64.to_le_bytes()); // version 7
        assert!(parse(&d).is_none());
    }
}
