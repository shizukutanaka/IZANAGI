//! Nikon NEF (Nikon Electronic Format) header census.
//!
//! NEF is a plain TIFF container (`II*\x00` or `MM\x00*`); the Nikon
//! identity comes from the `Make` tag value `NIKON CORPORATION`, which the
//! writer places within the first page. Census walks IFD0 for the
//! `Make` (0x010F) pointer and verifies the `NIKON` banner there, plus
//! counts entries and spots the `MakerNote` (0x927C) tag.
//!
//! ```
//! let mut d = b"II*\x00\x08\x00\x00\x00".to_vec();
//! d.extend([1, 0, 0x0F, 0x01, 2, 0, 20, 0, 0, 0, 0x20, 0, 0, 0, 0, 0, 0, 0]);
//! d.resize(0x20, 0);
//! d.extend_from_slice(b"NIKON CORPORATION\x00");
//! let n = izanagi_kit::nef::parse(&d).unwrap();
//! assert_eq!(n.entries, 1);
//! assert!(n.make_tag);
//! ```

/// Census fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Nef {
    /// Big-endian TIFF (`MM`) vs little-endian (`II`).
    pub big_endian: bool,
    /// IFD0 offset from the TIFF prologue.
    pub ifd0: u32,
    /// IFD0 entry count when in bounds.
    pub entries: u16,
    /// File offset where the `NIKON` banner was found.
    pub make_offset: usize,
    /// `Make` (0x010F) tag seen in IFD0.
    pub make_tag: bool,
    /// `MakerNote` (0x927C) tag seen in IFD0.
    pub maker_tag: bool,
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

fn find(hay: &[u8], needle: &[u8], limit: usize) -> Option<usize> {
    let end = hay.len().min(limit);
    hay[..end].windows(needle.len()).position(|w| w == needle)
}

/// `true` on TIFF magic + a `NIKON` banner within the first page.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    if b.len() < 16 {
        return false;
    }
    let tiff = &b[0..4] == b"II*\x00" || &b[0..4] == b"MM\x00*";
    tiff && find(b, b"NIKON", 4096).is_some()
}

/// Census; `None` on non-NEF.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Nef> {
    if !detect(b) {
        return None;
    }
    let big_endian = b[0] == b'M';
    let ifd0 = rd32(b, 4, big_endian);
    let mut entries = 0;
    let mut make_tag = false;
    let mut maker_tag = false;
    if let Some(o) = usize::try_from(ifd0).ok().filter(|&o| o + 2 <= b.len()) {
        entries = rd16(b, o, big_endian);
        let n = usize::from(entries).min(512);
        for i in 0..n {
            let e = o + 2 + i * 12;
            if e + 2 > b.len() {
                break;
            }
            match rd16(b, e, big_endian) {
                0x010F => make_tag = true,
                0x927C => maker_tag = true,
                _ => {}
            }
        }
    }
    Some(Nef {
        big_endian,
        ifd0,
        entries,
        make_offset: find(b, b"NIKON", 4096)?,
        make_tag,
        maker_tag,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = b"II*\x00\x08\x00\x00\x00".to_vec();
        d.extend([2, 0, 0x0F, 0x01, 2, 0, 20, 0, 0, 0, 0x20, 0, 0, 0]);
        d.extend([0x7C, 0x92, 1, 0, 4, 0, 0, 0, 0, 0, 0, 0]);
        d.extend([0, 0, 0, 0]);
        d.resize(0x20, 0);
        d.extend_from_slice(b"NIKON CORPORATION\x00");
        d
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        assert!(!detect(b"II*\x00\x08\x00\x00\x00no brand here at all.."));
    }

    #[test]
    fn parses() {
        let n = parse(&fixture()).unwrap();
        assert!(!n.big_endian);
        assert_eq!(n.ifd0, 8);
        assert_eq!(n.entries, 2);
        assert_eq!(n.make_offset, 0x20);
        assert!(n.make_tag);
        assert!(n.maker_tag);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"II*\x00\x08\x00\x00\x00IIRO\x08\x00\x00\x00").is_none());
    }
    #[test]
    fn big_endian() {
        let mut d = b"MM\x00*\x00\x00\x00\x08".to_vec();
        d.resize(0x30, 0);
        d.extend_from_slice(b"NIKON");
        assert!(parse(&d).unwrap().big_endian);
    }
}
