//! MultiTracker `.mtm` module header scanner.
//!
//! `.mtm` layout (little-endian throughout):
//!
//! `MTM` magic \@0 (3 bytes), `version u8` \@3, `song_name[20]` \@4,
//! `num_tracks u16` \@24, `last_pattern u8` \@26, `last_order u8` \@27,
//! `num_comments u16` \@28, `num_samples u8` \@30, `attribute u8` \@31,
//! `beats_per_track u16` \@32, `num_channels u8` \@34,
//! `channel_pan[32] u8` \@35, then instrument/sample blocks and the
//! per-track comment area.
//!
//! ```
//! let mut f = vec![0u8; 67];
//! f[..3].copy_from_slice(b"MTM");
//! f[3] = 0x10;                        // version 1.0
//! f[4..9].copy_from_slice(b"Title");
//! f[24..26].copy_from_slice(&4u16.to_le_bytes());   // tracks
//! f[26] = 7;                          // last pattern
//! f[27] = 15;                         // last order
//! f[30] = 3;                          // samples
//! f[32..34].copy_from_slice(&16u16.to_le_bytes());  // beats/track
//! f[34] = 8;                          // channels
//! let m = izanagi_kit::mtm::parse(&f).unwrap();
//! assert_eq!(m.song_name.as_deref(), Some("Title"));
//! assert_eq!(m.channels, 8);
//! assert_eq!(m.tracks, 4);
//! ```
//!
//! Reference: the MultiTracker format description circulated as
//! `mtm-form.txt` / `mtm.txt` (Renaissance / B. Brown docs).

/// Parsed MTM header.
#[derive(Debug, Clone, PartialEq)]
pub struct Mtm {
    /// File format version byte (`0x10` = 1.0, `0x11` = 1.1, …).
    pub version: u8,
    /// Song name (20 bytes, NUL-padded).
    pub song_name: Option<String>,
    /// Number of track patterns (`num_tracks`).
    pub tracks: u16,
    /// Highest pattern index used (`last_pattern`).
    pub last_pattern: u8,
    /// Last order index (`last_order`).
    pub last_order: u8,
    /// Declared comment-field count (`num_comments`).
    pub comments: u16,
    /// Declared sample count (`num_samples`).
    pub samples: u8,
    /// Beats per track (`beats_per_track`).
    pub beats_per_track: u16,
    /// Logical channel count (`num_channels`, 1–32).
    pub channels: u8,
}

fn u16le(d: &[u8], off: usize) -> u16 {
    d[off] as u16 | ((d[off + 1] as u16) << 8)
}

/// Parse an `.mtm` file; `None` if the magic is absent or implausible.
pub fn parse(d: &[u8]) -> Option<Mtm> {
    if d.len() < 67 {
        return None;
    }
    if &d[..3] != b"MTM" {
        return None;
    }
    // Known versions are 1.x: 0x10 and 0x11 cover virtually all files.
    if !(0x10..=0x20).contains(&d[3]) {
        return None;
    }
    let channels = d[34];
    if channels == 0 || channels > 32 {
        return None;
    }
    let name_len = d[4..24].iter().position(|&c| c == 0).unwrap_or(20);
    let song_name = if name_len == 0 {
        None
    } else {
        core::str::from_utf8(&d[4..4 + name_len])
            .ok()
            .map(|s| s.to_string())
    };
    Some(Mtm {
        version: d[3],
        song_name,
        tracks: u16le(d, 24),
        last_pattern: d[26],
        last_order: d[27],
        comments: u16le(d, 28),
        samples: d[30],
        beats_per_track: u16le(d, 32),
        channels,
    })
}

/// `true` if the buffer looks like an `.mtm` module.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mtm() -> Vec<u8> {
        let mut f = vec![0u8; 80];
        f[..3].copy_from_slice(b"MTM");
        f[3] = 0x10;
        f[4..9].copy_from_slice(b"Title");
        f[24..26].copy_from_slice(&4u16.to_le_bytes());
        f[26] = 7;
        f[27] = 15;
        f[28..30].copy_from_slice(&2u16.to_le_bytes());
        f[30] = 3;
        f[32..34].copy_from_slice(&16u16.to_le_bytes());
        f[34] = 8;
        f
    }

    #[test]
    fn parses() {
        let m = parse(&mtm()).unwrap();
        assert_eq!(m.version, 0x10);
        assert_eq!(m.song_name.as_deref(), Some("Title"));
        assert_eq!(m.tracks, 4);
        assert_eq!(m.last_pattern, 7);
        assert_eq!(m.last_order, 15);
        assert_eq!(m.comments, 2);
        assert_eq!(m.samples, 3);
        assert_eq!(m.beats_per_track, 16);
        assert_eq!(m.channels, 8);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"MTM").is_none());
        let mut f = mtm();
        f[0] = b'X';
        assert!(parse(&f).is_none());
        let mut g = mtm();
        g[34] = 0; // zero channels
        assert!(parse(&g).is_none());
        let mut h = mtm();
        h[34] = 33; // >32 channels
        assert!(parse(&h).is_none());
        let mut i = mtm();
        i[3] = 0x50; // implausible version
        assert!(parse(&i).is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(&mtm()));
        assert!(!detect(b"MTM"));
    }
}
