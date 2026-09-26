//! Blizzard MPQ archive header.
//!
//! A MPQ opens with `MPQ\x1A` (0x1A51504D) followed by the v1
//! header: header size, archive size, format version, sector
//! shift, hash/block table positions and entry counts. v2+ adds
//! the extended block-table offset, hi-table positions and the
//! large-file bit. `parse` bounds-checks every declared table.
//!
//! ```
//! use izanagi_kit::mpq::{parse, MAGIC};
//!
//! let mut d = vec![0u8; 512];
//! d[..4].copy_from_slice(&MAGIC.to_le_bytes());
//! let w = |d: &mut [u8], o: usize, v: u32| {
//!     for i in 0..4 { d[o + i] = (v >> (i * 8)) as u8; }
//! };
//! w(&mut d, 4, 32);           // header size
//! w(&mut d, 8, 512);          // archive size
//! w(&mut d, 14, 3);           // sector shift -> 4096
//! w(&mut d, 16, 32);          // hash table offset
//! w(&mut d, 24, 4);           // hash table entries
//! w(&mut d, 28, 2);           // block table entries
//! let m = parse(&d).unwrap();
//! assert_eq!(m.sector_size(), 4096);
//! assert_eq!(m.hash_table_entries, 4);
//! ```

/// `MPQ\x1A` user-header magic.
pub const MAGIC: u32 = 0x1a51_504d;
/// v1 header size.
pub const HEADER_V1: usize = 32;
/// v2 header size (adds extended fields).
pub const HEADER_V2: usize = 44;

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
fn le64(d: &[u8], at: usize) -> Option<u64> {
    Some(u64::from(le32(d, at)?) | u64::from(le32(d, at + 4)?) << 32)
}

/// A parsed MPQ header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mpq {
    /// `dwHeaderSize` (≥32).
    pub header_size: u32,
    /// `dwArchiveSize`.
    pub archive_size: u32,
    /// `wFormatVersion` (0..=3).
    pub format_version: u16,
    /// `wSectorSizeShift` — sector = `512 << shift`.
    pub sector_shift: u16,
    /// Hash table offset.
    pub hash_table_pos: u32,
    /// Block table offset.
    pub block_table_pos: u32,
    /// Hash table entry count (16B each).
    pub hash_table_entries: u32,
    /// Block table entry count (16B each).
    pub block_table_entries: u32,
    /// v2+: hi-block table 64-bit offset (`None` on v1).
    pub hi_block_table_pos: Option<u64>,
    /// v2+: hash table high 16 bits.
    pub hash_table_pos_hi: Option<u16>,
    /// v2+: block table high 16 bits.
    pub block_table_pos_hi: Option<u16>,
}

impl Mpq {
    /// Sector size in bytes (`512 << shift`).
    pub fn sector_size(&self) -> u32 {
        512u32
            .checked_shl(u32::from(self.sector_shift))
            .unwrap_or(0)
    }
    /// Byte end of the hash table.
    pub fn hash_table_end(&self) -> u64 {
        u64::from(self.hash_table_pos) + u64::from(self.hash_table_entries) * 16
    }
    /// Byte end of the block table.
    pub fn block_table_end(&self) -> u64 {
        u64::from(self.block_table_pos) + u64::from(self.block_table_entries) * 16
    }
}

/// Parse an MPQ header. Returns `None` on a bad magic, a header
/// smaller than v1, or declared tables that escape the buffer.
pub fn parse(d: &[u8]) -> Option<Mpq> {
    if le32(d, 0)? != MAGIC {
        return None;
    }
    let header_size = le32(d, 4)?;
    if header_size < HEADER_V1 as u32 {
        return None;
    }
    let format_version = le16(d, 12)?;
    let m = Mpq {
        header_size,
        archive_size: le32(d, 8)?,
        format_version,
        sector_shift: le16(d, 14)?,
        hash_table_pos: le32(d, 16)?,
        block_table_pos: le32(d, 20)?,
        hash_table_entries: le32(d, 24)?,
        block_table_entries: le32(d, 28)?,
        hi_block_table_pos: if format_version >= 2 {
            Some(le64(d, 32)?)
        } else {
            None
        },
        hash_table_pos_hi: if format_version >= 2 {
            Some(le16(d, 40)?)
        } else {
            None
        },
        block_table_pos_hi: if format_version >= 2 {
            Some(le16(d, 42)?)
        } else {
            None
        },
    };
    let len = d.len() as u64;
    if m.hash_table_end() > len || m.block_table_end() > len {
        return None;
    }
    Some(m)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn fixture(ver: u16) -> Vec<u8> {
        let mut d = vec![0u8; 4096];
        d[..4].copy_from_slice(&MAGIC.to_le_bytes());
        let w = |d: &mut [u8], o: usize, v: u32| {
            for i in 0..4 {
                d[o + i] = (v >> (i * 8)) as u8;
            }
        };
        w(&mut d, 4, if ver >= 2 { 44 } else { 32 });
        w(&mut d, 8, 4096);
        d[12] = ver as u8;
        d[13] = (ver >> 8) as u8;
        d[14] = 3; // shift
        d[15] = 0;
        w(&mut d, 16, 1024); // hash pos
        w(&mut d, 20, 2048); // block pos
        w(&mut d, 24, 8); // hash entries (128B)
        w(&mut d, 28, 4); // block entries (64B)
        if ver >= 2 {
            w(&mut d, 32, 3072); // hi block table lo32
            w(&mut d, 36, 0);
            d[40] = 0; // hash hi
            d[41] = 0;
            d[42] = 1; // block hi
            d[43] = 0;
        }
        d
    }

    #[test]
    fn v1_fields() {
        let m = parse(&fixture(0)).unwrap();
        assert_eq!(m.header_size, 32);
        assert_eq!(m.archive_size, 4096);
        assert_eq!(m.format_version, 0);
        assert_eq!(m.sector_size(), 4096);
        assert_eq!(m.hash_table_pos, 1024);
        assert_eq!(m.hash_table_entries, 8);
        assert_eq!(m.hash_table_end(), 1024 + 128);
        assert_eq!(m.block_table_end(), 2048 + 64);
        assert_eq!(m.hi_block_table_pos, None);
    }

    #[test]
    fn v2_and_rejects() {
        let m = parse(&fixture(2)).unwrap();
        assert_eq!(m.format_version, 2);
        assert_eq!(m.hi_block_table_pos, Some(3072));
        assert_eq!(m.block_table_pos_hi, Some(1));
        // table past EOF
        let mut bad = fixture(0);
        bad[24] = 255; // hash entries 255 * 16 + 1024 > 4096
        assert!(parse(&bad).is_none());
        // bad magic / short header
        let mut bad2 = fixture(0);
        bad2[0] = 0;
        assert!(parse(&bad2).is_none());
        let mut bad3 = fixture(0);
        bad3[4] = 16; // header_size < 32
        assert!(parse(&bad3).is_none());
        assert!(parse(&[0u8; 8]).is_none());
    }
}
