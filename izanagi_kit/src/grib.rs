//! GRIB (GRIdded Binary) message envelope, editions 1 and 2.
//!
//! Both editions open with `GRIB`. Edition 1 carries a 3-byte total
//! length and a Product Definition Section with the reference-time
//! fields, parameter table, centre and grid ids. Edition 2 has a
//! discipline byte plus a u64 total length, followed by numbered
//! sections — `sections` walks the `(id, len)` chain.
//!
//! ```
//! use izanagi_kit::grib::{parse, Kind};
//!
//! let mut d = vec![0u8; 128];
//! d[..4].copy_from_slice(b"GRIB");
//! d[4] = 0; d[5] = 0; d[6] = 128; // total len 128 (3B)
//! d[7] = 1;                        // edition 1
//! let put16 = |d: &mut [u8], at: usize, v: u16| {
//!     d[at] = (v >> 8) as u8; d[at + 1] = v as u8;
//! };
//! // indicator section at 8
//! d[10] = 80;                     // PDS len 80? keep simple
//! d[11] = 7;                      // parameter table version
//! put16(&mut d, 12, 98);          // centre ECMWF
//! d[14] = 1;                      // process
//! d[16] = 0b11;                   // GDS+BMS present
//! let g = parse(&d).unwrap();
//! assert_eq!(g.kind, Kind::Edition1);
//! assert_eq!(g.total_len, 128);
//! ```

/// Edition detected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// GRIB edition 1.
    Edition1,
    /// GRIB edition 2.
    Edition2,
    /// Any other edition byte.
    Other(u8),
}

/// One numbered section in a GRIB2 message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Section {
    /// Section number (1 id, 2 local use, 3 grid, 4 product, 5 data repr, 6 bitmap, 7 data).
    pub num: u8,
    /// Section length in bytes.
    pub len: u32,
    /// File offset of the section.
    pub at: usize,
}

/// A parsed GRIB envelope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grib {
    /// Edition classification.
    pub kind: Kind,
    /// Declared total message length.
    pub total_len: u64,
    /// Edition-1: PDS table version, or edition-2: discipline.
    pub discipline_or_table: u8,
    /// Edition-1: originating centre id.
    pub centre: u16,
    /// Edition-2: walked sections.
    pub sections: Vec<Section>,
    /// Offset of the first section (ed2) or indicator section (ed1).
    pub body_at: usize,
}

fn be16(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from(*d.get(at)?) << 8 | u16::from(*d.get(at + 1)?))
}
fn be32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(at)?) << 24
            | u32::from(*d.get(at + 1)?) << 16
            | u32::from(*d.get(at + 2)?) << 8
            | u32::from(*d.get(at + 3)?),
    )
}
fn be64(d: &[u8], at: usize) -> Option<u64> {
    Some(
        u64::from(*d.get(at)?) << 56
            | u64::from(*d.get(at + 1)?) << 48
            | u64::from(*d.get(at + 2)?) << 40
            | u64::from(*d.get(at + 3)?) << 32
            | u64::from(*d.get(at + 4)?) << 24
            | u64::from(*d.get(at + 5)?) << 16
            | u64::from(*d.get(at + 6)?) << 8
            | u64::from(*d.get(at + 7)?),
    )
}
fn be24(d: &[u8], at: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(at)?) << 16 | u32::from(*d.get(at + 1)?) << 8 | u32::from(*d.get(at + 2)?),
    )
}

/// Parse a GRIB envelope. Returns `None` on a bad magic or a
/// truncated header.
pub fn parse(d: &[u8]) -> Option<Grib> {
    if d.get(..4)? != b"GRIB" {
        return None;
    }
    let edition = d.get(7).copied()?;
    match edition {
        1 => {
            let total_len = be24(d, 4)?;
            // PDS at 8: len u24 (8..10), table version u8 (11), centre u16 (12..13)
            Some(Grib {
                kind: Kind::Edition1,
                total_len: u64::from(total_len),
                discipline_or_table: d.get(11).copied()?,
                centre: be16(d, 12)?,
                sections: Vec::new(),
                body_at: 8,
            })
        }
        2 => {
            let total_len = be64(d, 8)?;
            let mut sections = Vec::new();
            let mut at = 16usize;
            // walk numbered sections until section 7 or EOS
            while let Some(len) = be32(d, at) {
                let num = d.get(at + 4).copied()?;
                if len < 5 {
                    return None;
                }
                sections.push(Section { num, len, at });
                let n = at.checked_add(usize::try_from(len).ok()?)?;
                if n > d.len() {
                    return None;
                }
                at = n;
                if num == 7 || at >= d.len() {
                    break;
                }
            }
            Some(Grib {
                kind: Kind::Edition2,
                total_len,
                discipline_or_table: d.get(6).copied()?,
                centre: 0,
                sections,
                body_at: 16,
            })
        }
        e => Some(Grib {
            kind: Kind::Other(e),
            total_len: u64::from(be24(d, 4)?),
            discipline_or_table: 0,
            centre: 0,
            sections: Vec::new(),
            body_at: 8,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edition1_fields() {
        let mut d = vec![0u8; 96];
        d[..4].copy_from_slice(b"GRIB");
        d[4] = 0;
        d[5] = 0;
        d[6] = 96;
        d[7] = 1;
        d[11] = 128; // table 128
        d[12] = 0;
        d[13] = 7; // centre 7
        let g = parse(&d).unwrap();
        assert_eq!(g.kind, Kind::Edition1);
        assert_eq!(g.total_len, 96);
        assert_eq!(g.discipline_or_table, 128);
        assert_eq!(g.centre, 7);
        assert_eq!(g.body_at, 8);
    }

    #[test]
    fn edition2_sections() {
        let mut d = vec![0u8; 200];
        d[..4].copy_from_slice(b"GRIB");
        d[6] = 0; // discipline: meteorological
        d[7] = 2;
        for (i, b) in (200u64).to_be_bytes().iter().enumerate() {
            d[8 + i] = *b;
        }
        // section 1 (len 21) then section 3 (len 12), end marker "7777"
        let w32 = |d: &mut [u8], o: usize, v: u32| {
            d[o] = (v >> 24) as u8;
            d[o + 1] = (v >> 16) as u8;
            d[o + 2] = (v >> 8) as u8;
            d[o + 3] = v as u8;
        };
        w32(&mut d, 16, 21);
        d[20] = 1;
        w32(&mut d, 37, 12);
        d[41] = 3;
        w32(&mut d, 49, 5);
        d[53] = 7;
        let g = parse(&d).unwrap();
        assert_eq!(g.kind, Kind::Edition2);
        assert_eq!(g.total_len, 200);
        assert_eq!(g.discipline_or_table, 0);
        assert_eq!(g.sections.len(), 3);
        assert_eq!(g.sections[0].num, 1);
        assert_eq!(g.sections[0].len, 21);
        assert_eq!(g.sections[1].num, 3);
        assert_eq!(g.sections[1].at, 37);
        assert_eq!(g.sections[2].num, 7);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 4]).is_none());
        assert!(parse(b"XYZW\0\0\0\x01").is_none());
        // edition 2 with a section length that runs past the buffer
        let mut d = vec![0u8; 20];
        d[..4].copy_from_slice(b"GRIB");
        d[7] = 2;
        d[19] = 255; // section len 255, section num read at 20 -> out of bounds
        assert!(parse(&d).is_none());
        // a declared section len < 5 is malformed
        let mut d2 = vec![0u8; 24];
        d2[..4].copy_from_slice(b"GRIB");
        d2[7] = 2;
        d2[19] = 4;
        d2[20] = 1;
        assert!(parse(&d2).is_none());
    }
}
