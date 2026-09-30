//! Panasonic RW2 (RAW version 2) header census.
//!
//! RW2 keeps the `II` byte order mark but replaces the TIFF magic `42`
//! with `85` (`IIU\x00`); IFD0 follows the `u32le` offset and the
//! `Panasonic` brand string sits within the first page. Census reports
//! IFD0 plus `Make`/`MakerNote`-style tag presence.
//!
//! ```
//! let mut d = b"IIU\x00\x08\x00\x00\x00".to_vec();
//! d.extend([1, 0, 0x0F, 0x01, 2, 0, 10, 0, 0, 0, 0x20, 0, 0, 0, 0, 0, 0, 0]);
//! d.resize(0x20, 0);
//! d.extend_from_slice(b"Panasonic\x00");
//! let w = izanagi_kit::rw2::parse(&d).unwrap();
//! assert_eq!(w.entries, 1);
//! ```

/// Census fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rw2 {
    /// IFD0 offset from the header.
    pub ifd0: u32,
    /// IFD0 entry count when in bounds.
    pub entries: u16,
    /// File offset of the `Panasonic` banner when present.
    pub panasonic_offset: Option<usize>,
    /// `Make` (0x010F) tag seen.
    pub make_tag: bool,
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

/// `true` on the `IIU\x00` magic.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 16 && &b[0..4] == b"IIU\x00"
}

/// Census; `None` on non-RW2.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Rw2> {
    if !detect(b) {
        return None;
    }
    let ifd0 = u32le(b, 4);
    let mut entries = 0;
    let (mut make_tag, mut maker_tag) = (false, false);
    if let Some(o) = usize::try_from(ifd0).ok().filter(|&o| o + 2 <= b.len()) {
        entries = u16le(b, o);
        for i in 0..usize::from(entries).min(512) {
            let e = o + 2 + i * 12;
            if e + 2 > b.len() {
                break;
            }
            match u16le(b, e) {
                0x010F => make_tag = true,
                0x927C => maker_tag = true,
                _ => {}
            }
        }
    }
    Some(Rw2 {
        ifd0,
        entries,
        panasonic_offset: find(b, b"Panasonic", 4096),
        make_tag,
        maker_tag,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = b"IIU\x00\x08\x00\x00\x00".to_vec();
        d.extend([2, 0, 0x0F, 0x01, 2, 0, 10, 0, 0, 0, 0x20, 0, 0, 0]);
        d.extend([0x7C, 0x92, 7, 0, 4, 0, 0, 0, 0x60, 0, 0, 0]);
        d.extend([0, 0, 0, 0]);
        d.resize(0x20, 0);
        d.extend_from_slice(b"Panasonic\x00");
        d
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        assert!(!detect(b"II*\x00\x08\x00\x00\x00IIRO........"));
    }

    #[test]
    fn parses() {
        let w = parse(&fixture()).unwrap();
        assert_eq!(w.ifd0, 8);
        assert_eq!(w.entries, 2);
        assert_eq!(w.panasonic_offset, Some(0x20));
        assert!(w.make_tag && w.maker_tag);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"IIV\x00\x08\x00\x00\x00........").is_none());
    }
}
