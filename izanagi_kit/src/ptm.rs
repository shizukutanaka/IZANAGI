//! Poly Tracker `.ptm` module header scanner.
//!
//! `.ptm` opens with `PTMF`, then a version byte, a 28-byte song name, an
//! `0x1A` EOF byte, a u16 file version, a u16 reserved field, and the
//! counts block:
//!
//! `num_orders u16`, `num_patterns u16`, `num_channels u16`,
//! `num_samples u16`, `attr_flags u16`, `songinfo_len u32`.
//!
//! ```
//! let mut f = vec![0u8; 64];
//! f[..4].copy_from_slice(b"PTMF");
//! f[4] = 0x11;                        // v1.1 family byte
//! f[5..10].copy_from_slice(b"Title");
//! f[33] = 0x1A;
//! f[34..36].copy_from_slice(&0x0203u16.to_le_bytes()); // file v2.3
//! f[38..40].copy_from_slice(&4u16.to_le_bytes());    // orders
//! f[40..42].copy_from_slice(&8u16.to_le_bytes());    // patterns
//! f[42..44].copy_from_slice(&16u16.to_le_bytes());   // channels
//! let p = izanagi_kit::ptm::parse(&f).unwrap();
//! assert_eq!(p.song_name.as_deref(), Some("Title"));
//! assert_eq!(p.channels, 16);
//! assert_eq!(p.patterns, 8);
//! ```
//!
//! Reference: Poly Tracker file format (ptm-form.txt, "The Coder" /
//! Respect Insects; used by OpenMPT's `Load_ptm`).

/// Parsed PTM header.
#[derive(Debug, Clone, PartialEq)]
pub struct Ptm {
    /// Header version byte after `PTMF` (tracker family).
    pub version: u8,
    /// Song name (28 bytes at offset 5, NUL-padded).
    pub song_name: Option<String>,
    /// Raw file-version field (e.g. `0x0203` ≈ "v2.3").
    pub file_version: u16,
    /// Order-table entries (`num_orders`).
    pub orders: u16,
    /// Pattern count (`num_patterns`).
    pub patterns: u16,
    /// Channel count (`num_channels`, 1–32).
    pub channels: u16,
    /// Sample count (`num_samples`).
    pub samples: u16,
    /// Attribute/flags word.
    pub flags: u16,
    /// Byte length of the embedded song-information block.
    pub songinfo_len: u32,
}

fn u16le(d: &[u8], off: usize) -> u16 {
    d[off] as u16 | ((d[off + 1] as u16) << 8)
}

fn u32le(d: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([d[off], d[off + 1], d[off + 2], d[off + 3]])
}

/// Parse a `.ptm` file; `None` if the signature is absent or implausible.
pub fn parse(d: &[u8]) -> Option<Ptm> {
    if d.len() < 0x30 {
        return None;
    }
    if &d[..4] != b"PTMF" {
        return None;
    }
    if d[33] != 0x1A {
        return None;
    }
    let channels = u16le(d, 42);
    if channels == 0 || channels > 32 {
        return None;
    }
    let name_len = d[5..33].iter().position(|&c| c == 0).unwrap_or(28);
    let song_name = if name_len == 0 {
        None
    } else {
        core::str::from_utf8(&d[5..5 + name_len])
            .ok()
            .map(|s| s.to_string())
    };
    Some(Ptm {
        version: d[4],
        song_name,
        file_version: u16le(d, 34),
        orders: u16le(d, 38),
        patterns: u16le(d, 40),
        channels,
        samples: u16le(d, 44),
        flags: u16le(d, 46),
        songinfo_len: u32le(d, 48),
    })
}

/// `true` if the buffer looks like a `.ptm` module.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ptm() -> Vec<u8> {
        let mut f = vec![0u8; 64];
        f[..4].copy_from_slice(b"PTMF");
        f[4] = 0x11;
        f[5..10].copy_from_slice(b"Title");
        f[33] = 0x1A;
        f[34..36].copy_from_slice(&0x0203u16.to_le_bytes());
        f[38..40].copy_from_slice(&4u16.to_le_bytes());
        f[40..42].copy_from_slice(&8u16.to_le_bytes());
        f[42..44].copy_from_slice(&16u16.to_le_bytes());
        f[44..46].copy_from_slice(&5u16.to_le_bytes());
        f[46..48].copy_from_slice(&3u16.to_le_bytes());
        f[48..52].copy_from_slice(&96u32.to_le_bytes());
        f
    }

    #[test]
    fn parses() {
        let p = parse(&ptm()).unwrap();
        assert_eq!(p.version, 0x11);
        assert_eq!(p.song_name.as_deref(), Some("Title"));
        assert_eq!(p.file_version, 0x0203);
        assert_eq!(p.orders, 4);
        assert_eq!(p.patterns, 8);
        assert_eq!(p.channels, 16);
        assert_eq!(p.samples, 5);
        assert_eq!(p.flags, 3);
        assert_eq!(p.songinfo_len, 96);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"PTMF").is_none());
        let mut f = ptm();
        f[0] = b'x';
        assert!(parse(&f).is_none());
        let mut g = ptm();
        g[33] = 0;
        assert!(parse(&g).is_none());
        let mut h = ptm();
        h[42] = 0;
        h[43] = 0;
        assert!(parse(&h).is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(&ptm()));
        assert!(!detect(b"PTMF"));
    }
}
