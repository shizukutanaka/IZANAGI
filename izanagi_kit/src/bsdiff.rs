//! bsdiff — Colin Percival's `BSDIFF40` patch container. A file opens
//! with the 8-byte magic, then three u64LE lengths: the bzip2'ed
//! control block's compressed size, the bzip2'ed diff block's
//! compressed size, and the *uncompressed* new file's size — the
//! control/diff/extra sections then follow consecutively.
//!
//! ```
//! use izanagi_kit::bsdiff::{parse, ctrl_at, diff_at, extra_at};
//! let mut d = b"BSDIFF40".to_vec();
//! d.extend_from_slice(&4u64.to_le_bytes());  // ctrl compressed size
//! d.extend_from_slice(&7u64.to_le_bytes());  // diff compressed size
//! d.extend_from_slice(&64u64.to_le_bytes()); // new file size
//! d.extend_from_slice(&[0; 4 + 7 + 3]);      // sections + extra
//! let b = parse(&d).unwrap();
//! assert_eq!(ctrl_at(&b), 32);
//! assert_eq!(diff_at(&b), 36);
//! assert_eq!(extra_at(&b), 43);
//! ```

/// File magic.
pub const MAGIC: &[u8; 8] = b"BSDIFF40";
/// Fixed header length.
pub const HEADER: usize = 32;

/// A parsed header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bsdiff {
    /// Compressed size of the control section.
    pub ctrl_size: u64,
    /// Compressed size of the diff section.
    pub diff_size: u64,
    /// Uncompressed size of the target file.
    pub new_size: u64,
}

fn u64l(d: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(d.get(at..at + 8)?.try_into().ok()?))
}

/// Parse the 32-byte header; `None` without `BSDIFF40`.
pub fn parse(d: &[u8]) -> Option<Bsdiff> {
    if d.get(..8)? != MAGIC {
        return None;
    }
    Some(Bsdiff {
        ctrl_size: u64l(d, 8)?,
        diff_size: u64l(d, 16)?,
        new_size: u64l(d, 24)?,
    })
}

/// Offset of the compressed control section.
pub fn ctrl_at(_b: &Bsdiff) -> usize {
    HEADER
}

/// Offset of the compressed diff section.
pub fn diff_at(b: &Bsdiff) -> usize {
    HEADER.saturating_add(b.ctrl_size as usize)
}

/// Offset of the compressed extra-data section.
pub fn extra_at(b: &Bsdiff) -> usize {
    HEADER
        .saturating_add(b.ctrl_size as usize)
        .saturating_add(b.diff_size as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = MAGIC.to_vec();
        d.extend_from_slice(&4u64.to_le_bytes());
        d.extend_from_slice(&7u64.to_le_bytes());
        d.extend_from_slice(&64u64.to_le_bytes());
        d.extend_from_slice(&[0; 14]);
        d
    }

    #[test]
    fn parses_header() {
        let b = parse(&fixture()).unwrap();
        assert_eq!(b.ctrl_size, 4);
        assert_eq!(b.diff_size, 7);
        assert_eq!(b.new_size, 64);
        assert_eq!(ctrl_at(&b), 32);
        assert_eq!(diff_at(&b), 36);
        assert_eq!(extra_at(&b), 43);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"BSDIFF50").is_none());
        // magic ok but header truncated mid-u64
        assert!(parse(b"BSDIFF40\x01\x02").is_none());
    }
}
