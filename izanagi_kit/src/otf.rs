//! OpenType/CFF (`.otf`) specialization of the sfnt container.
//!
//! Reuses [`crate::ttf`] for the offset table + directory and additionally
//! requires the `OTTO`/`typ1` flavour and a `CFF ` table, whose Compact
//! Font Format header (`major`, `minor`, `hdrSize`, `offSize`) and Name
//! INDEX count are decoded.
//!
//! ```
//! use izanagi_kit::otf;
//! let mut d = b"OTTO".to_vec();
//! d.extend_from_slice(&1u16.to_be_bytes()); // numTables
//! d.extend_from_slice(&[0; 6]);             // searchRange etc.
//! d.extend_from_slice(b"CFF ");
//! d.extend_from_slice(&0u32.to_be_bytes()); // checksum
//! d.extend_from_slice(&28u32.to_be_bytes()); // offset
//! d.extend_from_slice(&6u32.to_be_bytes()); // length
//! d.extend_from_slice(&[1, 0, 4, 1, 0, 1]); // CFF header + Name INDEX count 1
//! let o = otf::parse(&d).unwrap();
//! assert_eq!(o.cff.major, 1);
//! ```

use crate::ttf::{self, Ttf};
use std::vec::Vec;

/// The CFF table's own header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cff {
    /// Format major version (1 or 2).
    pub major: u8,
    /// Format minor version.
    pub minor: u8,
    /// Header size in bytes (start of Name INDEX).
    pub hdr_size: u8,
    /// Absolute `offSize` declared for header data.
    pub off_size: u8,
    /// Entries in the Name INDEX (font count).
    pub names: u16,
}

/// A parsed `.otf`.
#[derive(Clone, Debug)]
pub struct Otf {
    /// The underlying sfnt directory.
    pub sfnt: Ttf,
    /// Decoded CFF header.
    pub cff: Cff,
}

fn u16be(d: &[u8], at: usize) -> Option<u16> {
    Some((*d.get(at)? as u16) << 8 | *d.get(at + 1)? as u16)
}

/// Parses an OpenType font: sfnt flavour must be `OTTO` or `typ1` and a
/// `CFF ` table with a ≥4-byte header and a Name INDEX must fit.
pub fn parse(d: &[u8]) -> Option<Otf> {
    let sfnt = ttf::parse(d)?;
    // OTTO = CFF outlines; typ1 = CFF-flavoured Type 1 wrapper.
    if sfnt.sfnt != 0x4F54_544F && sfnt.sfnt != 0x7479_7031 {
        return None;
    }
    let tab = ttf::table(&sfnt, b"CFF ")?;
    let base = tab.offset;
    if base.checked_add(4)? > d.len() || tab.length < 4 {
        return None;
    }
    let major = *d.get(base)?;
    let minor = *d.get(base + 1)?;
    let hdr_size = *d.get(base + 2)? as usize;
    let off_size = *d.get(base + 3)?;
    if !(1..=4).contains(&off_size) || hdr_size < 4 {
        return None;
    }
    let ni = base.checked_add(hdr_size)?;
    let names = u16be(d, ni)?;
    Some(Otf {
        sfnt,
        cff: Cff {
            major,
            minor,
            hdr_size: hdr_size as u8,
            off_size,
            names,
        },
    })
}

/// Names of all `CFF ` fonts (from the Name INDEX), when readable.
pub fn font_names(d: &[u8], o: &Otf) -> Option<Vec<Vec<u8>>> {
    let tab = ttf::table(&o.sfnt, b"CFF ")?;
    let mut at = tab.offset.checked_add(o.cff.hdr_size as usize)?;
    let count = u16be(d, at)? as usize;
    at += 2;
    if count == 0 {
        return Some(Vec::new());
    }
    let off_size = *d.get(at)? as usize;
    if !(1..=4).contains(&off_size) {
        return None;
    }
    let offs_at = at + 1;
    let data_base = offs_at.checked_add(count.checked_add(1)?.checked_mul(off_size)?)?;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let start = read_off(d, offs_at + i * off_size, off_size)?;
        let end = read_off(d, offs_at + (i + 1) * off_size, off_size)?;
        if end < start {
            return None;
        }
        let a = data_base.checked_add(start - 1)?;
        let b = data_base.checked_add(end - 1)?;
        out.push(d.get(a..b)?.to_vec());
    }
    Some(out)
}

fn read_off(d: &[u8], at: usize, n: usize) -> Option<usize> {
    let mut v = 0usize;
    for i in 0..n {
        v = v.checked_mul(256)?.checked_add(*d.get(at + i)? as usize)?;
    }
    Some(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn otf(name: &[u8]) -> Vec<u8> {
        // CFF: hdr 4B, Name INDEX: count=1, offSize=1, offsets [1, len+1], data
        let mut cff = vec![1u8, 0, 4, 1];
        cff.extend_from_slice(&1u16.to_be_bytes());
        cff.push(1); // offSize
        cff.push(1);
        cff.push((name.len() + 1) as u8);
        cff.extend_from_slice(name);
        let mut d = b"OTTO".to_vec();
        d.extend_from_slice(&1u16.to_be_bytes());
        d.extend_from_slice(&[0; 6]);
        d.extend_from_slice(b"CFF ");
        d.extend_from_slice(&0u32.to_be_bytes());
        d.extend_from_slice(&28u32.to_be_bytes());
        d.extend_from_slice(&(cff.len() as u32).to_be_bytes());
        d.extend_from_slice(&cff);
        d
    }

    #[test]
    fn parses_otto() {
        let d = otf(b"MyFont");
        let o = parse(&d).unwrap();
        assert_eq!(o.cff.names, 1);
        assert_eq!(font_names(&d, &o), Some(vec![b"MyFont".to_vec()]));
    }

    #[test]
    fn rejects_truetype_flavour() {
        let mut d = otf(b"X");
        d[0..4].copy_from_slice(&1u32.to_be_bytes()); // 0x00010000
        assert!(parse(&d).is_none());
        assert!(parse(b"OTTO").is_none());
    }
}
