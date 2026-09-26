//! PSF — the PC Screen Font format used by the Linux console
//! (`kbd`/`console-setup`). PSF1 is a 4-byte header (`0x36 0x04`,
//! mode, glyph bytes) over 8-wide bitmap glyphs; PSF2 is a 32-byte
//! header (`0x864AB572` LE) with explicit `length`/`charsize`/
//! `height`/`width` fields. Both can carry a per-glyph Unicode
//! mapping table after the bitmaps: UTF-16LE units, `0xFFFF`
//! terminating each glyph's list, `0xFFFE` separating glyphs.
//!
//! ```
//! use izanagi_kit::psf::{parse, Version};
//!
//! // PSF1, 256 glyphs of 8 bytes, no unicode table.
//! let mut d = vec![0x36, 0x04, 0x00, 0x08];
//! d.extend(std::iter::repeat(0xFFu8).take(256 * 8));
//! let f = parse(&d).unwrap();
//! assert_eq!(f.version(), Version::Psf1);
//! assert_eq!(f.glyphs, 256);
//! assert_eq!(f.glyph(&d, 0).unwrap().len(), 8);
//! ```

use std::vec::Vec;

/// PSF1 magic byte pair.
pub const PSF1_MAGIC: [u8; 2] = [0x36, 0x04];
/// PSF2 magic (little-endian u32).
pub const PSF2_MAGIC: u32 = 0x864A_B572;
/// PSF1 mode bit: 512 glyphs instead of 256.
pub const PSF1_512: u8 = 0x01;
/// PSF1 mode bit: unicode table follows the bitmaps.
pub const PSF1_UNICODE: u8 = 0x02;

/// Which header the file has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Version {
    /// 4-byte header, 8-pixel-wide glyphs.
    Psf1,
    /// 32-byte header, arbitrary geometry.
    Psf2,
}

/// A parsed PSF header.
#[derive(Clone, Debug, PartialEq)]
pub struct Psf {
    /// File variant.
    pub version: Version,
    /// Glyph count (256 or 512 for PSF1).
    pub glyphs: usize,
    /// Bytes per glyph bitmap.
    pub charsize: usize,
    /// Glyph height in pixels.
    pub height: usize,
    /// Glyph width in pixels (always 8 for PSF1).
    pub width: usize,
    /// Byte offset of the bitmap area.
    pub bitmaps_at: usize,
    /// Byte offset of the Unicode table, `0` when absent.
    pub table_at: usize,
}

fn u32le(d: &[u8], at: usize) -> u32 {
    u32::from(d[at])
        | u32::from(d[at + 1]) << 8
        | u32::from(d[at + 2]) << 16
        | u32::from(d[at + 3]) << 24
}

/// Parse the header and locate the bitmap/table areas.
pub fn parse(d: &[u8]) -> Option<Psf> {
    if d.len() >= 4 && d[..2] == PSF1_MAGIC {
        let mode = d[2];
        let charsize = usize::from(d[3]);
        let glyphs = if mode & PSF1_512 != 0 { 512 } else { 256 };
        let bitmaps_at = 4;
        let table_at = bitmaps_at + glyphs * charsize;
        if charsize == 0 || d.len() < table_at {
            return None;
        }
        return Some(Psf {
            version: Version::Psf1,
            glyphs,
            charsize,
            height: charsize,
            width: 8,
            bitmaps_at,
            table_at: if mode & PSF1_UNICODE != 0 {
                table_at
            } else {
                0
            },
        });
    }
    if d.len() < 32 || u32le(d, 0) != PSF2_MAGIC {
        return None;
    }
    let headersize = u32le(d, 8) as usize;
    let flags = u32le(d, 12);
    let glyphs = u32le(d, 16) as usize;
    let charsize = u32le(d, 20) as usize;
    let height = u32le(d, 24) as usize;
    let width = u32le(d, 28) as usize;
    if headersize < 32 || charsize == 0 || height == 0 || width == 0 {
        return None;
    }
    let bitmaps_at = headersize;
    let end = bitmaps_at.checked_add(glyphs.checked_mul(charsize)?)?;
    if end > d.len() {
        return None;
    }
    Some(Psf {
        version: Version::Psf2,
        glyphs,
        charsize,
        height,
        width,
        bitmaps_at,
        table_at: if flags & 1 != 0 { end } else { 0 },
    })
}

impl Psf {
    /// Which header the file has.
    pub fn version(&self) -> Version {
        self.version
    }

    /// Glyph `i`'s bitmap bytes.
    pub fn glyph<'a>(&self, d: &'a [u8], i: usize) -> Option<&'a [u8]> {
        if i >= self.glyphs {
            return None;
        }
        let at = self.bitmaps_at + i * self.charsize;
        d.get(at..at + self.charsize)
    }

    /// The Unicode mappings for glyph `i` (each codepoint listed).
    /// `None` when the file has no table.
    pub fn unicode(&self, d: &[u8], i: usize) -> Option<Vec<char>> {
        if self.table_at == 0 || i >= self.glyphs {
            return None;
        }
        // Walk the table: glyph lists are separated by 0xFFFE, each
        // glyph's codepoints end at 0xFFFF.
        let mut at = self.table_at;
        let mut glyph = 0;
        let mut out = Vec::new();
        while at + 1 < d.len() {
            let u = u16::from(d[at]) | u16::from(d[at + 1]) << 8;
            at += 2;
            match u {
                0xFFFE => {
                    glyph += 1; // sequence-of-glyphs separator
                }
                0xFFFF => {
                    if glyph == i {
                        return Some(out);
                    }
                    glyph += 1;
                    out = Vec::new();
                }
                _ => {
                    if glyph == i {
                        if let Some(c) = char::from_u32(u32::from(u)) {
                            out.push(c);
                        }
                    }
                }
            }
        }
        if glyph == i {
            Some(out)
        } else {
            Some(Vec::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn psf1_with_table() -> Vec<u8> {
        let mut d = vec![0x36, 0x04, PSF1_UNICODE, 0x08];
        d.extend(std::iter::repeat(0u8).take(256 * 8));
        // glyph 0 → 'A' ; glyph 1 → 'B' and 'b'
        d.extend_from_slice(&u16::to_le_bytes(b'A' as u16));
        d.extend_from_slice(&0xFFFFu16.to_le_bytes());
        d.extend_from_slice(&u16::to_le_bytes(b'B' as u16));
        d.extend_from_slice(&u16::to_le_bytes(b'b' as u16));
        d.extend_from_slice(&0xFFFFu16.to_le_bytes());
        // remaining glyphs: empty lists
        for _ in 2..256 {
            d.extend_from_slice(&0xFFFFu16.to_le_bytes());
        }
        d
    }

    #[test]
    fn psf1_variants() {
        let mut d = vec![0x36, 0x04, 0x00, 0x08];
        d.extend(std::iter::repeat(0xFFu8).take(256 * 8));
        let f = parse(&d).unwrap();
        assert_eq!(f.version(), Version::Psf1);
        assert_eq!((f.glyphs, f.charsize, f.width, f.height), (256, 8, 8, 8));
        assert_eq!(f.glyph(&d, 255).unwrap(), &[0xFF; 8]);
        assert!(f.glyph(&d, 256).is_none());
        assert!(f.unicode(&d, 0).is_none()); // no table flag

        let mut d = vec![0x36, 0x04, PSF1_512, 0x04];
        d.extend(std::iter::repeat(0u8).take(512 * 4));
        let f = parse(&d).unwrap();
        assert_eq!((f.glyphs, f.height), (512, 4));
    }

    #[test]
    fn psf1_unicode_table() {
        let d = psf1_with_table();
        let f = parse(&d).unwrap();
        assert!(f.table_at > 0);
        assert_eq!(f.unicode(&d, 0).unwrap(), vec!['A']);
        assert_eq!(f.unicode(&d, 1).unwrap(), vec!['B', 'b']);
        assert_eq!(f.unicode(&d, 5).unwrap(), Vec::<char>::new());
    }

    #[test]
    fn psf2_header() {
        let mut d = Vec::new();
        let vals = [PSF2_MAGIC, 0, 32, 1, 64, 16, 8, 9];
        for v in vals {
            d.extend_from_slice(&v.to_le_bytes());
        }
        d.extend(std::iter::repeat(0xAAu8).take(64 * 16));
        // unicode table: glyph 0 → 'Q'
        d.extend_from_slice(&u16::to_le_bytes(b'Q' as u16));
        d.extend_from_slice(&0xFFFFu16.to_le_bytes());
        let f = parse(&d).unwrap();
        assert_eq!(f.version(), Version::Psf2);
        assert_eq!((f.glyphs, f.charsize, f.height, f.width), (64, 16, 8, 9));
        assert_eq!(f.glyph(&d, 63).unwrap().len(), 16);
        assert_eq!(f.unicode(&d, 0).unwrap(), vec!['Q']);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&[0x36, 0x04, 0x00, 0x00]).is_none()); // charsize 0
        let mut d = vec![0x36, 0x04, 0x00, 0x08];
        d.extend(std::iter::repeat(0u8).take(10));
        assert!(parse(&d).is_none()); // bitmaps truncated
        let mut bad = vec![0x72, 0xB5, 0x4A, 0x86];
        bad.extend(std::iter::repeat(0u8).take(40)); // psf2 but headersize=0
        assert!(parse(&bad).is_none());
    }
}
