//! Sony ARW (Alpha RAW) header census.
//!
//! ARW is little-endian TIFF (`II*\x00`, IFD0 at 8); the Sony identity is
//! the `Make` value `SONY` near the head. Census walks IFD0 tags for
//! `Make` (0x010F), `Model` (0x0110), `ExifIFD` (0x8769) and
//! `DNGPrivateData`-style Sony makernote (0x927C).
//!
//! ```
//! let mut d = b"II*\x00\x08\x00\x00\x00".to_vec();
//! d.extend([1, 0, 0x0F, 0x01, 2, 0, 5, 0, 0, 0, 0x20, 0, 0, 0, 0, 0, 0, 0]);
//! d.resize(0x20, 0);
//! d.extend_from_slice(b"SONY\x00");
//! let a = izanagi_kit::arw::parse(&d).unwrap();
//! assert_eq!(a.entries, 1);
//! assert_eq!(a.make_offset, 0x20);
//! ```

/// Census fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Arw {
    /// IFD0 offset from the TIFF prologue (usually 8).
    pub ifd0: u32,
    /// IFD0 entry count when in bounds.
    pub entries: u16,
    /// File offset of the `SONY` banner.
    pub make_offset: usize,
    /// `Make` (0x010F) tag seen.
    pub make_tag: bool,
    /// `Model` (0x0110) tag seen.
    pub model_tag: bool,
    /// `ExifIFD` (0x8769) tag seen.
    pub exif_tag: bool,
    /// `MakerNote` (0x927C) tag seen.
    pub maker_tag: bool,
}

fn u16le(b: &[u8], i: usize) -> u16 {
    (b[i] as u16) | ((b[i + 1] as u16) << 8)
}

fn u32le(b: &[u8], i: usize) -> u32 {
    (b[i] as u32) | ((b[i + 1] as u32) << 8) | ((b[i + 2] as u32) << 16) | ((b[i + 3] as u32) << 24)
}

fn find(hay: &[u8], needle: &[u8], limit: usize) -> Option<usize> {
    let end = hay.len().min(limit);
    hay[..end].windows(needle.len()).position(|w| w == needle)
}

/// `true` on `II*\x00` + a `SONY` banner within the first page.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 16 && &b[0..4] == b"II*\x00" && find(b, b"SONY", 4096).is_some()
}

/// Census; `None` on non-ARW.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Arw> {
    if !detect(b) {
        return None;
    }
    let ifd0 = u32le(b, 4);
    let mut entries = 0;
    let (mut make_tag, mut model_tag, mut exif_tag, mut maker_tag) = (false, false, false, false);
    if let Some(o) = usize::try_from(ifd0).ok().filter(|&o| o + 2 <= b.len()) {
        entries = u16le(b, o);
        for i in 0..usize::from(entries).min(512) {
            let e = o + 2 + i * 12;
            if e + 2 > b.len() {
                break;
            }
            match u16le(b, e) {
                0x010F => make_tag = true,
                0x0110 => model_tag = true,
                0x8769 => exif_tag = true,
                0x927C => maker_tag = true,
                _ => {}
            }
        }
    }
    Some(Arw {
        ifd0,
        entries,
        make_offset: find(b, b"SONY", 4096)?,
        make_tag,
        model_tag,
        exif_tag,
        maker_tag,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = b"II*\x00\x08\x00\x00\x00".to_vec();
        d.extend([2, 0, 0x0F, 0x01, 2, 0, 5, 0, 0, 0, 0x20, 0, 0, 0]);
        d.extend([0x10, 0x01, 2, 0, 8, 0, 0, 0, 0x30, 0, 0, 0]);
        d.extend([0, 0, 0, 0]);
        d.resize(0x20, 0);
        d.extend_from_slice(b"SONY\x00");
        d.resize(0x30, 0);
        d.extend_from_slice(b"DSC-RX100");
        d
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        assert!(!detect(b"II*\x00\x08\x00\x00\x00NIKON........."));
    }

    #[test]
    fn parses() {
        let a = parse(&fixture()).unwrap();
        assert_eq!(a.entries, 2);
        assert_eq!(a.make_offset, 0x20);
        assert!(a.make_tag && a.model_tag);
        assert!(!a.exif_tag);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"MM\x00*\x00\x00\x00\x08SONY........").is_none());
    }
}
