//! F2FS (Flash-Friendly File System) superblock scanner.
//!
//! The first F2FS superblock sits at byte offset `0x400` of the partition
//! (a second copy lives in the next block). Its first field is the magic
//! `0xF2F52010`. Layout (little-endian):
//!
//! `magic u32`, `major_ver u16`, `minor_ver u16`, `log_sectorsize u32`,
//! `log_sectors_per_block u32`, `log_blocksize u32`, `log_blocks_per_seg u32`,
//! `segs_per_sec u32`, `secs_per_zone u32`, `checksum_offset u32`,
//! `block_count u64`, `section_count u32`, `segment_count u32`,
//! `segment_count_ckpt u32`, `segment_count_sit u32`, `segment_count_nat u32`,
//! `segment_count_ssa u32`, `segment_count_main u32`, `segment0_blkaddr u32`,
//! `cp_blkaddr u32`, `sit_blkaddr u32`, `nat_blkaddr u32`, `ssa_blkaddr u32`,
//! `main_blkaddr u32`, `root_ino u32`, `node_ino u32`, `meta_ino u32` …
//!
//! ```
//! let mut f = vec![0u8; 0x400 + 0x200];
//! let s = 0x400;
//! f[s..s + 4].copy_from_slice(&0xF2F5_2010u32.to_le_bytes());
//! f[s + 4..s + 6].copy_from_slice(&1u16.to_le_bytes());  // major
//! f[s + 8..s + 12].copy_from_slice(&9u32.to_le_bytes()); // log_sectorsize
//! f[s + 0x10..s + 0x14].copy_from_slice(&12u32.to_le_bytes()); // log_blocksize
//! f[s + 0x1C..s + 0x24].copy_from_slice(&0x100000u64.to_le_bytes()); // block_count
//! let x = izanagi_kit::f2fs::parse(&f).unwrap();
//! assert_eq!(x.block_size(), 4096);
//! assert_eq!(x.block_count, 0x100000);
//! ```
//!
//! Reference: `include/linux/f2fs_fs.h` (`F2FS_SUPER_MAGIC 0xF2F52010`,
//! `F2FS_SUPER_OFFSET 0x400`).

/// Parsed F2FS superblock fields.
#[derive(Debug, Clone, PartialEq)]
pub struct F2fs {
    /// Major version number.
    pub major_version: u16,
    /// Minor version number.
    pub minor_version: u16,
    /// `log2` of the sector size (typically 9 → 512 B).
    pub log_sector_size: u32,
    /// `log2` of the block size (typically 12 → 4 KiB).
    pub log_block_size: u32,
    /// `log2` of blocks per segment.
    pub log_blocks_per_seg: u32,
    /// Total number of blocks in the device.
    pub block_count: u64,
    /// Total segment count.
    pub segment_count: u32,
    /// Segments reserved for the checkpoint area.
    pub segment_count_ckpt: u32,
    /// Segments reserved for the SIT area.
    pub segment_count_sit: u32,
    /// Segments reserved for the NAT area.
    pub segment_count_nat: u32,
    /// Segments reserved for the SSA area.
    pub segment_count_ssa: u32,
    /// Segments in the main (data) area.
    pub segment_count_main: u32,
    /// Block address where the checkpoint area starts.
    pub cp_blkaddr: u32,
    /// Root directory inode number.
    pub root_ino: u32,
    /// Node inode number.
    pub node_ino: u32,
}

fn u16le(d: &[u8], off: usize) -> u16 {
    d[off] as u16 | ((d[off + 1] as u16) << 8)
}

fn u32le(d: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([d[off], d[off + 1], d[off + 2], d[off + 3]])
}

fn u64le(d: &[u8], off: usize) -> u64 {
    u64::from_le_bytes([
        d[off],
        d[off + 1],
        d[off + 2],
        d[off + 3],
        d[off + 4],
        d[off + 5],
        d[off + 6],
        d[off + 7],
    ])
}

impl F2fs {
    /// Block size in bytes (`1 << log_block_size`).
    pub fn block_size(&self) -> u64 {
        1u64 << self.log_block_size
    }
}

/// Parse a partition image; `None` if the magic is absent or implausible.
pub fn parse(d: &[u8]) -> Option<F2fs> {
    const SB: usize = 0x400;
    if d.len() < SB + 0x60 {
        return None;
    }
    if u32le(d, SB) != 0xF2F5_2010 {
        return None;
    }
    let log_sector_size = u32le(d, SB + 0x08);
    let log_block_size = u32le(d, SB + 0x10);
    // sane bounds: 512 B – 64 KiB sectors, 1 KiB – 64 KiB blocks
    if !(9..=16).contains(&log_sector_size) || !(10..=16).contains(&log_block_size) {
        return None;
    }
    Some(F2fs {
        major_version: u16le(d, SB + 0x04),
        minor_version: u16le(d, SB + 0x06),
        log_sector_size,
        log_block_size,
        log_blocks_per_seg: u32le(d, SB + 0x14),
        block_count: u64le(d, SB + 0x1C),
        segment_count: u32le(d, SB + 0x28),
        segment_count_ckpt: u32le(d, SB + 0x2C),
        segment_count_sit: u32le(d, SB + 0x30),
        segment_count_nat: u32le(d, SB + 0x34),
        segment_count_ssa: u32le(d, SB + 0x38),
        segment_count_main: u32le(d, SB + 0x3C),
        cp_blkaddr: u32le(d, SB + 0x44),
        root_ino: u32le(d, SB + 0x58),
        node_ino: u32le(d, SB + 0x5C),
    })
}

/// `true` if the image looks like an F2FS filesystem.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f2fs() -> Vec<u8> {
        let mut f = vec![0u8; 0x400 + 0x200];
        let s = 0x400;
        f[s..s + 4].copy_from_slice(&0xF2F5_2010u32.to_le_bytes());
        f[s + 4..s + 6].copy_from_slice(&1u16.to_le_bytes());
        f[s + 6..s + 8].copy_from_slice(&0u16.to_le_bytes());
        f[s + 8..s + 12].copy_from_slice(&9u32.to_le_bytes());
        f[s + 0x10..s + 0x14].copy_from_slice(&12u32.to_le_bytes());
        f[s + 0x14..s + 0x18].copy_from_slice(&8u32.to_le_bytes());
        f[s + 0x1C..s + 0x24].copy_from_slice(&0x40000u64.to_le_bytes());
        f[s + 0x28..s + 0x2C].copy_from_slice(&1024u32.to_le_bytes());
        f[s + 0x2C..s + 0x30].copy_from_slice(&2u32.to_le_bytes());
        f[s + 0x30..s + 0x34].copy_from_slice(&6u32.to_le_bytes());
        f[s + 0x34..s + 0x38].copy_from_slice(&20u32.to_le_bytes());
        f[s + 0x38..s + 0x3C].copy_from_slice(&40u32.to_le_bytes());
        f[s + 0x3C..s + 0x40].copy_from_slice(&960u32.to_le_bytes());
        f[s + 0x44..s + 0x48].copy_from_slice(&0x200u32.to_le_bytes());
        f[s + 0x58..s + 0x5C].copy_from_slice(&3u32.to_le_bytes());
        f[s + 0x5C..s + 0x60].copy_from_slice(&1u32.to_le_bytes());
        f
    }

    #[test]
    fn parses_fields() {
        let x = parse(&f2fs()).unwrap();
        assert_eq!(x.major_version, 1);
        assert_eq!(x.log_sector_size, 9);
        assert_eq!(x.log_block_size, 12);
        assert_eq!(x.block_size(), 4096);
        assert_eq!(x.block_count, 0x40000);
        assert_eq!(x.segment_count, 1024);
        assert_eq!(x.segment_count_main, 960);
        assert_eq!(x.cp_blkaddr, 0x200);
        assert_eq!(x.root_ino, 3);
        assert_eq!(x.node_ino, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&vec![0u8; 0x600]).is_none());
        let mut f = f2fs();
        f[0x400] = 0;
        assert!(parse(&f).is_none());
        // implausible sector size
        let mut g = f2fs();
        g[0x408] = 3;
        assert!(parse(&g).is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(&f2fs()));
        assert!(!detect(b"mkfs"));
    }
}
