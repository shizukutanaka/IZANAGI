//! FLIF (Free Lossless Image Format) scanner.
//!
//! A FLIF file starts with the ASCII magic `FLIF`, then one byte
//! whose high nibble is the interlace flag (1 = non-interlaced,
//! 2 = interlaced) and whose low nibble is the channel count
//! (1 gray, 3 RGB, 4 RGBA, also 2 gray+alpha). The next byte is
//! `0` for stills or `'0'`–`'9'` + extras for animations.
//!
//! ```
//! let d = b"FLIF\x13\x30\x01\x02"; // non-interlaced, 3 channels
//! let f = izanagi_kit::flif::parse(d).unwrap();
//! assert!(!f.interlaced);
//! assert_eq!(f.channels, 3);
//! ```
//!
//! Reference: FLIF specification (flif.info / FreeLossless Image
//! Format spec) — the `FLIF` magic and the packed
//! interlace×channels descriptor byte.

/// Parsed FLIF fields.
#[derive(Debug, Clone, PartialEq)]
pub struct Flif {
    /// `true` for interlaced (progressive) files.
    pub interlaced: bool,
    /// Channel count (1 gray, 2 GA, 3 RGB, 4 RGBA).
    pub channels: u8,
    /// `true` for animation (second descriptor byte `> '0'`? FLIF
    /// uses ASCII digit count of frames; `0` = still image).
    pub animation: bool,
    /// Bytes following the descriptor pair (the entropy stream).
    pub stream_bytes: usize,
}

/// Parse a FLIF header; `None` unless `FLIF` magic + a valid
/// descriptor pair are present.
pub fn parse(d: &[u8]) -> Option<Flif> {
    if d.len() < 6 || &d[..4] != b"FLIF" {
        return None;
    }
    let (il, ch) = (d[4] >> 4, d[4] & 0xF);
    if !(1..=2).contains(&il) || !(1..=4).contains(&ch) {
        return None;
    }
    Some(Flif {
        interlaced: il == 2,
        channels: ch,
        animation: d[5] != b'0',
        stream_bytes: d.len() - 6,
    })
}

/// `true` if the buffer looks like a FLIF file.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let f = parse(b"FLIF\x13\x30\xAA\xBB").unwrap();
        assert!(!f.interlaced);
        assert_eq!(f.channels, 3);
        assert!(!f.animation);
        assert_eq!(f.stream_bytes, 2);
        let f = parse(b"FLIF\x24\x39\x00").unwrap();
        assert!(f.interlaced);
        assert_eq!(f.channels, 4);
        assert!(f.animation);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"FLIF\x00\x30").is_none()); // il = 0
        assert!(parse(b"FLIF\x15\x30").is_none()); // ch = 5
        assert!(parse(b"FLI").is_none());
    }

    #[test]
    fn detects() {
        assert!(detect(b"FLIF\x13\x30\x01"));
        assert!(!detect(b"FLIF"));
    }
}
