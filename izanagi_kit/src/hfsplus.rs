//! HFS+ / HFSX volume header.
//!
//! The volume header sits 1024 bytes into the volume, opens with
//! `H+` (HFS+) or `HX` (HFSX) and a version u16, then attributes,
//! counts, the allocation block geometry and the five 80-byte
//! `ForkData` descriptors for the special files (allocation,
//! extents, catalog, attributes, startup). All fields are
//! big-endian.
//!
//! ```
//! use izanagi_kit::hfsplus::{parse, VOL_AT, Signature};
//!
//! let mut d = vec![0u8; VOL_AT + 512];
//! let w32 = |d: &mut [u8], o: usize, v: u32| {
//!     d[VOL_AT + o] = (v >> 24) as u8; d[VOL_AT + o + 1] = (v >> 16) as u8;
//!     d[VOL_AT + o + 2] = (v >> 8) as u8; d[VOL_AT + o + 3] = v as u8;
//! };
//! d[VOL_AT] = b'H'; d[VOL_AT + 1] = b'+';
//! d[VOL_AT + 3] = 4;          // version 4
//! w32(&mut d, 32, 12345);     // file count
//! w32(&mut d, 40, 4096);      // block size
//! w32(&mut d, 44, 1_000_000); // total blocks
//! let v = parse(&d).unwrap();
//! assert_eq!(v.signature, Signature::HfsPlus);
//! assert_eq!(v.block_size, 4096);
//! ```

/// Volume-header byte offset.
pub const VOL_AT: usize = 1024;

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
    Some(u64::from(be32(d, at)?) << 32 | u64::from(be32(d, at + 4)?))
}

/// Signature word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Signature {
    /// `H+` — HFS Plus.
    HfsPlus,
    /// `HX` — HFSX (case-sensitive).
    HfsX,
}

/// One `ForkData` descriptor (80 bytes): logical size, clump
/// size, total blocks, then 8 `(start_block, block_count)`
/// extent pairs — kept raw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fork {
    /// Logical size in bytes.
    pub logical_size: u64,
    /// Clump size.
    pub clump_size: u32,
    /// Total blocks allocated.
    pub total_blocks: u32,
    /// First extent's start block (convenience; all 8 pairs stay raw).
    pub first_extent_block: u32,
    /// First extent's block count.
    pub first_extent_blocks: u32,
}

/// A parsed HFS+ volume header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HfsPlus {
    /// `H+` / `HX` / unknown.
    pub signature: Signature,
    /// Version (4 = HFS+, 5 = HFSX).
    pub version: u16,
    /// Volume attributes bitfield.
    pub attributes: u32,
    /// `lastMountedVersion` (e.g. `10.0`, `fsck`).
    pub last_mounted: u32,
    /// Journal info block address (0 = none).
    pub journal_info_block: u32,
    /// Creation/modify/backup/checked timestamps (Mac epoch
    /// 1904 — kept as raw u32 seconds).
    pub create_date: u32,
    /// Modify timestamp.
    pub modify_date: u32,
    /// Backup timestamp.
    pub backup_date: u32,
    /// Checked timestamp.
    pub checked_date: u32,
    /// File/folder counts.
    pub file_count: u32,
    /// Folder count.
    pub folder_count: u32,
    /// Allocation block size.
    pub block_size: u32,
    /// Total blocks.
    pub total_blocks: u32,
    /// Free blocks.
    pub free_blocks: u32,
    /// Next allocation hint.
    pub next_allocation: u32,
    /// Next catalog node id.
    pub next_catalog_id: u32,
    /// Write count.
    pub write_count: u32,
    /// The five special-file forks: allocation, extents, catalog,
    /// attributes, startup.
    pub forks: [Fork; 5],
}

fn fork(d: &[u8], at: usize) -> Option<Fork> {
    Some(Fork {
        logical_size: be64(d, at)?,
        clump_size: be32(d, at + 8)?,
        total_blocks: be32(d, at + 12)?,
        first_extent_block: be32(d, at + 16)?,
        first_extent_blocks: be32(d, at + 20)?,
    })
}

/// Parse the volume header. Returns `None` on a bad signature or
/// a truncated header.
pub fn parse(d: &[u8]) -> Option<HfsPlus> {
    let s = d.get(VOL_AT..VOL_AT + 512)?;
    let sig = be16(s, 0)?;
    let signature = match sig {
        0x482b => Signature::HfsPlus,
        0x4858 => Signature::HfsX,
        _ => return None,
    };
    let mut forks = [Fork {
        logical_size: 0,
        clump_size: 0,
        total_blocks: 0,
        first_extent_block: 0,
        first_extent_blocks: 0,
    }; 5];
    for (i, f) in forks.iter_mut().enumerate() {
        *f = fork(s, 112 + i * 80)?;
    }
    Some(HfsPlus {
        signature,
        version: be16(s, 2)?,
        attributes: be32(s, 4)?,
        last_mounted: be32(s, 8)?,
        journal_info_block: be32(s, 12)?,
        create_date: be32(s, 16)?,
        modify_date: be32(s, 20)?,
        backup_date: be32(s, 24)?,
        checked_date: be32(s, 28)?,
        file_count: be32(s, 32)?,
        folder_count: be32(s, 36)?,
        block_size: be32(s, 40)?,
        total_blocks: be32(s, 44)?,
        free_blocks: be32(s, 48)?,
        next_allocation: be32(s, 52)?,
        next_catalog_id: be32(s, 64)?,
        write_count: be32(s, 68)?,
        forks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn fixture(sig: &[u8; 2]) -> Vec<u8> {
        let mut d = vec![0u8; VOL_AT + 512];
        d[VOL_AT] = sig[0];
        d[VOL_AT + 1] = sig[1];
        let w32 = |d: &mut [u8], o: usize, v: u32| {
            let a = VOL_AT + o;
            d[a] = (v >> 24) as u8;
            d[a + 1] = (v >> 16) as u8;
            d[a + 2] = (v >> 8) as u8;
            d[a + 3] = v as u8;
        };
        let w64 = |d: &mut [u8], o: usize, v: u64| {
            let a = VOL_AT + o;
            for i in 0..8 {
                d[a + i] = (v >> ((7 - i) * 8)) as u8;
            }
        };
        d[VOL_AT + 2] = 0;
        d[VOL_AT + 3] = 4;
        w32(&mut d, 4, 1 << 10); // journal attribute bit? just a value
        w32(&mut d, 32, 400_000);
        w32(&mut d, 36, 25_000);
        w32(&mut d, 40, 4096);
        w32(&mut d, 44, 4_000_000);
        w32(&mut d, 48, 1_500_000);
        w32(&mut d, 52, 9_999);
        w32(&mut d, 64, 123_456);
        w32(&mut d, 68, 77);
        // catalog fork (3rd of 5, after the 112-byte common part):
        // logical size + clump + total + first extent
        w64(&mut d, 272, 16_777_216);
        w32(&mut d, 280, 8_388_608);
        w32(&mut d, 284, 4096);
        w32(&mut d, 288, 16);
        w32(&mut d, 292, 4096);
        d
    }

    #[test]
    fn fields() {
        let v = parse(&fixture(b"H+")).unwrap();
        assert_eq!(v.signature, Signature::HfsPlus);
        assert_eq!(v.version, 4);
        assert_eq!(v.file_count, 400_000);
        assert_eq!(v.folder_count, 25_000);
        assert_eq!(v.block_size, 4096);
        assert_eq!(v.total_blocks, 4_000_000);
        assert_eq!(v.free_blocks, 1_500_000);
        assert_eq!(v.next_catalog_id, 123_456);
        assert_eq!(v.write_count, 77);
        let cat = v.forks[2];
        assert_eq!(cat.logical_size, 16_777_216);
        assert_eq!(cat.first_extent_block, 16);
        assert_eq!(cat.first_extent_blocks, 4096);
    }

    #[test]
    fn hfsx_and_rejects() {
        assert_eq!(parse(&fixture(b"HX")).unwrap().signature, Signature::HfsX);
        assert!(parse(&fixture(b"H-")).is_none());
        assert!(parse(&[0u8; 500]).is_none());
    }
}
