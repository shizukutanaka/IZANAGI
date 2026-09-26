//! MINIX filesystem superblock (v1/v2).
//!
//! The superblock occupies the second block (offset 1024) and
//! identifies the filesystem version by its magic: `0x137F` /
//! `0x138F` (v1, 14-char and 30-char names, 16-bit `s_nzones`)
//! and `0x2468` / `0x2478` (v2, 14/30-char names, 32-bit
//! `s_zones`). `parse` reports geometry, inode/zone maps and the
//! version-derived zone count.
//!
//! ```
//! use izanagi_kit::minix::{parse, SUPER_AT, Magic};
//!
//! let mut d = vec![0u8; SUPER_AT + 64];
//! let w16 = |d: &mut [u8], o: usize, v: u16| {
//!     d[SUPER_AT + o] = v as u8; d[SUPER_AT + o + 1] = (v >> 8) as u8;
//! };
//! let w32 = |d: &mut [u8], o: usize, v: u32| {
//!     for i in 0..4 { d[SUPER_AT + o + i] = (v >> (i * 8)) as u8; }
//! };
//! w16(&mut d, 0, 480);        // inodes
//! w16(&mut d, 2, 0);          // s_nzones (v1 only)
//! w16(&mut d, 4, 2);          // imap blocks
//! w16(&mut d, 6, 4);          // zmap blocks
//! w16(&mut d, 8, 33);         // first data zone
//! w32(&mut d, 20, 1440);      // v2 zones
//! w16(&mut d, 16, 0x2478);    // v2, 30-char names
//! let s = parse(&d).unwrap();
//! assert_eq!(s.magic, Magic::V2LongNames);
//! assert_eq!(s.zones(), 1440);
//! ```

/// Superblock byte offset.
pub const SUPER_AT: usize = 1024;

fn le16(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from(*d.get(at)?) | u16::from(*d.get(at + 1)?) << 8)
}
fn le32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(at)?)
            | u32::from(*d.get(at + 1)?) << 8
            | u32::from(*d.get(at + 2)?) << 16
            | u32::from(*d.get(at + 3)?) << 24,
    )
}

/// `s_magic` values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Magic {
    /// `0x137F` — v1, 14-char names, zones in `s_nzones`.
    V1,
    /// `0x138F` — v1, 30-char names.
    V1LongNames,
    /// `0x2468` — v2, 14-char names, zones in `s_zones`.
    V2,
    /// `0x2478` — v2, 30-char names.
    V2LongNames,
}

impl Magic {
    /// From the raw u16.
    pub fn of(v: u16) -> Option<Magic> {
        Some(match v {
            0x137f => Magic::V1,
            0x138f => Magic::V1LongNames,
            0x2468 => Magic::V2,
            0x2478 => Magic::V2LongNames,
            _ => return None,
        })
    }
    /// True for the v1 layouts (16-bit zones).
    pub fn is_v1(self) -> bool {
        matches!(self, Magic::V1 | Magic::V1LongNames)
    }
    /// Maximum filename length (14 or 30).
    pub fn name_len(self) -> usize {
        match self {
            Magic::V1 | Magic::V2 => 14,
            _ => 30,
        }
    }
}

/// A parsed MINIX superblock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Minix {
    /// Inodes (`s_ninodes`).
    pub ninodes: u16,
    /// v1 16-bit zones (`s_nzones`, ignored on v2).
    pub nzones16: u16,
    /// Inode map blocks (`s_imap_blocks`).
    pub imap_blocks: u16,
    /// Zone map blocks (`s_zmap_blocks`).
    pub zmap_blocks: u16,
    /// First data zone (`s_firstdatazone`).
    pub first_data_zone: u16,
    /// `log2` of zone size in 1K blocks (`s_log_zone_size`).
    pub log_zone_size: u16,
    /// Maximum file size (`s_max_size`).
    pub max_size: u32,
    /// Version magic.
    pub magic: Magic,
    /// `s_state`: 1 = clean mount.
    pub state: u16,
    /// v2 32-bit zones (`s_zones`, ignored on v1).
    pub zones32: u32,
}

impl Minix {
    /// Zone count resolved by version (16-bit on v1, 32-bit on
    /// v2).
    pub fn zones(&self) -> u32 {
        if self.magic.is_v1() {
            u32::from(self.nzones16)
        } else {
            self.zones32
        }
    }
    /// Zone size in bytes (`1024 << s_log_zone_size`).
    pub fn zone_size(&self) -> u32 {
        1024u32
            .checked_shl(u32::from(self.log_zone_size))
            .unwrap_or(0)
    }
}

/// Parse the superblock at offset 1024. Returns `None` on a bad
/// magic or truncation.
pub fn parse(d: &[u8]) -> Option<Minix> {
    let s = d.get(SUPER_AT..SUPER_AT + 24)?;
    Some(Minix {
        ninodes: le16(s, 0)?,
        nzones16: le16(s, 2)?,
        imap_blocks: le16(s, 4)?,
        zmap_blocks: le16(s, 6)?,
        first_data_zone: le16(s, 8)?,
        log_zone_size: le16(s, 10)?,
        max_size: le32(s, 12)?,
        magic: Magic::of(le16(s, 16)?)?,
        state: le16(s, 18)?,
        zones32: le32(s, 20)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn fixture(magic: u16) -> Vec<u8> {
        let mut d = vec![0u8; SUPER_AT + 64];
        let w16 = |d: &mut [u8], o: usize, v: u16| {
            let a = SUPER_AT + o;
            d[a] = v as u8;
            d[a + 1] = (v >> 8) as u8;
        };
        let w32 = |d: &mut [u8], o: usize, v: u32| {
            for i in 0..4 {
                d[SUPER_AT + o + i] = (v >> (i * 8)) as u8;
            }
        };
        w16(&mut d, 0, 480);
        w16(&mut d, 2, 720);
        w16(&mut d, 4, 2);
        w16(&mut d, 6, 4);
        w16(&mut d, 8, 33);
        w16(&mut d, 10, 0);
        w32(&mut d, 12, 16_777_216);
        w16(&mut d, 16, magic);
        w16(&mut d, 18, 1);
        w32(&mut d, 20, 1440);
        d
    }

    #[test]
    fn v2_fields() {
        let s = parse(&fixture(0x2478)).unwrap();
        assert_eq!(s.magic, Magic::V2LongNames);
        assert_eq!(s.magic.name_len(), 30);
        assert!(!s.magic.is_v1());
        assert_eq!(s.ninodes, 480);
        assert_eq!(s.imap_blocks, 2);
        assert_eq!(s.zmap_blocks, 4);
        assert_eq!(s.first_data_zone, 33);
        assert_eq!(s.max_size, 16_777_216);
        assert_eq!(s.state, 1);
        assert_eq!(s.zones(), 1440);
        assert_eq!(s.zone_size(), 1024);
    }

    #[test]
    fn v1_uses_16bit_zones() {
        let s = parse(&fixture(0x137f)).unwrap();
        assert_eq!(s.magic, Magic::V1);
        assert!(s.magic.is_v1());
        assert_eq!(s.magic.name_len(), 14);
        assert_eq!(s.zones(), 720);
    }

    #[test]
    fn rejects() {
        assert!(parse(&fixture(0x1234)).is_none());
        assert!(parse(&[0u8; 100]).is_none());
        assert_eq!(Magic::of(0x2468), Some(Magic::V2));
        assert_eq!(Magic::of(0x9999), None);
    }
}
