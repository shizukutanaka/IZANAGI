//! Adobe DNG (Digital Negative) header census.
//!
//! DNG is TIFF (`II*\x00` or `MM\x00*`); the signature is IFD0 tag
//! `50706` (`DNGVersion`, 4 bytes `major.minor.0.0`). Census walks IFD0,
//! reports the version and spots `UniqueCameraModel` (0xC614) and
//! `CFAPlaneColor`/opcode tags.
//!
//! ```
//! let mut d = b"II*\x00\x08\x00\x00\x00".to_vec();
//! d.extend([1, 0, 0x12, 0xC6, 1, 0, 4, 0, 0, 0, 1, 4, 0, 0, 0, 0, 0, 0]);
//! let g = izanagi_kit::dng::parse(&d).unwrap();
//! assert_eq!(g.version, Some([1, 4, 0, 0]));
//! ```

/// DNG version tag id.
pub const DNG_VERSION_TAG: u16 = 0xC612;
/// `UniqueCameraModel` tag id.
pub const CAMERA_MODEL_TAG: u16 = 0xC614;

/// Census fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dng {
    /// Big-endian TIFF (`MM`).
    pub big_endian: bool,
    /// IFD0 offset.
    pub ifd0: u32,
    /// IFD0 entry count.
    pub entries: u16,
    /// `DNGVersion` payload `[major, minor, 0, 0]` when readable.
    pub version: Option<[u8; 4]>,
    /// `UniqueCameraModel` (0xC614) tag seen.
    pub camera_model: bool,
    /// Count of tags in the 0xC000..=0xFFFF DNG-extension range.
    pub dng_tags: u16,
}

fn rd16(b: &[u8], i: usize, be: bool) -> u16 {
    if be {
        ((b[i] as u16) << 8) | (b[i + 1] as u16)
    } else {
        (b[i] as u16) | ((b[i + 1] as u16) << 8)
    }
}

fn rd32(b: &[u8], i: usize, be: bool) -> u32 {
    let (a, c, d, e) = (
        b[i] as u32,
        b[i + 1] as u32,
        b[i + 2] as u32,
        b[i + 3] as u32,
    );
    if be {
        (a << 24) | (c << 16) | (d << 8) | e
    } else {
        a | (c << 8) | (d << 16) | (e << 24)
    }
}

fn has_version_tag(b: &[u8], be: bool, ifd0: u32) -> bool {
    let Some(o) = usize::try_from(ifd0).ok().filter(|&o| o + 2 <= b.len()) else {
        return false;
    };
    let n = usize::from(rd16(b, o, be)).min(1024);
    (0..n).any(|i| {
        let e = o + 2 + i * 12;
        e + 2 <= b.len() && rd16(b, e, be) == DNG_VERSION_TAG
    })
}

/// `true` on TIFF magic + a `DNGVersion` (0xC612) tag in IFD0.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    if b.len() < 20 {
        return false;
    }
    let be = match &b[0..4] {
        m if m == b"II*\x00" => false,
        m if m == b"MM\x00*" => true,
        _ => return false,
    };
    has_version_tag(b, be, rd32(b, 4, be))
}

/// Census; `None` on non-DNG.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Dng> {
    if !detect(b) {
        return None;
    }
    let big_endian = b[0] == b'M';
    let ifd0 = rd32(b, 4, big_endian);
    let o = usize::try_from(ifd0).ok()?;
    let entries = rd16(b, o, big_endian);
    let mut version = None;
    let mut camera_model = false;
    let mut dng_tags = 0u16;
    for i in 0..usize::from(entries).min(1024) {
        let e = o + 2 + i * 12;
        if e + 12 > b.len() {
            break;
        }
        let tag = rd16(b, e, big_endian);
        if tag >= 0xC000 {
            dng_tags = dng_tags.saturating_add(1);
        }
        match tag {
            t if t == DNG_VERSION_TAG => {
                // BYTE type: the 4-byte value sits inline in the value field.
                version = Some([b[e + 8], b[e + 9], b[e + 10], b[e + 11]]);
            }
            t if t == CAMERA_MODEL_TAG => camera_model = true,
            _ => {}
        }
    }
    Some(Dng {
        big_endian,
        ifd0,
        entries,
        version,
        camera_model,
        dng_tags,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = b"II*\x00\x08\x00\x00\x00".to_vec();
        d.extend([2, 0]);
        d.extend([0x12, 0xC6, 1, 0, 4, 0, 0, 0, 1, 4, 0, 0]);
        d.extend([0x14, 0xC6, 2, 0, 10, 0, 0, 0, 0x40, 0, 0, 0]);
        d.extend([0, 0, 0, 0]);
        d
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        assert!(!detect(b"II*\x00\x08\x00\x00\x00\x00\x00"));
    }

    #[test]
    fn parses() {
        let g = parse(&fixture()).unwrap();
        assert_eq!(g.entries, 2);
        assert_eq!(g.version, Some([1, 4, 0, 0]));
        assert!(g.camera_model);
        assert_eq!(g.dng_tags, 2);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"II*\x00\x08\x00\x00\x00CR\x02\x00").is_none());
    }
}
