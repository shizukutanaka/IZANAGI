//! Gatan DigitalMicrograph 3/4 (`.dm3`/`.dm4`) tag-tree header:
//! `version` u32 BE (3 or 4), file `length` (u32 for v3, u64 for
//! v4), `little_endian` flag u32, then the root tag group:
//! `sorted` u8, `closed` u8, `ntags` i32 — big-endian even when the
//! payload is little-endian.
//!
//! ```
//! let mut d = Vec::new();
//! d.extend_from_slice(&[0, 0, 0, 3]);          // version 3
//! d.extend_from_slice(&[0, 0, 0, 30]);         // length (total - 16 or total)
//! d.extend_from_slice(&[0, 0, 0, 1]);          // little_endian
//! d.extend_from_slice(&[0, 0]);                // sorted/closed
//! d.extend_from_slice(&[0, 0, 0, 2]);          // ntags
//! d.extend_from_slice(&[0u8; 12]);
//! let f = izanagi_kit::dm3::parse(&d).unwrap();
//! assert_eq!(f.version, 3);
//! assert_eq!(f.root_tags, 2);
//! ```

/// A parsed DM3/DM4 file header.
#[derive(Clone, Debug)]
pub struct Dm {
    /// File-format version: 3 or 4.
    pub version: u32,
    /// Declared file length.
    pub file_len: u64,
    /// `true` when tag payloads are little-endian.
    pub little_endian: bool,
    /// Root-group tag count.
    pub root_tags: u32,
    /// Whether the file appears self-consistent (declared length
    /// equals the buffer length or the buffer minus the header).
    pub len_consistent: bool,
}

fn be32(d: &[u8], o: usize) -> Option<u32> {
    Some(
        (u32::from(*d.get(o)?) << 24)
            | (u32::from(*d.get(o + 1)?) << 16)
            | (u32::from(*d.get(o + 2)?) << 8)
            | u32::from(*d.get(o + 3)?),
    )
}
fn be64(d: &[u8], o: usize) -> Option<u64> {
    let mut v = 0u64;
    for i in 0..8 {
        v = v << 8 | u64::from(*d.get(o + i)?);
    }
    Some(v)
}

/// Parse a DM3/DM4 header; `None` for other versions or truncated
/// input.
pub fn parse(d: &[u8]) -> Option<Dm> {
    let version = be32(d, 0)?;
    let (file_len, off) = match version {
        3 => (u64::from(be32(d, 4)?), 8usize),
        4 => (be64(d, 4)?, 12usize),
        _ => return None,
    };
    let little_endian = match be32(d, off)? {
        0 => false,
        1 => true,
        _ => return None,
    };
    let p = off + 4;
    let sorted = *d.get(p)?;
    let closed = *d.get(p + 1)?;
    if sorted > 1 || closed > 1 {
        return None;
    }
    let root_tags = be32(d, p + 2)?;
    if root_tags > 1_000_000 {
        return None;
    }
    let n = d.len() as u64;
    let len_consistent = file_len == n || file_len + 16 == n || file_len == n + 16;
    Some(Dm {
        version,
        file_len,
        little_endian,
        root_tags,
        len_consistent,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk3(tags: u32, len: u32) -> Vec<u8> {
        let mut d = Vec::new();
        d.extend_from_slice(&[0, 0, 0, 3]);
        d.extend_from_slice(&[
            (len >> 24) as u8,
            (len >> 16) as u8,
            (len >> 8) as u8,
            len as u8,
        ]);
        d.extend_from_slice(&[0, 0, 0, 1]);
        d.extend_from_slice(&[0, 0]);
        d.extend_from_slice(&[
            (tags >> 24) as u8,
            (tags >> 16) as u8,
            (tags >> 8) as u8,
            tags as u8,
        ]);
        d
    }

    #[test]
    fn v3() {
        let d = mk3(7, 18);
        let f = parse(&d).unwrap();
        assert_eq!(f.version, 3);
        assert_eq!(f.file_len, 18);
        assert!(f.little_endian);
        assert_eq!(f.root_tags, 7);
        assert!(f.len_consistent); // 18 == d.len()
    }

    #[test]
    fn v4() {
        let mut d = Vec::new();
        d.extend_from_slice(&[0, 0, 0, 4]);
        d.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 22]); // u64 len == d.len()
        d.extend_from_slice(&[0, 0, 0, 1]);
        d.extend_from_slice(&[0, 0, 0, 0, 0, 1]);
        let f = parse(&d).unwrap();
        assert_eq!(f.version, 4);
        assert_eq!(f.root_tags, 1);
        assert!(f.len_consistent);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        assert!(parse(&[0, 0, 0, 5, 0, 0, 0, 0]).is_none()); // version 5
        let mut d = mk3(1, 18);
        d[11] = 7; // little_endian = 7
        assert!(parse(&d).is_none());
        d = mk3(0, 18);
        assert!(parse(&d).is_some()); // 0 tags ok
        d = mk3(2_000_000, 18);
        assert!(parse(&d).is_none());
    }
}
