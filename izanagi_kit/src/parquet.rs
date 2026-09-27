//! Apache Parquet file envelope.
//!
//! `PAR1` magic at both ends; the last 8 bytes before the trailing
//! magic are a little-endian u32 footer length, and the footer
//! (Thrift compact-serialized `FileMetaData`, not decoded here) sits
//! before that. `PARE` in place of the trailing `PAR1` marks an
//! encrypted-footer file.
//!
//! ```
//! use izanagi_kit::parquet::{parse, PAR1};
//! let mut d = b"PAR1".to_vec();
//! d.extend_from_slice(&[0x55; 12]); // "row group" bytes
//! d.extend_from_slice(&[0xEE; 20]); // footer (fake thrift)
//! d.extend_from_slice(&20u32.to_le_bytes());
//! d.extend_from_slice(&PAR1);
//! let p = parse(&d).unwrap();
//! assert_eq!(p.meta_len, 20);
//! assert_eq!(p.meta_at, 16); // 4 + 12
//! ```

fn le32(d: &[u8], o: usize) -> Option<u32> {
    Some(
        (*d.get(o)? as u32)
            | (*d.get(o + 1)? as u32) << 8
            | (*d.get(o + 2)? as u32) << 16
            | (*d.get(o + 3)? as u32) << 24,
    )
}

/// Leading and trailing magic `PAR1` (trailing may be `PARE` for
/// encrypted footers).
pub const PAR1: [u8; 4] = *b"PAR1";
/// Trailing magic when the footer is encrypted.
pub const PARE: [u8; 4] = *b"PARE";

/// Parsed Parquet envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Parquet {
    /// Offset of the Thrift `FileMetaData` footer.
    pub meta_at: usize,
    /// Footer length in bytes.
    pub meta_len: u32,
    /// `true` when the trailing magic is `PARE` (footer encrypted).
    pub encrypted: bool,
}

/// Parse `PAR1 … meta_len LE, PAR1|PARE`. `None` on a missing magic
/// or a footer that overruns the file.
pub fn parse(d: &[u8]) -> Option<Parquet> {
    if d.len() < 12 {
        return None;
    }
    if d.get(0..4)? != PAR1 {
        return None;
    }
    let tail = d.len() - 4;
    let encrypted = match d.get(tail..)? {
        t if t == PAR1 => false,
        t if t == PARE => true,
        _ => return None,
    };
    let meta_len = le32(d, tail - 4)?;
    let meta_at = (tail - 4).checked_sub(meta_len as usize)?;
    if meta_at < 4 {
        return None;
    }
    Some(Parquet {
        meta_at,
        meta_len,
        encrypted,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope() {
        let mut d = b"PAR1".to_vec();
        d.extend_from_slice(&[0x55; 12]);
        d.extend_from_slice(&[0xEE; 20]);
        d.extend_from_slice(&20u32.to_le_bytes());
        d.extend_from_slice(&PAR1);
        let p = parse(&d).unwrap();
        assert_eq!(p.meta_at, 16);
        assert_eq!(p.meta_len, 20);
        assert!(!p.encrypted);
        // PARE tail
        let n = d.len();
        d[n - 4..].copy_from_slice(&PARE);
        assert!(parse(&d).unwrap().encrypted);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"PAR1").is_none()); // too short
        assert!(parse(b"\0PAR1\0\0\0\0\0\0\0\0").is_none());
        let mut d = b"PAR1".to_vec();
        d.extend_from_slice(&[0; 4]);
        d.extend_from_slice(&500u32.to_le_bytes()); // meta_len overrun
        d.extend_from_slice(&PAR1);
        assert!(parse(&d).is_none());
        let mut d2 = b"PAR1".to_vec();
        d2.extend_from_slice(&[0; 4]);
        d2.extend_from_slice(&0u32.to_le_bytes()); // meta_len 0 at byte 4 — ok, meta_at = 4..?
                                                   // meta_at = 8-4=4-0 → 4 → valid! check:
        d2.extend_from_slice(&PAR1);
        assert!(parse(&d2).is_some());
    }
}
