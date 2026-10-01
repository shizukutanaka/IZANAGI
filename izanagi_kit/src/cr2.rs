//! Canon CR2 (Canon RAW version 2) header census.
//!
//! CR2 is TIFF-based: `II*\x00` little-endian prologue, but IFD0 is pushed
//! to offset 16 and bytes 8..16 carry `CR` + major/minor version + the
//! `u32le` offset of the embedded JPEG preview:
//!
//! ```text
//! 00 "II*\x00"  04 u32le ifd0=16  08 "CR"  0A major u8  0B minor u8  0C u32le jpeg_off
//! ```
//!
//! ```
//! let mut d = b"II*\x00\x10\x00\x00\x00CR\x02\x00\x00\x10\x00\x00".to_vec();
//! d.extend([0u8; 8]);
//! let c = izanagi_kit::cr2::parse(&d).unwrap();
//! assert_eq!(c.major, 2);
//! assert_eq!(c.jpeg_offset, 0x1000);
//! ```

/// Census fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cr2 {
    /// IFD0 offset (always 16 per spec).
    pub ifd0: u32,
    /// CR2 format major version byte (offset 10; `2` on CR2, `3` on CR3-era TIFF wrappers).
    pub major: u8,
    /// Minor version byte (offset 11).
    pub minor: u8,
    /// Offset of the embedded JPEG preview (offset 12, `u32le`).
    pub jpeg_offset: u32,
    /// IFD0 entry count when in bounds.
    pub entries: u16,
}

fn u16le(b: &[u8], i: usize) -> u16 {
    (b[i] as u16) | ((b[i + 1] as u16) << 8)
}

fn u32le(b: &[u8], i: usize) -> u32 {
    (b[i] as u32) | ((b[i + 1] as u32) << 8) | ((b[i + 2] as u32) << 16) | ((b[i + 3] as u32) << 24)
}

/// `true` on the `II*\x00` + `CR\x02\xNN` dual signature.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 16 && &b[0..4] == b"II*\x00" && &b[8..10] == b"CR" && b[10] == 2
}

/// Census; `None` on non-CR2.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Cr2> {
    if !detect(b) {
        return None;
    }
    let ifd0 = u32le(b, 4);
    let jpeg_offset = u32le(b, 12);
    let entries = usize::try_from(ifd0)
        .ok()
        .filter(|&o| o + 2 <= b.len())
        .map_or(0, |o| u16le(b, o));
    Some(Cr2 {
        ifd0,
        major: b[10],
        minor: b[11],
        jpeg_offset,
        entries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = b"II*\x00\x10\x00\x00\x00CR\x02\x00\x00\x10\x00\x00".to_vec();
        d.extend_from_slice(&[2, 0, 0x02, 0x01, 0, 0, 4, 0, 0, 0, 1, 0, 0, 0]);
        d
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        assert!(!detect(
            b"MM\x00*\x00\x00\x00\x08CR\x02\x00\x00\x10\x00\x00"
        ));
        assert!(!detect(b"II*\x00\x08\x00\x00\x00IIRO\x08\x00\x00\x00"));
    }

    #[test]
    fn parses() {
        let c = parse(&fixture()).unwrap();
        assert_eq!(c.ifd0, 16);
        assert_eq!(c.major, 2);
        assert_eq!(c.minor, 0);
        assert_eq!(c.jpeg_offset, 0x1000);
        assert_eq!(c.entries, 2);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"II*\x00\x10\x00\x00\x00CR\x03\x00\x00\x10\x00\x00").is_none());
    }
}
