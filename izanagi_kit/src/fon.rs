//! `.fon` font-library container: an NE (New Executable) whose resource
//! table holds `RT_FONT` (0x8008) bitmap-font resources and usually a
//! `RT_FONTDIR` (0x8007) directory resource.
//!
//! The NE resource table (at `ne.at + ne.resource_table`) is a
//! `u16 align_shift` followed by type records `{type_id, count, reserved}`
//! each with `count` × 12-byte `{offset, length, flags, id, reserved}`
//! resource entries; offsets/lengths are in `1 << align_shift` sectors.
//!
//! ```
//! use izanagi_kit::fon;
//! // MZ + NE with one RT_FONT resource at sector offset 4, len 3.
//! let mut d = vec![0u8; 512];
//! d[0] = b'M'; d[1] = b'Z'; d[0x3c..0x40].copy_from_slice(&64u32.to_le_bytes());
//! d[64..66].copy_from_slice(b"NE");
//! d[64 + 0x24..64 + 0x26].copy_from_slice(&0x40u16.to_le_bytes()); // rsrc tbl
//! let t = 64 + 0x40;
//! d[t..t + 2].copy_from_slice(&4u16.to_le_bytes()); // align shift
//! d[t + 2..t + 4].copy_from_slice(&0x8008u16.to_le_bytes()); // RT_FONT
//! d[t + 4..t + 6].copy_from_slice(&1u16.to_le_bytes()); // count
//! d[t + 10..t + 12].copy_from_slice(&4u16.to_le_bytes()); // offset (sectors)
//! d[t + 12..t + 14].copy_from_slice(&3u16.to_le_bytes()); // length (sectors)
//! d[t + 22..t + 24].copy_from_slice(&0u16.to_le_bytes()); // terminator
//! let f = fon::parse(&d).unwrap();
//! assert_eq!(f.fonts.len(), 1);
//! assert_eq!(f.fonts[0].offset, 4 << 4);
//! ```

use crate::ne;
use std::vec::Vec;

/// NE resource-type id.
pub const RT_FONT: u16 = 0x8008;
/// NE resource-type id for the font directory resource.
pub const RT_FONTDIR: u16 = 0x8007;

/// One `RT_FONT` resource (file offset + size already sector-expanded).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Font {
    /// Absolute file offset of the FNT data.
    pub offset: usize,
    /// Resource length in bytes.
    pub length: usize,
    /// Resource id (`rID`), or the name-table flag high bit kept verbatim.
    pub id: u16,
    /// Resource flags (`rFlags`: movable/discordant bit set).
    pub flags: u16,
}

/// A parsed `.fon` file.
#[derive(Clone, Debug, PartialEq)]
pub struct Fon {
    /// All `RT_FONT` resources in table order.
    pub fonts: Vec<Font>,
    /// `(offset, length)` of the `RT_FONTDIR` resource, if present.
    pub fontdir: Option<(usize, usize)>,
    /// The resource-table alignment shift actually used.
    pub sector_shift: u16,
}

fn u16le(d: &[u8], at: usize) -> Option<u16> {
    Some(*d.get(at)? as u16 | (*d.get(at + 1)? as u16) << 8)
}

/// Parses `MZ` → `NE` → resource table; returns `None` when the NE
/// header or resource table is absent/truncated or no font records fit.
pub fn parse(d: &[u8]) -> Option<Fon> {
    let ne = ne::parse(d)?;
    let mut at = ne.at.checked_add(ne.resource_table as usize)?;
    let shift = u16le(d, at)? as u32;
    if shift > 30 {
        return None;
    }
    at += 2;
    let mut fonts = Vec::new();
    let mut fontdir = None;
    loop {
        let ty = u16le(d, at)?;
        if ty == 0 {
            break; // end of type list
        }
        let count = u16le(d, at + 2)? as usize;
        // entries follow the 8-byte type record
        let mut e = at.checked_add(8)?;
        for _ in 0..count {
            let r_off = u16le(d, e)? as usize;
            let r_len = u16le(d, e + 2)? as usize;
            let r_flags = u16le(d, e + 4)?;
            let r_id = u16le(d, e + 6)?;
            let offset = r_off.checked_shl(shift)?;
            let length = r_len.checked_shl(shift)?;
            if ty == RT_FONT {
                if offset.checked_add(length)? > d.len() {
                    return None;
                }
                fonts.push(Font {
                    offset,
                    length,
                    id: r_id,
                    flags: r_flags,
                });
            } else if ty == RT_FONTDIR {
                if offset.checked_add(length)? > d.len() {
                    return None;
                }
                fontdir = Some((offset, length));
            }
            e = e.checked_add(12)?;
        }
        at = e;
    }
    Some(Fon {
        fonts,
        fontdir,
        sector_shift: shift as u16,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fon_with(types: &[(u16, u16, u16, u16)]) -> Vec<u8> {
        // types: (type_id, count, r_off, r_len) single-entry records
        let mut d = vec![0u8; 4096];
        d[0] = b'M';
        d[1] = b'Z';
        d[0x3c..0x40].copy_from_slice(&64u32.to_le_bytes());
        d[64..66].copy_from_slice(b"NE");
        d[64 + 0x24..64 + 0x26].copy_from_slice(&0x40u16.to_le_bytes());
        let t = 64 + 0x40;
        d[t..t + 2].copy_from_slice(&4u16.to_le_bytes());
        let mut e = t + 2;
        for &(ty, count, roff, rlen) in types {
            d[e..e + 2].copy_from_slice(&ty.to_le_bytes());
            d[e + 2..e + 4].copy_from_slice(&count.to_le_bytes());
            e += 8;
            for _ in 0..count {
                d[e..e + 2].copy_from_slice(&roff.to_le_bytes());
                d[e + 2..e + 4].copy_from_slice(&rlen.to_le_bytes());
                e += 12;
            }
        }
        d[e..e + 2].copy_from_slice(&0u16.to_le_bytes());
        d
    }

    #[test]
    fn font_resources() {
        let d = fon_with(&[(RT_FONT, 2, 4, 3), (RT_FONTDIR, 1, 8, 2)]);
        // second RT_FONT entry was written with the same roff — fine for layout
        let f = parse(&d).unwrap();
        assert_eq!(f.fonts.len(), 2);
        assert_eq!(f.fonts[0].offset, 64);
        assert_eq!(f.fonts[0].length, 48);
        assert_eq!(f.fontdir, Some((128, 32)));
        assert_eq!(f.sector_shift, 4);
    }

    #[test]
    fn rejects_non_ne_and_bounds() {
        assert!(parse(b"MZ").is_none());
        // resource pointing past EOF
        let d = fon_with(&[(RT_FONT, 1, 200, 100)]);
        assert!(parse(&d).is_none());
    }
}
