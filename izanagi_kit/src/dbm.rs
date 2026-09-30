//! DigiBooster Pro `.dbm` module header scanner.
//!
//! `.dbm` files open with the four bytes `DBM0` (big-endian `0x44424D30`),
//! followed by big-endian words:
//!
//! `version u16` \@4, `instruments u16` \@6, `samples u16` \@8,
//! `songs u16` \@10, `patterns u16` \@12, `channels u16` \@14,
//! then a variable-length name table. All multi-byte fields are
//! big-endian (Amiga heritage).
//!
//! ```
//! let mut f = vec![0u8; 32];
//! f[..4].copy_from_slice(b"DBM0");
//! f[4..6].copy_from_slice(&[0, 1]);      // version 1
//! f[6..8].copy_from_slice(&[0, 5]);      // instruments
//! f[8..10].copy_from_slice(&[0, 5]);     // samples
//! f[12..14].copy_from_slice(&[0, 9]);    // patterns
//! f[14..16].copy_from_slice(&[0, 8]);    // channels
//! let m = izanagi_kit::dbm::parse(&f).unwrap();
//! assert_eq!(m.channels, 8);
//! assert_eq!(m.instruments, 5);
//! ```
//!
//! Reference: DigiBooster Pro module format (dbpro format documentation;
//! mirrored in libxmp / MilkyTracker loaders).

/// Parsed DBM header.
#[derive(Debug, Clone, PartialEq)]
pub struct Dbm {
    /// Format version word.
    pub version: u16,
    /// Instrument count.
    pub instruments: u16,
    /// Sample count.
    pub samples: u16,
    /// Song count (DBM files can hold several songs).
    pub songs: u16,
    /// Pattern count.
    pub patterns: u16,
    /// Channel count (typically 4–16).
    pub channels: u16,
}

fn u16be(d: &[u8], off: usize) -> u16 {
    ((d[off] as u16) << 8) | d[off + 1] as u16
}

/// Parse a `.dbm` file; `None` if the `DBM0` signature is absent.
pub fn parse(d: &[u8]) -> Option<Dbm> {
    if d.len() < 16 {
        return None;
    }
    if &d[..4] != b"DBM0" {
        return None;
    }
    let version = u16be(d, 4);
    let channels = u16be(d, 14);
    if channels == 0 || channels > 32 {
        return None;
    }
    Some(Dbm {
        version,
        instruments: u16be(d, 6),
        samples: u16be(d, 8),
        songs: u16be(d, 10),
        patterns: u16be(d, 12),
        channels,
    })
}

/// `true` if the buffer looks like a `.dbm` module.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dbm() -> Vec<u8> {
        let mut f = vec![0u8; 32];
        f[..4].copy_from_slice(b"DBM0");
        f[4..6].copy_from_slice(&[0, 1]);
        f[6..8].copy_from_slice(&[0, 5]);
        f[8..10].copy_from_slice(&[0, 5]);
        f[10..12].copy_from_slice(&[0, 1]);
        f[12..14].copy_from_slice(&[0, 9]);
        f[14..16].copy_from_slice(&[0, 8]);
        f
    }

    #[test]
    fn parses() {
        let m = parse(&dbm()).unwrap();
        assert_eq!(m.version, 1);
        assert_eq!(m.instruments, 5);
        assert_eq!(m.samples, 5);
        assert_eq!(m.songs, 1);
        assert_eq!(m.patterns, 9);
        assert_eq!(m.channels, 8);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"DBM0").is_none());
        let mut f = dbm();
        f[3] = b'1'; // wrong magic digit
        assert!(parse(&f).is_none());
        let mut g = dbm();
        g[14] = 0;
        g[15] = 0; // zero channels
        assert!(parse(&g).is_none());
        let mut h = dbm();
        h[15] = 40; // >32 channels
        assert!(parse(&h).is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(&dbm()));
        assert!(!detect(b"DBM0"));
    }
}
