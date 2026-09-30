//! MED / OctaMED `.med` module header scanner.
//!
//! OctaMED modules (MMD format) begin with a 4-byte signature:
//! `MMD0`, `MMD1`, `MMD2`, `MMD3` (OctaMED versions 0–3), or the original
//! MED `MED?` (pre-MMD). After the tag comes a big-endian `song_len u32`
//! (offset of song data within the file) and further big-endian block
//! pointers — this parser reads the signature, the format number, and the
//! declared song-data offset.
//!
//! ```
//! let mut f = vec![0u8; 0x200];
//! f[..4].copy_from_slice(b"MMD1");
//! f[4..8].copy_from_slice(&[0, 0, 1, 0x40]); // song_len = 0x140 (BE)
//! let m = izanagi_kit::med::parse(&f).unwrap();
//! assert_eq!(m.version, 1);
//! assert_eq!(m.song_offset, 0x140);
//! ```
//!
//! Reference: the OctaMED MMD format description (Teijo Kinnunen's
//! `med-form.doc` and the public `OctaMED Soundstudio` sources used by
//! libxmp / UADE).

/// Parsed MED/MMD header.
#[derive(Debug, Clone, PartialEq)]
pub struct Med {
    /// MMD format version: `0`–`3` for `MMD0`–`MMD3`.
    pub version: u8,
    /// Big-endian offset of the song structure within the file.
    pub song_offset: u32,
}

/// Parse a `.med` file; `None` if the `MMDn` magic is absent.
pub fn parse(d: &[u8]) -> Option<Med> {
    if d.len() < 8 {
        return None;
    }
    let version = match &d[..4] {
        b"MMD0" => 0,
        b"MMD1" => 1,
        b"MMD2" => 2,
        b"MMD3" => 3,
        _ => return None,
    };
    let song_offset =
        ((d[4] as u32) << 24) | ((d[5] as u32) << 16) | ((d[6] as u32) << 8) | d[7] as u32;
    Some(Med {
        version,
        song_offset,
    })
}

/// `true` if the buffer looks like a MED/OctaMED module.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn med() -> Vec<u8> {
        let mut f = vec![0u8; 0x400];
        f[..4].copy_from_slice(b"MMD1");
        f[4..8].copy_from_slice(&[0, 0, 1, 0x40]);
        f
    }

    #[test]
    fn parses() {
        let m = parse(&med()).unwrap();
        assert_eq!(m.version, 1);
        assert_eq!(m.song_offset, 0x140);
    }

    #[test]
    fn versions() {
        for (tag, v) in [(b"MMD0", 0u8), (b"MMD2", 2), (b"MMD3", 3)] {
            let mut f = med();
            f[..4].copy_from_slice(tag);
            assert_eq!(parse(&f).unwrap().version, v);
        }
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"MMD1").is_none());
        let mut f = med();
        f[..4].copy_from_slice(b"MMD4");
        assert!(parse(&f).is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(&med()));
        assert!(!detect(b"MMD9deadbeef"));
    }
}
