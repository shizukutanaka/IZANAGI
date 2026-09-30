//! Silhouette Studio3 cutting file: starts with the ASCII signature
//! `studio3` followed by a big-endian header (`version` u32 and a
//! length-prefixed prolog) — used by Silhouette cutting machines
//! before the encrypted/binary body.
//!
//! ```
//! let mut d = b"studio3".to_vec();
//! d.extend_from_slice(&[0, 0, 0, 0]);      // version
//! d.extend_from_slice(&[0, 0, 0, 4]);      // prolog len = 4
//! d.extend_from_slice(b"\x00\x00\x00\x00");
//! let s = izanagi_kit::studio3::parse(&d).unwrap();
//! assert_eq!(s.version, 0);
//! assert_eq!(s.prolog_len, 4);
//! ```

/// A parsed Studio3 header.
#[derive(Clone, Debug)]
pub struct Studio3 {
    /// Header version.
    pub version: u32,
    /// Prolog/segment length declared after the version.
    pub prolog_len: u32,
}

fn be32(d: &[u8], o: usize) -> u32 {
    (u32::from(d[o]) << 24)
        | (u32::from(d[o + 1]) << 16)
        | (u32::from(d[o + 2]) << 8)
        | u32::from(d[o + 3])
}

/// Parse a Studio3 file; `None` without the `studio3` signature.
pub fn parse(d: &[u8]) -> Option<Studio3> {
    if !d.starts_with(b"studio3") {
        return None;
    }
    if d.len() < 15 {
        return None;
    }
    let version = be32(d, 7);
    let prolog_len = be32(d, 11);
    Some(Studio3 {
        version,
        prolog_len,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let mut d = b"studio3".to_vec();
        d.extend_from_slice(&[0, 0, 0, 2]);
        d.extend_from_slice(&[0, 0, 0, 0x40]);
        d.extend_from_slice(&[0u8; 0x40]);
        let s = parse(&d).unwrap();
        assert_eq!(s.version, 2);
        assert_eq!(s.prolog_len, 0x40);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"studio3").is_none()); // truncated
        assert!(parse(b"Studio3\x00\x00\x00\x00\x00\x00\x00\x00").is_none()); // case
    }
}
