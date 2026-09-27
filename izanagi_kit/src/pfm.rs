//! Printer Font Metrics (`.pfm`) header parsing (Microsoft PFM spec).
//!
//! Little-endian `PFMHEADER`: `dfSize`(u16 @0, ≥117), `szCopyright`
//! (60B @2), `dfVersion`/`dfPoints`/`dfVertRes`/`dfHorizRes`/`dfAscent`/
//! `dfIntLeading`/`dfExtLeading`(@62..75), flags @76..79,
//! `dfWeight`(u16 @79), `dfCharSet` @81, `dfPixWidth`/`dfPixHeight`
//! (@82/@84), `dfPitchAndFamily` @86, `dfAvgWidth`/`dfMaxWidth`(@87/@89),
//! first/last/default/break char @91..95, `dfWidthBytes`@95, then
//! `dfDevice`/`dfFace`/`dfBitsPointer`/`dfBitsOffset`(u32 @97..113).
//!
//! ```
//! use izanagi_kit::pfm;
//! let mut d = vec![0u8; 117];
//! d[0] = 117; d[1] = 0; // dfSize
//! d[62] = 0x00; d[63] = 0x01; // version 1.0 = 0x0100
//! d[91] = 32; d[92] = 127;
//! let p = pfm::parse(&d).unwrap();
//! assert_eq!(p.last_char, 127);
//! ```

use std::vec::Vec;

/// A parsed PFM header.
#[derive(Clone, Debug, PartialEq)]
pub struct Pfm {
    /// `dfSize` — declared size (≤ input length).
    pub size: u16,
    /// `szCopyright` (NUL-trimmed 60 bytes).
    pub copyright: Vec<u8>,
    /// `dfVersion` (usually 0x0100).
    pub version: u16,
    /// `dfPoints`.
    pub points: u16,
    /// Device vertical resolution.
    pub vert_res: u16,
    /// Device horizontal resolution.
    pub horiz_res: u16,
    /// `dfAscent`.
    pub ascent: u16,
    /// `dfWeight` (400 = normal).
    pub weight: u16,
    /// `dfCharSet`.
    pub char_set: u8,
    /// `dfPixWidth`/`dfPixHeight` (0 = scalable).
    pub pix_width: u16,
    /// Pixel height (0 = scalable).
    pub pix_height: u16,
    /// `dfPitchAndFamily` byte.
    pub pitch_and_family: u8,
    /// `dfFirstChar`.
    pub first_char: u8,
    /// `dfLastChar`.
    pub last_char: u8,
    /// `dfDevice` name-offset (0 = none).
    pub device_name: u32,
    /// `dfFace` name-offset (0 = none).
    pub face_name: u32,
}

fn u16le(d: &[u8], at: usize) -> Option<u16> {
    Some(*d.get(at)? as u16 | (*d.get(at + 1)? as u16) << 8)
}

fn u32le(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at.checked_add(4)?)?;
    Some(s[0] as u32 | (s[1] as u32) << 8 | (s[2] as u32) << 16 | (s[3] as u32) << 24)
}

/// Parses the 117-byte header; `dfSize` must fit the input.
pub fn parse(d: &[u8]) -> Option<Pfm> {
    if d.len() < 117 {
        return None;
    }
    let size = u16le(d, 0)?;
    if size < 117 || size as usize > d.len() {
        return None;
    }
    let mut copyright: Vec<u8> = d[2..62].to_vec();
    while copyright.last() == Some(&0) {
        copyright.pop();
    }
    Some(Pfm {
        size,
        copyright,
        version: u16le(d, 62)?,
        points: u16le(d, 64)?,
        vert_res: u16le(d, 66)?,
        horiz_res: u16le(d, 68)?,
        ascent: u16le(d, 70)?,
        weight: u16le(d, 79)?,
        char_set: *d.get(81)?,
        pix_width: u16le(d, 82)?,
        pix_height: u16le(d, 84)?,
        pitch_and_family: *d.get(86)?,
        first_char: *d.get(91)?,
        last_char: *d.get(92)?,
        device_name: u32le(d, 97)?,
        face_name: u32le(d, 101)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hdr() -> Vec<u8> {
        let mut d = vec![0u8; 117];
        d[0] = 117;
        d[2..5].copy_from_slice(b"(c)");
        d[62] = 0x00;
        d[63] = 0x01;
        d[79..81].copy_from_slice(&400u16.to_le_bytes());
        d[91] = 32;
        d[92] = 255;
        d
    }

    #[test]
    fn header_fields() {
        let p = parse(&hdr()).unwrap();
        assert_eq!(p.size, 117);
        assert_eq!(p.copyright, b"(c)".to_vec());
        assert_eq!(p.version, 0x0100);
        assert_eq!(p.weight, 400);
        assert_eq!(p.first_char, 32);
        assert_eq!(p.last_char, 255);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 60]).is_none());
        let mut d = hdr();
        d[0] = 50; // dfSize below minimum
        assert!(parse(&d).is_none());
    }
}
