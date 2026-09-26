//! ext2/ext3/ext4 superblock.
//!
//! The superblock lives at byte offset 1024 and carries the block
//! geometry (`s_log_block_size` → `1024 << n` bytes), inode and
//! group counts, mount state, the `0xEF53` magic and the
//! `feature_compat`/`incompat`/`ro_compat` masks that distinguish
//! ext2 from ext3/ext4 extents and journaling.
//!
//! ```
//! use izanagi_kit::ext2::{parse, SUPER_AT, MAGIC};
//!
//! let mut d = vec![0u8; SUPER_AT + 200];
//! let w = |d: &mut [u8], o: usize, v: u32| {
//!     d[SUPER_AT + o] = v as u8; d[SUPER_AT + o + 1] = (v >> 8) as u8;
//!     d[SUPER_AT + o + 2] = (v >> 16) as u8; d[SUPER_AT + o + 3] = (v >> 24) as u8;
//! };
//! w(&mut d, 0, 2000);        // inodes
//! w(&mut d, 4, 8192);        // blocks
//! w(&mut d, 24, 2);          // log block size -> 4096
//! w(&mut d, 32, 8192);       // blocks/group
//! w(&mut d, 40, 2000);       // inodes/group
//! w(&mut d, 56, MAGIC as u32); // magic (u16 low)
//! w(&mut d, 58, 1);          // state: clean
//! w(&mut d, 88, 256);        // inode size
//! let s = parse(&d).unwrap();
//! assert_eq!(s.block_size(), 4096);
//! assert_eq!(s.inodes, 2000);
//! ```

/// Byte offset of the superblock.
pub const SUPER_AT: usize = 1024;
/// `s_magic` value.
pub const MAGIC: u16 = 0xef53;
/// `s_state`: cleanly unmounted.
pub const STATE_CLEAN: u16 = 1;
/// `s_state`: errors detected.
pub const STATE_ERRORS: u16 = 2;
/// `incompat`: journaling present (ext3+).
pub const INCOMPAT_JOURNAL: u32 = 0x0004;
/// `incompat`: extents (ext4).
pub const INCOMPAT_EXTENTS: u32 = 0x0040;
/// `incompat`: 64-bit block counts (ext4).
pub const INCOMPAT_64BIT: u32 = 0x0080;

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

/// A parsed ext2-family superblock.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ext2 {
    /// Total inodes.
    pub inodes: u32,
    /// Total blocks (32-bit count; 64-bit ext4 uses `blocks_hi`).
    pub blocks: u32,
    /// Reserved blocks.
    pub reserved_blocks: u32,
    /// Free blocks/inodes.
    pub free_blocks: u32,
    /// Free inodes.
    pub free_inodes: u32,
    /// First data block (0 on 1K fs, 1 on larger).
    pub first_data_block: u32,
    /// `s_log_block_size`.
    pub log_block_size: u32,
    /// Blocks per group.
    pub blocks_per_group: u32,
    /// Inodes per group.
    pub inodes_per_group: u32,
    /// Mount count since last fsck.
    pub mount_count: u16,
    /// `s_max_mnt_count` (as u16 bits; `0xFFFF` = disabled).
    pub max_mount_count: u16,
    /// Filesystem state (`STATE_*`).
    pub state: u16,
    /// Error-handling policy (1 continue, 2 remount-ro, 3 panic).
    pub errors: u16,
    /// Creator OS (0 linux, 1 hurd, 2 masix, 3 freebsd, 4 lites).
    pub creator_os: u32,
    /// Revision level (0 or 1 dynamic).
    pub rev_level: u32,
    /// First non-reserved inode (11 on rev0).
    pub first_ino: u32,
    /// Inode size in bytes.
    pub inode_size: u16,
    /// Feature masks.
    pub feature_compat: u32,
    /// Incompatible features.
    pub feature_incompat: u32,
    /// Read-only-compatible features.
    pub feature_ro_compat: u32,
    /// Filesystem UUID.
    pub uuid: [u8; 16],
    /// Volume label (16B, NUL-trimmed).
    pub volume_name: String,
}

impl Ext2 {
    /// Block size in bytes (`1024 << s_log_block_size`).
    pub fn block_size(&self) -> u32 {
        1024u32.checked_shl(self.log_block_size).unwrap_or(0)
    }
    /// True when the journal inode feature is present.
    pub fn has_journal(&self) -> bool {
        self.feature_incompat & INCOMPAT_JOURNAL != 0
    }
    /// True when extents are used (ext4).
    pub fn has_extents(&self) -> bool {
        self.feature_incompat & INCOMPAT_EXTENTS != 0
    }
    /// Number of block groups (ceil-div of the group span).
    pub fn block_groups(&self) -> u32 {
        if self.blocks_per_group == 0 {
            return 0;
        }
        self.blocks
            .saturating_sub(self.first_data_block)
            .div_ceil(self.blocks_per_group)
    }
}

/// Parse a superblock at offset 1024. Returns `None` on a bad
/// magic or a truncated header.
pub fn parse(d: &[u8]) -> Option<Ext2> {
    let s = d.get(SUPER_AT..SUPER_AT + 184)?;
    let magic = le16(s, 56)?;
    if magic != MAGIC {
        return None;
    }
    let mut uuid = [0u8; 16];
    uuid.copy_from_slice(s.get(104..120)?);
    let name_raw = s.get(120..136)?;
    let end = name_raw
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(name_raw.len());
    Some(Ext2 {
        inodes: le32(s, 0)?,
        blocks: le32(s, 4)?,
        reserved_blocks: le32(s, 8)?,
        free_blocks: le32(s, 12)?,
        free_inodes: le32(s, 16)?,
        first_data_block: le32(s, 20)?,
        log_block_size: le32(s, 24)?,
        blocks_per_group: le32(s, 32)?,
        inodes_per_group: le32(s, 40)?,
        mount_count: le16(s, 52)?,
        max_mount_count: le16(s, 54)?,
        state: le16(s, 58)?,
        errors: le16(s, 60)?,
        creator_os: le32(s, 72)?,
        rev_level: le32(s, 76)?,
        first_ino: le32(s, 84)?,
        inode_size: le16(s, 88)?,
        feature_compat: le32(s, 92)?,
        feature_incompat: le32(s, 96)?,
        feature_ro_compat: le32(s, 100)?,
        uuid,
        volume_name: core::str::from_utf8(&name_raw[..end])
            .unwrap_or("")
            .to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0u8; SUPER_AT + 200];
        let w32 = |d: &mut [u8], o: usize, v: u32| {
            let a = SUPER_AT + o;
            d[a] = v as u8;
            d[a + 1] = (v >> 8) as u8;
            d[a + 2] = (v >> 16) as u8;
            d[a + 3] = (v >> 24) as u8;
        };
        let w16 = |d: &mut [u8], o: usize, v: u16| {
            let a = SUPER_AT + o;
            d[a] = v as u8;
            d[a + 1] = (v >> 8) as u8;
        };
        w32(&mut d, 0, 5000);
        w32(&mut d, 4, 20000);
        w32(&mut d, 8, 1000);
        w32(&mut d, 12, 18000);
        w32(&mut d, 16, 4990);
        w32(&mut d, 20, 0);
        w32(&mut d, 24, 0);
        w32(&mut d, 32, 8192);
        w32(&mut d, 40, 2048);
        w16(&mut d, 52, 4);
        w16(&mut d, 54, 30);
        w16(&mut d, 56, MAGIC);
        w16(&mut d, 58, STATE_CLEAN);
        w16(&mut d, 60, 2);
        w32(&mut d, 72, 0);
        w32(&mut d, 76, 1);
        w32(&mut d, 84, 11);
        w16(&mut d, 88, 256);
        w32(&mut d, 96, INCOMPAT_JOURNAL | INCOMPAT_EXTENTS);
        d[SUPER_AT + 120..SUPER_AT + 125].copy_from_slice(b"rootv");
        d
    }

    #[test]
    fn fields() {
        let s = parse(&fixture()).unwrap();
        assert_eq!(s.inodes, 5000);
        assert_eq!(s.blocks, 20000);
        assert_eq!(s.block_size(), 1024);
        assert_eq!(s.blocks_per_group, 8192);
        assert_eq!(s.block_groups(), 3); // ceil(20000/8192)
        assert_eq!(s.state, STATE_CLEAN);
        assert_eq!(s.errors, 2);
        assert_eq!(s.rev_level, 1);
        assert_eq!(s.first_ino, 11);
        assert_eq!(s.inode_size, 256);
        assert!(s.has_journal());
        assert!(s.has_extents());
        assert_eq!(s.volume_name, "rootv");
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 200]).is_none());
        let mut d = fixture();
        d[SUPER_AT + 56] = 0x12; // bad magic
        assert!(parse(&d).is_none());
    }
}
