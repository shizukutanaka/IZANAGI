//! Farandole Composer `.far` module header scanner.
//!
//! `.far` opens with `FAR` plus a `0xFE` marker byte, then a 40-byte
//! song name (space- and NUL-padded) and a u16 text-region length at
//! offset `0x2C`. The remaining bytes of the 8×16-byte fixed region carry
//! song-position and channel-mapping bytes.
//!
//! ```
//! let mut f = vec![0u8; 0x30];
//! f[..4].copy_from_slice(b"FAR\xFE");
//! f[4..9].copy_from_slice(b"Intro");
//! f[0x2C] = 3; f[0x2D] = 0;         // 3 text bytes follow the name
//! let m = izanagi_kit::far::parse(&f).unwrap();
//! assert_eq!(m.song_name.as_deref(), Some("Intro"));
//! assert_eq!(m.text_len, 3);
//! ```
//!
//! Reference: the Farandole Composer format description mirrored as
//! `far-form.doc` / modland docs (Bas Lijkendijk / Farandole).

/// Parsed FAR header.
#[derive(Debug, Clone, PartialEq)]
pub struct Far {
    /// Song name (40 bytes at offset 4, NUL/space-padded).
    pub song_name: Option<String>,
    /// Length of the title/text region that follows the name.
    pub text_len: u16,
}

/// Parse a `.far` file; `None` if the `FAR\xFE` signature is absent.
pub fn parse(d: &[u8]) -> Option<Far> {
    if d.len() < 0x2E {
        return None;
    }
    if &d[..4] != b"FAR\xFE" {
        return None;
    }
    let mut end = 0x2C;
    while end > 4 && (d[end - 1] == 0 || d[end - 1] == b' ') {
        end -= 1;
    }
    let song_name = if end == 4 {
        None
    } else {
        core::str::from_utf8(&d[4..end]).ok().map(|s| s.to_string())
    };
    Some(Far {
        song_name,
        text_len: d[0x2C] as u16 | ((d[0x2D] as u16) << 8),
    })
}

/// `true` if the buffer looks like a `.far` module.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn far() -> Vec<u8> {
        let mut f = vec![0u8; 0x40];
        f[..4].copy_from_slice(b"FAR\xFE");
        f[4..9].copy_from_slice(b"Intro");
        f[0x2C] = 3;
        f
    }

    #[test]
    fn parses() {
        let m = parse(&far()).unwrap();
        assert_eq!(m.song_name.as_deref(), Some("Intro"));
        assert_eq!(m.text_len, 3);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"FAR\xFE").is_none());
        let mut f = far();
        f[3] = 0;
        assert!(parse(&f).is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(&far()));
        assert!(!detect(b"FAR\xFE"));
    }
}
