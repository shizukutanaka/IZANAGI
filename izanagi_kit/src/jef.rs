//! Janome JEF embroidery file (heuristic — the format has no magic):
//! a little-endian `u32` at offset 0 gives the byte offset of the
//! stitch data, followed by a header containing an ASCII creation
//! timestamp (`yyyy:mm:dd hh:mm:ss`) around offset 8..27 and an
//! optional hoop/string fields. Rejected when the offset is
//! implausible.
//!
//! ```
//! let mut d = vec![0u8; 116];
//! d[0..4].copy_from_slice(&116u32.to_le_bytes());
//! d[8..27].copy_from_slice(b"2024:01:15 10:30:00");
//! d.extend_from_slice(&[0u8; 8]);
//! let j = izanagi_kit::jef::parse(&d).unwrap();
//! assert_eq!(j.stitch_offset, 116);
//! assert_eq!(j.timestamp.as_deref(), Some("2024:01:15 10:30:00"));
//! ```

use std::string::String;

/// A parsed JEF file header.
#[derive(Clone, Debug)]
pub struct Jef {
    /// Byte offset where the stitch data begins (from file start).
    pub stitch_offset: u32,
    /// Creation timestamp string when it parses as
    /// `yyyy:mm:dd hh:mm:ss`.
    pub timestamp: Option<String>,
}

fn le32(d: &[u8], o: usize) -> u32 {
    u32::from(d[o])
        | u32::from(d[o + 1]) << 8
        | u32::from(d[o + 2]) << 16
        | u32::from(d[o + 3]) << 24
}

fn looks_like_ts(b: &[u8]) -> bool {
    // "yyyy:mm:dd hh:mm:ss" — digits with ':' and ' ' separators.
    if b.len() < 19 {
        return false;
    }
    for (i, &c) in b[..19].iter().enumerate() {
        let want_digit = !matches!(i, 4 | 7 | 10 | 13 | 16);
        let sep = match i {
            4 | 7 | 13 | 16 => c == b':',
            10 => c == b' ',
            _ => true,
        };
        if want_digit && !c.is_ascii_digit() {
            return false;
        }
        if !want_digit && !sep {
            return false;
        }
    }
    true
}

/// Parse a JEF file; `None` when the stitch offset is out of range or
/// the header looks nothing like JEF.
pub fn parse(d: &[u8]) -> Option<Jef> {
    if d.len() < 32 {
        return None;
    }
    let stitch_offset = le32(d, 0);
    // Stitch data must land inside the file with a plausible header.
    if stitch_offset < 16 || stitch_offset as usize > d.len() {
        return None;
    }
    let timestamp = if d.len() >= 27 && looks_like_ts(&d[8..27]) {
        Some(String::from_utf8_lossy(&d[8..27]).to_string())
    } else {
        None
    };
    Some(Jef {
        stitch_offset,
        timestamp,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let mut d = vec![0u8; 116];
        d[0..4].copy_from_slice(&116u32.to_le_bytes());
        d[8..27].copy_from_slice(b"2024:01:15 10:30:00");
        d.extend_from_slice(&[1, 2, 3]);
        let j = parse(&d).unwrap();
        assert_eq!(j.stitch_offset, 116);
        assert_eq!(j.timestamp.as_deref(), Some("2024:01:15 10:30:00"));
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        let mut d = vec![0u8; 64];
        assert!(parse(&d).is_none()); // offset 0
        d[0..4].copy_from_slice(&500u32.to_le_bytes());
        assert!(parse(&d).is_none()); // beyond len
        d[0..4].copy_from_slice(&16u32.to_le_bytes());
        assert!(parse(&d).is_some()); // plausible
    }
}
