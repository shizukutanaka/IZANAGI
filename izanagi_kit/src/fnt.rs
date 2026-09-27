//! Windows bitmap `.fnt` (FONT resource) header parsing.
//!
//! The WINFNTHEADER is little-endian: `dfVersion`(u16 @0, 0x200 or
//! 0x300), `dfSize`(u32 @2), `dfCopyright`(60B @6), then fixed metrics —
//! `dfType`/`dfPoints`/`dfVertRes`/`dfHorizRes`/`dfAscent`/`dfIntLeading`/
//! `dfExtLeading`/`dfItalic`/`dfUnderline`/`dfStrikeOut`/`dfWeight`/
//! `dfCharSet`/`dfPixWidth`/`dfPixHeight`/`dfPitchAndFamily`, then
//! `dfAvgWidth`/`dfMaxWidth`, `dfFirstChar`/`dfLastChar`/`dfDefaultChar`/
//! `dfBreakChar`, `dfWidthBytes`(u32 @99), `dfDevice`/`dfFace`(u32),
//! `dfBitsPointer`/`dfBitsOffset`(u32 @115). Version ≥3.00 adds flags
//! and spacing extensions beyond that.
//!
//! ```
//! use izanagi_kit::fnt;
//! let mut d = vec![0u8; 119];
//! d[0] = 0x00; d[1] = 0x03; // version 3.00 LE
//! d[2..6].copy_from_slice(&119u32.to_le_bytes());
//! d[95] = 32; d[96] = 126; // first/last char
//! let f = fnt::parse(&d).unwrap();
//! assert_eq!(f.version, 0x300);
//! assert_eq!(f.last_char, 126);
//! ```

use std::vec::Vec;

/// A parsed `.fnt` header.
#[derive(Clone, Debug, PartialEq)]
pub struct Fnt {
    /// `dfVersion` (0x200 = Windows 2.x, 0x300 = 3.x).
    pub version: u16,
    /// `dfSize` — declared file size in bytes.
    pub size: u32,
    /// `dfCopyright` notice (NUL-padded 60 bytes, trimmed).
    pub copyright: Vec<u8>,
    /// `dfType` bitfield (bit0 = vector, bit1 = ROM, bit2 = device-resident...).
    pub df_type: u16,
    /// `dfPoints` nominal point size.
    pub points: u16,
    /// Vertical/horizontal device resolution.
    pub vert_res: u16,
    /// Horizontal device resolution.
    pub horiz_res: u16,
    /// `dfPixWidth`/`dfPixHeight` character cell.
    pub pix_width: u16,
    /// Pixel height of a character cell.
    pub pix_height: u16,
    /// First code point present.
    pub first_char: u8,
    /// Last code point present.
    pub last_char: u8,
    /// Byte offset where the glyph table starts (data offset).
    pub bits_offset: u32,
}

fn u16le(d: &[u8], at: usize) -> Option<u16> {
    Some(*d.get(at)? as u16 | (*d.get(at + 1)? as u16) << 8)
}

fn u32le(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at.checked_add(4)?)?;
    Some(s[0] as u32 | (s[1] as u32) << 8 | (s[2] as u32) << 16 | (s[3] as u32) << 24)
}

/// Parses the header; requires ≥119 bytes, version 0x200/0x300, and a
/// `dfSize` not exceeding the input length.
pub fn parse(d: &[u8]) -> Option<Fnt> {
    if d.len() < 119 {
        return None;
    }
    let version = u16le(d, 0)?;
    if version != 0x200 && version != 0x300 {
        return None;
    }
    let size = u32le(d, 2)?;
    if size as usize > d.len() {
        return None;
    }
    let mut copyright: Vec<u8> = d[6..66].to_vec();
    while copyright.last() == Some(&0) {
        copyright.pop();
    }
    Some(Fnt {
        version,
        size,
        copyright,
        df_type: u16le(d, 66)?,
        points: u16le(d, 68)?,
        vert_res: u16le(d, 70)?,
        horiz_res: u16le(d, 72)?,
        pix_width: u16le(d, 86)?,
        pix_height: u16le(d, 88)?,
        first_char: *d.get(95)?,
        last_char: *d.get(96)?,
        bits_offset: u32le(d, 115)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hdr() -> Vec<u8> {
        let mut d = vec![0u8; 200];
        d[0] = 0x00;
        d[1] = 0x03;
        d[2..6].copy_from_slice(&150u32.to_le_bytes());
        d[6..9].copy_from_slice(b"(c)");
        d[86..88].copy_from_slice(&8u16.to_le_bytes());
        d[88..90].copy_from_slice(&16u16.to_le_bytes());
        d[95] = 32;
        d[96] = 255;
        d
    }

    #[test]
    fn header_fields() {
        let f = parse(&hdr()).unwrap();
        assert_eq!(f.version, 0x300);
        assert_eq!(f.size, 150);
        assert_eq!(f.copyright, b"(c)".to_vec());
        assert_eq!(f.pix_height, 16);
        assert_eq!(f.first_char, 32);
        assert_eq!(f.last_char, 255);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 50]).is_none());
        let mut d = hdr();
        d[1] = 0x04; // bogus version
        assert!(parse(&d).is_none());
        let mut d = hdr();
        d[2..6].copy_from_slice(&999u32.to_le_bytes()); // size > input
        assert!(parse(&d).is_none());
    }
}
