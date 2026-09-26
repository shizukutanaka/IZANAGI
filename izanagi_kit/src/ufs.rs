//! UFS1/UFS2 superblock (Berkeley Fast File System).
//!
//! The UFS superblock is searched at the standard 8 KiB offset
//! (some disks carry copies at other sector-aligned locations —
//! `find` scans a few candidates). `fs_magic` at offset `0x55C`
//! (1372) distinguishes UFS1 (`0x00011954`) from UFS2
//! (`0x19540119`, which moved many fields to 64-bit). `parse`
//! reports both layouts.
//!
//! ```
//! use izanagi_kit::ufs::{parse, MAGIC_UFS1, MAGIC_UFS2, SB_AT};
//!
//! let mut d = vec![0u8; SB_AT + 2048];
//! d[SB_AT + 1372..SB_AT + 1376].copy_from_slice(&MAGIC_UFS1.to_le_bytes()[..]);
//! d[SB_AT + 8] = 4; d[SB_AT + 9] = 0; d[SB_AT + 10] = 0; d[SB_AT + 11] = 0; // bsize 1024
//! let u = parse(&d).unwrap();
//! assert_eq!(u.magic, MAGIC_UFS1);
//! assert!(u.is_ufs1());
//! ```

/// Primary superblock byte offset.
pub const SB_AT: usize = 8192;
/// UFS1 magic (`fs_magic` at +1372).
pub const MAGIC_UFS1: u32 = 0x0001_1954;
/// UFS2 magic.
pub const MAGIC_UFS2: u32 = 0x1954_0119;
/// `fs_magic` offset inside the superblock.
pub const MAGIC_AT: usize = 1372;

fn le32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(at)?)
            | u32::from(*d.get(at + 1)?) << 8
            | u32::from(*d.get(at + 2)?) << 16
            | u32::from(*d.get(at + 3)?) << 24,
    )
}

/// A parsed UFS superblock (fields shared between UFS1 and UFS2;
/// `kind` selects the layout).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ufs {
    /// `fs_magic` raw value.
    pub magic: u32,
    /// True for UFS2 (0x19540119).
    pub is_ufs2: bool,
    /// `fs_bsize` — file system block size.
    pub block_size: u32,
    /// `fs_fsize` — fragment size.
    pub frag_size: u32,
    /// `fs_frag` — fragments per block.
    pub frags_per_block: u32,
    /// `fs_ncg` — cylinder groups.
    pub cyl_groups: u32,
    /// `fs_fpg` — blocks per cylinder group.
    pub blocks_per_group: u32,
    /// `fs_ipg` — inodes per cylinder group.
    pub inodes_per_group: u32,
    /// `fs_minfree` percent.
    pub minfree_percent: i32,
    /// `fs_size` — total blocks.
    pub total_blocks: u64,
    /// `fs_dsize` — data blocks.
    pub data_blocks: u64,
    /// `fs_cstotal.cs_nbfree` — free blocks total.
    pub free_blocks_total: u64,
    /// `fs_cstotal.cs_nifree` — free inodes total.
    pub free_inodes_total: u64,
    /// `fs_fsmnt` — last-mounted-on path (52 bytes).
    pub mounted_at: String,
}

impl Ufs {
    /// True for a UFS1 layout (`0x00011954`).
    pub fn is_ufs1(&self) -> bool {
        !self.is_ufs2
    }
}

/// Parse the superblock at `at` (normally [`SB_AT`]). Returns
/// `None` when `fs_magic` is neither UFS1 nor UFS2.
pub fn parse_at(d: &[u8], at: usize) -> Option<Ufs> {
    let s = d.get(at..at + 1600)?;
    let magic = le32(s, MAGIC_AT)?;
    let is_ufs2 = match magic {
        MAGIC_UFS1 => false,
        MAGIC_UFS2 => true,
        _ => return None,
    };
    // Common early fields (same offsets in UFS1/2 for the LE32
    // part of the header).
    let frag_size = le32(s, 4)?;
    let block_size = le32(s, 8)?;
    let frags_per_block = le32(s, 20)?;
    let cyl_groups = le32(s, 28)?;
    let blocks_per_group = le32(s, 44)?;
    let inodes_per_group = le32(s, 48)?;
    let minfree_percent = le32(s, 56)? as i32;
    let (total_blocks, data_blocks, fb, fi) = if is_ufs2 {
        // UFS2 moved fs_size/fs_dsize to 64-bit at 0x200/0x208
        // and cs_nbfree/cs_nifree to 64-bit inside cstotal.
        let lo64 = |o: usize| -> Option<u64> {
            Some(u64::from(le32(s, o)?) | u64::from(le32(s, o + 4)?) << 32)
        };
        // cstotal is 4 x i64 at 0x400: ndir@1024, nbfree@1032,
        // nifree@1040, nffree@1048.
        (lo64(0x200)?, lo64(0x208)?, lo64(1032)?, lo64(1040)?)
    } else {
        (
            u64::from(le32(s, 128)?),
            u64::from(le32(s, 132)?),
            // cstotal is 4 x i32 at 552: ndir@552, nbfree@556,
            // nifree@560, nffree@564.
            u64::from(le32(s, 556)?),
            u64::from(le32(s, 560)?),
        )
    };
    let raw = s.get(1096..1148)?;
    let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
    Some(Ufs {
        magic,
        is_ufs2,
        block_size,
        frag_size,
        frags_per_block,
        cyl_groups,
        blocks_per_group,
        inodes_per_group,
        minfree_percent,
        total_blocks,
        data_blocks,
        free_blocks_total: fb,
        free_inodes_total: fi,
        mounted_at: core::str::from_utf8(&raw[..end]).unwrap_or("").to_string(),
    })
}

/// Parse at the standard [`SB_AT`] offset.
pub fn parse(d: &[u8]) -> Option<Ufs> {
    parse_at(d, SB_AT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn fixture(magic: u32) -> Vec<u8> {
        let mut d = vec![0u8; SB_AT + 2048];
        let w = |d: &mut [u8], o: usize, v: u32| {
            let a = SB_AT + o;
            d[a] = v as u8;
            d[a + 1] = (v >> 8) as u8;
            d[a + 2] = (v >> 16) as u8;
            d[a + 3] = (v >> 24) as u8;
        };
        w(&mut d, 4, 4096); // frag
        w(&mut d, 8, 32768); // block
        w(&mut d, 20, 8); // frags/block
        w(&mut d, 28, 50); // ncg
        w(&mut d, 44, 2048); // fpg
        w(&mut d, 48, 6400); // ipg
        w(&mut d, 56, 5); // minfree
        w(&mut d, 128, 102_400); // size (ufs1)
        w(&mut d, 132, 100_000); // dsize
        if magic == MAGIC_UFS2 {
            // 64-bit counters: nbfree@1032, nifree@1040
            w(&mut d, 1032, 8_000);
            w(&mut d, 1040, 300_000);
        } else {
            // 32-bit counters: nbfree@556, nifree@560
            w(&mut d, 556, 8_000);
            w(&mut d, 560, 300_000);
        }
        let m = b"/mnt/ufs";
        d[SB_AT + 1096..SB_AT + 1096 + m.len()].copy_from_slice(m);
        w(&mut d, MAGIC_AT, magic);
        d
    }

    #[test]
    fn ufs1_fields() {
        let u = parse(&fixture(MAGIC_UFS1)).unwrap();
        assert!(u.is_ufs1());
        assert_eq!(u.magic, MAGIC_UFS1);
        assert_eq!(u.frag_size, 4096);
        assert_eq!(u.block_size, 32768);
        assert_eq!(u.frags_per_block, 8);
        assert_eq!(u.cyl_groups, 50);
        assert_eq!(u.blocks_per_group, 2048);
        assert_eq!(u.inodes_per_group, 6400);
        assert_eq!(u.minfree_percent, 5);
        assert_eq!(u.total_blocks, 102_400);
        assert_eq!(u.data_blocks, 100_000);
        assert_eq!(u.free_blocks_total, 8_000);
        assert_eq!(u.free_inodes_total, 300_000);
        assert_eq!(u.mounted_at, "/mnt/ufs");
    }

    #[test]
    fn ufs2_magic_and_rejects() {
        let mut d = fixture(MAGIC_UFS2);
        // give the ufs2 64-bit fields real values
        let w = |d: &mut [u8], o: usize, v: u64| {
            for i in 0..8 {
                d[SB_AT + o + i] = (v >> (i * 8)) as u8;
            }
        };
        w(&mut d, 0x200, 999_999);
        w(&mut d, 0x208, 888_888);
        let u = parse(&d).unwrap();
        assert!(u.is_ufs2);
        assert_eq!(u.total_blocks, 999_999);
        assert_eq!(u.data_blocks, 888_888);
        assert_eq!(u.free_blocks_total, 8_000);
        assert_eq!(u.free_inodes_total, 300_000);
        // A large 64-bit inode counter must not bleed into nbfree.
        w(&mut d, 1040, 0x1_0000_0001);
        let u = parse(&d).unwrap();
        assert_eq!(u.free_inodes_total, 0x1_0000_0001);
        assert_eq!(u.free_blocks_total, 8_000);
        assert!(parse(&fixture(0xDEAD_BEEF)).is_none());
        assert!(parse(&[0u8; 100]).is_none());
        // parse_at honors an explicit offset (e.g. an alternate
        // superblock location).
        let mut off = vec![0u8; 512 + 2048];
        off[512 + MAGIC_AT..512 + MAGIC_AT + 4].copy_from_slice(&MAGIC_UFS1.to_le_bytes());
        off[512 + 8] = 4; // bsize low byte
        let u2 = parse_at(&off, 512).unwrap();
        assert!(u2.is_ufs1());
        assert!(parse_at(&off, 0).is_none());
    }
}
