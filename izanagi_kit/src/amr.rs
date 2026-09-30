//! AMR (Adaptive Multi-Rate) audio files — `#!AMR` / `#!AMR-WB` headers (RFC 4867 §5).
//!
//! ```
//! let mut d = b"#!AMR\n".to_vec();
//! d.push(0x3c); // FT=7 frame-type byte
//! d.extend_from_slice(&[0u8; 31]);
//! let a = izanagi_kit::amr::parse(&d).unwrap();
//! assert!(!a.wideband);
//! assert_eq!(a.frames, 1);
//! assert_eq!(izanagi_kit::amr::detect(&d), true);
//! ```

/// Parsed AMR header + frame census.
#[derive(Debug, Clone)]
pub struct Amr {
    /// AMR-WB (`#!AMR-WB`) vs AMR-NB.
    pub wideband: bool,
    /// Frame count walked with the codec's frame-size table.
    pub frames: u32,
    /// Total duration in milliseconds (20 ms per speech frame).
    pub duration_ms: u32,
    /// Distinct frame-type codes seen.
    pub frame_types: Vec<u8>,
    /// Bytes after the magic that were not full frames.
    pub trailing: usize,
}

/// AMR-NB frame sizes including the 1-byte frame header, indexed by FT.
const NB_SIZES: [u32; 16] = [13, 14, 16, 18, 20, 21, 27, 32, 6, 7, 6, 6, 1, 1, 1, 1];
/// AMR-WB frame sizes including the 1-byte frame header, indexed by FT.
const WB_SIZES: [u32; 16] = [18, 24, 33, 37, 41, 47, 51, 59, 61, 6, 6, 0, 0, 0, 1, 1];

/// Detects `#!AMR\n` or `#!AMR-WB\n` magic.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.starts_with(b"#!AMR\n") || b.starts_with(b"#!AMR-WB\n")
}

/// Parses an AMR stream. Returns `None` without the magic or on a zero-length frame table hit.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Amr> {
    let (wideband, off) = if b.starts_with(b"#!AMR-WB\n") {
        (true, 9)
    } else if b.starts_with(b"#!AMR\n") {
        (false, 6)
    } else {
        return None;
    };
    let table = if wideband { &WB_SIZES } else { &NB_SIZES };
    let mut f = Amr {
        wideband,
        frames: 0,
        duration_ms: 0,
        frame_types: Vec::new(),
        trailing: 0,
    };
    let mut i = off;
    while i < b.len() {
        let ft = (b[i] >> 3) & 0x0f;
        let size = table[ft as usize];
        if size == 0 || i + size as usize > b.len() {
            break;
        }
        f.frames += 1;
        f.duration_ms = f.duration_ms.saturating_add(20);
        if !f.frame_types.contains(&ft) {
            f.frame_types.push(ft);
        }
        i += size as usize;
    }
    f.trailing = b.len() - i;
    if f.frames == 0 {
        return None;
    }
    Some(f)
}

/// Magic as text for display.
#[must_use]
pub fn codec_name(a: &Amr) -> String {
    if a.wideband {
        String::from("AMR-WB")
    } else {
        String::from("AMR-NB")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = b"#!AMR\n".to_vec();
        d.push(0x3c); // FT=7
        d.extend_from_slice(&[0u8; 31]);
        d.push(0x00); // FT=0
        d.extend_from_slice(&[0u8; 12]);
        d
    }

    #[test]
    fn parses() {
        let f = parse(&fixture()).unwrap();
        assert_eq!(f.frames, 2);
        assert_eq!(f.duration_ms, 40);
        assert_eq!(f.frame_types, vec![7, 0]);
        assert_eq!(f.trailing, 0);
    }

    #[test]
    fn wideband() {
        let mut d = b"#!AMR-WB\n".to_vec();
        d.push(0x00);
        d.extend_from_slice(&[0u8; 17]);
        let f = parse(&d).unwrap();
        assert!(f.wideband);
        assert_eq!(codec_name(&f), "AMR-WB");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"#!AMR\n").is_none());
        assert!(parse(b"#!AMR\n\x50").is_none());
        assert!(!detect(b"#!AMR"));
        assert!(detect(b"#!AMR-WB\n"));
    }
}
