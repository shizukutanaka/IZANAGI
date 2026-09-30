//! ReiserFS superblock scanner.
//!
//! The ReiserFS superblock sits at byte offset `0x10000` (64 KiB) of the
//! device (journal code also probes `0x2000`). All fields are little-endian.
//! Layout of `reiserfs_super_block_v1`:
//!
//! `s_block_count u32` \@0, `s_free_blocks u32` \@4,
//! `s_root_block u32` \@8, `s_journal_block u32` \@12,
//! `s_journal_dev u32` \@16, `s_orig_journal_size u32` \@20,
//! `s_dum2 u32` \@24, `s_journal_trans_max u32` \@28,
//! `s_journal_block_count u32` \@32, `s_journal_max_batch u32` \@36,
//! `s_jncode u32` \@40, `s_bsize u16` \@44, `s_oid_maxsize u16` \@46,
//! `s_oid_cursize u16` \@48, `s_state u16` \@50, `s_magic[12]` \@52,
//! `s_hash_function_code u32` \@64, `s_tree_height u16` \@68,
//! `s_bmap_nr u16` \@70, `s_version u16` \@72.
//!
//! Magic strings: `ReIsErFs` (v1, format 3.5), `ReIsEr2Fs` (v2, format
//! 3.6), `ReIsEr3Fs` (v2 with newer journal fields).
//!
//! ```
//! let mut f = vec![0u8; 0x10000 + 0x100];
//! let s = 0x10000;
//! f[s..s + 4].copy_from_slice(&100000u32.to_le_bytes()); // block_count
//! f[s + 44..s + 46].copy_from_slice(&4096u16.to_le_bytes()); // s_bsize
//! f[s + 52..s + 61].copy_from_slice(b"ReIsEr2Fs");
//! let r = izanagi_kit::reiserfs::parse(&f).unwrap();
//! assert_eq!(r.format, 2);
//! assert_eq!(r.block_size, 4096);
//! ```
//!
//! Reference: Linux `fs/reiserfs/include/reiserfs_fs_sb.h`
//! (`REISERFS_DISK_OFFSET_IN_BYTES = 64 * 1024`, magic strings at offset
//! `52` inside the superblock).

/// Device offset where the primary superblock lives.
pub const SB_OFFSET: usize = 0x10000;

/// Parsed ReiserFS superblock.
#[derive(Debug, Clone, PartialEq)]
pub struct Reiserfs {
    /// Superblock format: `1` for `ReIsErFs` (3.5), `2` for `ReIsEr2Fs`
    /// (3.6), `3` for `ReIsEr3Fs`.
    pub format: u8,
    /// Total blocks (`s_block_count`).
    pub block_count: u32,
    /// Free blocks (`s_free_blocks`).
    pub free_blocks: u32,
    /// Root-directory block (`s_root_block`).
    pub root_block: u32,
    /// First journal block (`s_journal_block`).
    pub journal_block: u32,
    /// Block size in bytes (`s_bsize`).
    pub block_size: u16,
    /// Filesystem state (`s_state`): 1 = consistent, 2 = error.
    pub state: u16,
    /// Hash function code (`s_hash_function_code`): 0 unset, 1 rupasov,
    /// 3 r5, 5 tea…
    pub hash_function: u32,
    /// Height of the internal B+ tree (`s_tree_height`).
    pub tree_height: u16,
    /// Number of bitmap blocks (`s_bmap_nr`).
    pub bitmap_count: u16,
    /// On-disk version word (`s_version`).
    pub version: u16,
}

fn u16le(d: &[u8], off: usize) -> u16 {
    d[off] as u16 | ((d[off + 1] as u16) << 8)
}

fn u32le(d: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([d[off], d[off + 1], d[off + 2], d[off + 3]])
}

/// Parse a device image; `None` if no ReiserFS magic is present.
pub fn parse(d: &[u8]) -> Option<Reiserfs> {
    if d.len() < SB_OFFSET + 76 {
        return None;
    }
    let s = SB_OFFSET;
    let magic = &d[s + 52..s + 64];
    let format = if &magic[..8] == b"ReIsErFs" {
        1
    } else if &magic[..9] == b"ReIsEr2Fs" {
        2
    } else if &magic[..9] == b"ReIsEr3Fs" {
        3
    } else {
        return None;
    };
    let block_size = u16le(d, s + 44);
    // ReiserFS block sizes are powers of two in 512..=8192 (4 KiB typical).
    if !block_size.is_power_of_two() || !(512..=8192).contains(&block_size) {
        return None;
    }
    let block_count = u32le(d, s);
    let free_blocks = u32le(d, s + 4);
    if block_count == 0 || free_blocks > block_count {
        return None;
    }
    Some(Reiserfs {
        format,
        block_count,
        free_blocks,
        root_block: u32le(d, s + 8),
        journal_block: u32le(d, s + 12),
        block_size,
        state: u16le(d, s + 50),
        hash_function: u32le(d, s + 64),
        tree_height: u16le(d, s + 68),
        bitmap_count: u16le(d, s + 70),
        version: u16le(d, s + 72),
    })
}

/// `true` if the image looks like a ReiserFS volume.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reiserfs() -> Vec<u8> {
        let mut f = vec![0u8; SB_OFFSET + 0x100];
        let s = SB_OFFSET;
        f[s..s + 4].copy_from_slice(&100000u32.to_le_bytes());
        f[s + 4..s + 8].copy_from_slice(&99900u32.to_le_bytes());
        f[s + 8..s + 12].copy_from_slice(&8374u32.to_le_bytes());
        f[s + 12..s + 16].copy_from_slice(&8198u32.to_le_bytes());
        f[s + 44..s + 46].copy_from_slice(&4096u16.to_le_bytes());
        f[s + 50..s + 52].copy_from_slice(&1u16.to_le_bytes());
        f[s + 52..s + 61].copy_from_slice(b"ReIsEr2Fs");
        f[s + 64..s + 68].copy_from_slice(&5u32.to_le_bytes());
        f[s + 68..s + 70].copy_from_slice(&3u16.to_le_bytes());
        f[s + 70..s + 72].copy_from_slice(&17u16.to_le_bytes());
        f[s + 72..s + 74].copy_from_slice(&2u16.to_le_bytes());
        f
    }

    #[test]
    fn parses() {
        let r = parse(&reiserfs()).unwrap();
        assert_eq!(r.format, 2);
        assert_eq!(r.block_count, 100000);
        assert_eq!(r.free_blocks, 99900);
        assert_eq!(r.root_block, 8374);
        assert_eq!(r.journal_block, 8198);
        assert_eq!(r.block_size, 4096);
        assert_eq!(r.state, 1);
        assert_eq!(r.hash_function, 5);
        assert_eq!(r.tree_height, 3);
        assert_eq!(r.bitmap_count, 17);
        assert_eq!(r.version, 2);
    }

    #[test]
    fn v1_magic() {
        let mut f = reiserfs();
        let s = SB_OFFSET;
        f[s + 52..s + 64].fill(0);
        f[s + 52..s + 60].copy_from_slice(b"ReIsErFs");
        assert_eq!(parse(&f).unwrap().format, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&vec![0u8; SB_OFFSET + 0x100]).is_none());
        let mut f = reiserfs();
        f[SB_OFFSET + 52] = b'X';
        assert!(parse(&f).is_none());
        // free > total is inconsistent
        let mut g = reiserfs();
        g[SB_OFFSET + 4..SB_OFFSET + 8].copy_from_slice(&200000u32.to_le_bytes());
        assert!(parse(&g).is_none());
        // absurd block size
        let mut h = reiserfs();
        h[SB_OFFSET + 44..SB_OFFSET + 46].copy_from_slice(&123u16.to_le_bytes());
        assert!(parse(&h).is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(&reiserfs()));
        assert!(!detect(b"ReIsErFs"));
    }
}
