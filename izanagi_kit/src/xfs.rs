//! XFS superblock.
//!
//! The primary superblock sits at sector 0 of an allocation group
//! and opens with `XFSB` (big-endian throughout — XFS is the rare
//! BE filesystem). `parse` reports block geometry, AG layout,
//! inode sizing, counts, feature words and the UUID.
//!
//! ```
//! use izanagi_kit::xfs::{parse, MAGIC};
//!
//! let mut d = vec![0u8; 512];
//! let w32 = |d: &mut [u8], o: usize, v: u32| {
//!     d[o] = (v >> 24) as u8; d[o + 1] = (v >> 16) as u8;
//!     d[o + 2] = (v >> 8) as u8; d[o + 3] = v as u8;
//! };
//! d[..4].copy_from_slice(&MAGIC.to_be_bytes());
//! w32(&mut d, 4, 4096);       // block size
//! w32(&mut d, 84, 65536);     // agblocks
//! w32(&mut d, 88, 8);         // agcount
//! let s = parse(&d).unwrap();
//! assert_eq!(s.block_size, 4096);
//! assert_eq!(s.ag_count, 8);
//! ```

/// `sb_magicnum` — `XFSB`.
pub const MAGIC: u32 = 0x5846_5342;
/// Minimum superblock size we read (covers v4 base + v5 extras
/// up to `sb_features2` at +200).
pub const HEADER: usize = 204;

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

/// A parsed XFS superblock.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Xfs {
    /// `sb_blocksize` in bytes.
    pub block_size: u32,
    /// `sb_dblocks` — total data blocks.
    pub dblocks: u64,
    /// `sb_rblocks` — realtime blocks.
    pub rblocks: u64,
    /// `sb_rextents` — realtime extents.
    pub rextents: u64,
    /// `sb_uuid`.
    pub uuid: [u8; 16],
    /// `sb_logstart` — journal start block.
    pub log_start: u64,
    /// `sb_rootino` — root inode.
    pub root_ino: u64,
    /// `sb_rextsize` — realtime extent size in blocks.
    pub rtext_size: u32,
    /// `sb_agblocks` — blocks per allocation group.
    pub ag_blocks: u32,
    /// `sb_agcount` — number of AGs.
    pub ag_count: u32,
    /// `sb_rbmblocks` — realtime bitmap blocks.
    pub rbm_blocks: u32,
    /// `sb_logblocks` — journal blocks.
    pub log_blocks: u32,
    /// `sb_versionnum` (low nibble = 4 or 5; high bits = flags).
    pub version: u16,
    /// `sb_sectsize` — underlying sector size.
    pub sector_size: u16,
    /// `sb_inodesize`.
    pub inode_size: u16,
    /// `sb_inopblock` — inodes per block.
    pub inodes_per_block: u16,
    /// `sb_fname` — 12-char fs name (not NUL-terminated).
    pub name: String,
    /// `sb_icount` — allocated inodes.
    pub icount: u64,
    /// `sb_ifree` — free inodes.
    pub ifree: u64,
    /// `sb_fdblocks` — free data blocks.
    pub free_dblocks: u64,
    /// `sb_frextents` — free realtime extents.
    pub free_rextents: u64,
    /// `sb_features2` (v5 only meaningful; 0 on short headers).
    pub features2: u32,
}

impl Xfs {
    /// Major version nibble (4 or 5).
    pub fn major(&self) -> u16 {
        self.version & 0x000f
    }
    /// Total filesystem size in bytes.
    pub fn size_bytes(&self) -> u64 {
        self.dblocks.saturating_mul(u64::from(self.block_size))
    }
}

/// Parse a superblock at offset 0. Returns `None` on a bad magic
/// or truncation.
pub fn parse(d: &[u8]) -> Option<Xfs> {
    let s = d.get(..HEADER)?;
    if be32(s, 0)? != MAGIC {
        return None;
    }
    let mut uuid = [0u8; 16];
    uuid.copy_from_slice(s.get(32..48)?);
    let name_raw = s.get(108..120)?;
    let end = name_raw
        .iter()
        .position(|&b| b == 0 || !(32..127).contains(&b))
        .unwrap_or(name_raw.len());
    Some(Xfs {
        block_size: be32(s, 4)?,
        dblocks: be64(s, 8)?,
        rblocks: be64(s, 16)?,
        rextents: be64(s, 24)?,
        uuid,
        log_start: be64(s, 48)?,
        root_ino: be64(s, 56)?,
        rtext_size: be32(s, 80)?,
        ag_blocks: be32(s, 84)?,
        ag_count: be32(s, 88)?,
        rbm_blocks: be32(s, 92)?,
        log_blocks: be32(s, 96)?,
        version: be16(s, 100)?,
        sector_size: be16(s, 102)?,
        inode_size: be16(s, 104)?,
        inodes_per_block: be16(s, 106)?,
        name: core::str::from_utf8(&name_raw[..end])
            .unwrap_or("")
            .to_string(),
        icount: be64(s, 128)?,
        ifree: be64(s, 136)?,
        free_dblocks: be64(s, 144)?,
        free_rextents: be64(s, 152)?,
        features2: be32(s, 200)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0u8; 512];
        let w32 = |d: &mut [u8], o: usize, v: u32| {
            d[o] = (v >> 24) as u8;
            d[o + 1] = (v >> 16) as u8;
            d[o + 2] = (v >> 8) as u8;
            d[o + 3] = v as u8;
        };
        let w64 = |d: &mut [u8], o: usize, v: u64| {
            for i in 0..8 {
                d[o + i] = (v >> ((7 - i) * 8)) as u8;
            }
        };
        let w16 = |d: &mut [u8], o: usize, v: u16| {
            d[o] = (v >> 8) as u8;
            d[o + 1] = v as u8;
        };
        w32(&mut d, 0, MAGIC);
        w32(&mut d, 4, 4096);
        w64(&mut d, 8, 1_048_576); // dblocks
        w64(&mut d, 48, 1024); // logstart
        w64(&mut d, 56, 128); // rootino
        w32(&mut d, 80, 64); // rextsize
        w32(&mut d, 84, 131_072); // agblocks
        w32(&mut d, 88, 8); // agcount
        w32(&mut d, 92, 4); // rbm
        w32(&mut d, 96, 2560); // logblocks
        w16(&mut d, 100, 0xB4A5); // v5 + attrs etc
        w16(&mut d, 102, 512);
        w16(&mut d, 104, 512);
        w16(&mut d, 106, 8);
        d[108..120].copy_from_slice(b"myxfs\0\0\0\0\0\0\0");
        w64(&mut d, 128, 512_000); // icount
        w64(&mut d, 136, 500_000); // ifree
        w64(&mut d, 144, 900_000); // fdblocks
        w64(&mut d, 152, 0);
        w32(&mut d, 200, 0x0000_0001); // features2
        d[32..48].copy_from_slice(&[0xab; 16]);
        d
    }

    #[test]
    fn fields() {
        let s = parse(&fixture()).unwrap();
        assert_eq!(s.block_size, 4096);
        assert_eq!(s.dblocks, 1_048_576);
        assert_eq!(s.size_bytes(), 1_048_576 * 4096);
        assert_eq!(s.log_start, 1024);
        assert_eq!(s.root_ino, 128);
        assert_eq!(s.ag_blocks, 131_072);
        assert_eq!(s.ag_count, 8);
        assert_eq!(s.major(), 5);
        assert_eq!(s.sector_size, 512);
        assert_eq!(s.inode_size, 512);
        assert_eq!(s.inodes_per_block, 8);
        assert_eq!(s.name, "myxfs");
        assert_eq!(s.icount, 512_000);
        assert_eq!(s.ifree, 500_000);
        assert_eq!(s.free_dblocks, 900_000);
        assert_eq!(s.features2, 1);
        assert_eq!(s.uuid, [0xab; 16]);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 100]).is_none());
        let mut d = fixture();
        d[0] = 0;
        assert!(parse(&d).is_none());
    }
}
