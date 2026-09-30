//! UltraTracker `.ult` module header scanner.
//!
//! `.ult` files open with the 14-byte signature `MAS_UTrack_V00` plus one
//! ASCII version digit (`'1'`–`'4'`), making a 15-byte ID. Then:
//!
//! `song_name[32]` \@15, `reserved u8` \@47, then per-sample headers.
//!
//! ```
//! let mut f = vec![0u8; 48];
//! f[..14].copy_from_slice(b"MAS_UTrack_V00");
//! f[14] = b'4';                    // ULT v4
//! f[15..20].copy_from_slice(b"Track");
//! let u = izanagi_kit::ult::parse(&f).unwrap();
//! assert_eq!(u.version, 4);
//! assert_eq!(u.song_name.as_deref(), Some("Track"));
//! ```
//!
//! Reference: UltraTracker file format (`ULTFORM.TXT`, M. van der Vossen;
//! mirrored at modland.com and in Schism Tracker's `fmt/ult.c`).

/// Parsed ULT header.
#[derive(Debug, Clone, PartialEq)]
pub struct Ult {
    /// ULT format version (`1`–`4`).
    pub version: u8,
    /// Song name, up to 32 bytes (NUL-padded).
    pub song_name: Option<String>,
}

/// Parse an `.ult` file; `None` if the signature is absent.
pub fn parse(d: &[u8]) -> Option<Ult> {
    if d.len() < 48 {
        return None;
    }
    if &d[..14] != b"MAS_UTrack_V00" {
        return None;
    }
    let version = match d[14] {
        b'1'..=b'4' => d[14] - b'0',
        _ => return None,
    };
    let name_len = d[15..47].iter().position(|&c| c == 0).unwrap_or(32);
    let song_name = if name_len == 0 {
        None
    } else {
        core::str::from_utf8(&d[15..15 + name_len])
            .ok()
            .map(|s| s.to_string())
    };
    Some(Ult { version, song_name })
}

/// `true` if the buffer looks like an `.ult` module.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ult() -> Vec<u8> {
        let mut f = vec![0u8; 64];
        f[..14].copy_from_slice(b"MAS_UTrack_V00");
        f[14] = b'4';
        f[15..20].copy_from_slice(b"Track");
        f
    }

    #[test]
    fn parses() {
        let u = parse(&ult()).unwrap();
        assert_eq!(u.version, 4);
        assert_eq!(u.song_name.as_deref(), Some("Track"));
    }

    #[test]
    fn versions() {
        for v in b'1'..=b'4' {
            let mut f = ult();
            f[14] = v;
            assert_eq!(parse(&f).unwrap().version, v - b'0');
        }
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"MAS_UTrack_V001").is_none()); // too short
        let mut f = ult();
        f[14] = b'5'; // unknown version
        assert!(parse(&f).is_none());
        let mut g = ult();
        g[0] = b'x';
        assert!(parse(&g).is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(&ult()));
        assert!(!detect(b"MAS_UTrack_V00"));
    }
}
