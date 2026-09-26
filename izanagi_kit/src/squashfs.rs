//! SquashFS superblock (read-only compressed filesystem).
//!
//! The 96-byte v4 superblock starts with the `hsqs` magic
//! (`0x73717368` little-endian), then counts (inodes, fragments,
//! ids), geometry (`block_size`, `block_log`), the compression id
//! (gzip/lzma/lzo/xz/lz4/zstd), flag bits (uncompressed / no-fragments
//! / exportable / ...), the root inode reference, `bytes_used`, and
//! the start offsets of the id, xattr, inode, directory, fragment and
//! export tables.
//!
//! ```
//! use izanagi_kit::squashfs::{parse, Compression, FLAG_NO_FRAGMENTS};
//!
//! let mut d = vec![0u8; 96];
//! d[..4].copy_from_slice(&0x73717368u32.to_le_bytes());
//! d[4..8].copy_from_slice(&100u32.to_le_bytes());      // inodes
//! d[12..16].copy_from_slice(&131072u32.to_le_bytes()); // 128 KiB blocks
//! d[20..22].copy_from_slice(&1u16.to_le_bytes());      // gzip
//! d[24..26].copy_from_slice(&(FLAG_NO_FRAGMENTS as u16).to_le_bytes());
//! d[28..30].copy_from_slice(&4u16.to_le_bytes());      // major
//! let s = parse(&d).unwrap();
//! assert_eq!(s.inodes, 100);
//! assert_eq!(s.compression, Compression::Gzip);
//! assert!(s.flag(FLAG_NO_FRAGMENTS));
//! assert_eq!(s.block_size, 131072);
//! ```

/// Superblock size in bytes.
pub const SUPERBLOCK: usize = 96;
/// Little-endian magic (`"hsqs"`).
pub const MAGIC: u32 = 0x7371_7368;
/// Inodes stored uncompressed.
pub const FLAG_INODES_RAW: u32 = 0x0001;
/// Data blocks stored uncompressed.
pub const FLAG_DATA_RAW: u32 = 0x0002;
/// Fragment entries stored uncompressed.
pub const FLAG_FRAGS_RAW: u32 = 0x0008;
/// No fragment table (all tails inline-packed as whole blocks).
pub const FLAG_NO_FRAGMENTS: u32 = 0x0010;
/// Every file tail is forced into a fragment block.
pub const FLAG_ALWAYS_FRAGMENTS: u32 = 0x0020;
/// Duplicate data blocks are kept.
pub const FLAG_DUPLICATES: u32 = 0x0040;
/// Exportable via NFS (export table present).
pub const FLAG_EXPORTABLE: u32 = 0x0080;
/// Xattr entries stored uncompressed.
pub const FLAG_XATTRS_RAW: u32 = 0x0100;
/// No xattr table.
pub const FLAG_NO_XATTRS: u32 = 0x0200;
/// Compressor options block present.
pub const FLAG_COMP_OPTS: u32 = 0x0400;
/// Id table stored uncompressed.
pub const FLAG_IDS_RAW: u32 = 0x0800;

fn u16le(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from(*d.get(at)?) | u16::from(*d.get(at + 1)?) << 8)
}

fn u32le(d: &[u8], at: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(at)?)
            | u32::from(*d.get(at + 1)?) << 8
            | u32::from(*d.get(at + 2)?) << 16
            | u32::from(*d.get(at + 3)?) << 24,
    )
}

fn u64le(d: &[u8], at: usize) -> Option<u64> {
    Some(u64::from(u32le(d, at)?) | u64::from(u32le(d, at + 4)?) << 32)
}

/// Compressor identifier.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Compression {
    /// gzip/deflate (id 1) — the default.
    Gzip,
    /// LZMA (id 2).
    Lzma,
    /// LZO (id 3).
    Lzo,
    /// XZ (id 4).
    Xz,
    /// LZ4 (id 5).
    Lz4,
    /// Zstd (id 6).
    Zstd,
    /// Any other id.
    Other(u16),
}

impl Compression {
    fn of(v: u16) -> Self {
        match v {
            1 => Compression::Gzip,
            2 => Compression::Lzma,
            3 => Compression::Lzo,
            4 => Compression::Xz,
            5 => Compression::Lz4,
            6 => Compression::Zstd,
            v => Compression::Other(v),
        }
    }
}

/// A parsed SquashFS v4 superblock.
#[derive(Clone, Debug, PartialEq)]
pub struct Squashfs {
    /// Number of inodes in the image.
    pub inodes: u32,
    /// Filesystem creation time (seconds since epoch).
    pub mkfs_time: u32,
    /// Data block size in bytes (a power of two between 4 KiB and 1 MiB).
    pub block_size: u32,
    /// Number of fragments.
    pub fragments: u32,
    /// Compressor in use.
    pub compression: Compression,
    /// log2 of `block_size`.
    pub block_log: u16,
    /// Flag word (`FLAG_*` constants).
    pub flags: u16,
    /// Number of ids in the id table.
    pub id_count: u16,
    /// Format major version (4 for v4 images).
    pub major: u16,
    /// Format minor version.
    pub minor: u16,
    /// Root inode reference (`(block << 16) | offset` packed in 64 bits).
    pub root_inode: u64,
    /// Total bytes consumed by the image.
    pub bytes_used: u64,
    /// Byte offset of the id lookup table.
    pub id_table_start: u64,
    /// Byte offset of the xattr id table (or `0xFFFF...` when absent).
    pub xattr_table_start: u64,
    /// Byte offset of the inode table.
    pub inode_table_start: u64,
    /// Byte offset of the directory table.
    pub dir_table_start: u64,
    /// Byte offset of the fragment table (or `0xFFFF...` when absent).
    pub fragment_table_start: u64,
    /// Byte offset of the export table (or `0xFFFF...` when absent).
    pub export_table_start: u64,
}

impl Squashfs {
    /// Test a flag bit (`FLAG_*` constant).
    pub fn flag(&self, bit: u32) -> bool {
        u32::from(self.flags) & bit != 0
    }
}

/// Parse a SquashFS superblock. Returns `None` on a bad magic or a
/// buffer shorter than 96 bytes.
pub fn parse(d: &[u8]) -> Option<Squashfs> {
    if u32le(d, 0)? != MAGIC {
        return None;
    }
    Some(Squashfs {
        inodes: u32le(d, 4)?,
        mkfs_time: u32le(d, 8)?,
        block_size: u32le(d, 12)?,
        fragments: u32le(d, 16)?,
        compression: Compression::of(u16le(d, 20)?),
        block_log: u16le(d, 22)?,
        flags: u16le(d, 24)?,
        id_count: u16le(d, 26)?,
        major: u16le(d, 28)?,
        minor: u16le(d, 30)?,
        root_inode: u64le(d, 32)?,
        bytes_used: u64le(d, 40)?,
        id_table_start: u64le(d, 48)?,
        xattr_table_start: u64le(d, 56)?,
        inode_table_start: u64le(d, 64)?,
        dir_table_start: u64le(d, 72)?,
        fragment_table_start: u64le(d, 80)?,
        export_table_start: u64le(d, 88)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn image() -> Vec<u8> {
        let mut d = vec![0u8; SUPERBLOCK];
        d[..4].copy_from_slice(&MAGIC.to_le_bytes());
        d[4..8].copy_from_slice(&1234u32.to_le_bytes());
        d[8..12].copy_from_slice(&0x6500_0000u32.to_le_bytes());
        d[12..16].copy_from_slice(&131072u32.to_le_bytes());
        d[16..20].copy_from_slice(&56u32.to_le_bytes());
        d[20..22].copy_from_slice(&4u16.to_le_bytes());
        d[22..24].copy_from_slice(&17u16.to_le_bytes());
        d[24..26].copy_from_slice(&((FLAG_EXPORTABLE | FLAG_NO_XATTRS) as u16).to_le_bytes());
        d[26..28].copy_from_slice(&2u16.to_le_bytes());
        d[28..30].copy_from_slice(&4u16.to_le_bytes());
        d[30..32].copy_from_slice(&0u16.to_le_bytes());
        d[32..40].copy_from_slice(&0x0001_0000_0042u64.to_le_bytes());
        d[40..48].copy_from_slice(&9999u64.to_le_bytes());
        d[48..56].copy_from_slice(&8000u64.to_le_bytes());
        d[56..64].copy_from_slice(&u64::MAX.to_le_bytes());
        d[64..72].copy_from_slice(&2000u64.to_le_bytes());
        d[72..80].copy_from_slice(&5000u64.to_le_bytes());
        d[80..88].copy_from_slice(&7000u64.to_le_bytes());
        d[88..96].copy_from_slice(&9000u64.to_le_bytes());
        d
    }

    #[test]
    fn parse_reads_every_field() {
        let s = parse(&image()).unwrap();
        assert_eq!(s.inodes, 1234);
        assert_eq!(s.mkfs_time, 0x6500_0000);
        assert_eq!(s.block_size, 131072);
        assert_eq!(s.fragments, 56);
        assert_eq!(s.compression, Compression::Xz);
        assert_eq!(s.block_log, 17);
        assert!(s.flag(FLAG_EXPORTABLE));
        assert!(s.flag(FLAG_NO_XATTRS));
        assert!(!s.flag(FLAG_NO_FRAGMENTS));
        assert_eq!(s.id_count, 2);
        assert_eq!(s.major, 4);
        assert_eq!(s.minor, 0);
        assert_eq!(s.root_inode, 0x0001_0000_0042);
        assert_eq!(s.bytes_used, 9999);
        assert_eq!(s.id_table_start, 8000);
        assert_eq!(s.xattr_table_start, u64::MAX);
        assert_eq!(s.inode_table_start, 2000);
        assert_eq!(s.dir_table_start, 5000);
        assert_eq!(s.fragment_table_start, 7000);
        assert_eq!(s.export_table_start, 9000);
    }

    #[test]
    fn compression_ids() {
        assert_eq!(Compression::of(1), Compression::Gzip);
        assert_eq!(Compression::of(2), Compression::Lzma);
        assert_eq!(Compression::of(3), Compression::Lzo);
        assert_eq!(Compression::of(5), Compression::Lz4);
        assert_eq!(Compression::of(6), Compression::Zstd);
        assert_eq!(Compression::of(9), Compression::Other(9));
    }

    #[test]
    fn rejects() {
        assert_eq!(parse(&[0u8; 8]), None);
        let mut d = image();
        d[0] = 0x69;
        assert_eq!(parse(&d), None);
    }

    #[test]
    fn constants() {
        assert_eq!(SUPERBLOCK, 96);
        assert_eq!(MAGIC, 0x7371_7368);
        assert_eq!(FLAG_INODES_RAW | FLAG_DATA_RAW, 0x0003);
        assert_eq!(FLAG_FRAGS_RAW, 0x0008);
        assert_eq!(FLAG_ALWAYS_FRAGMENTS, 0x0020);
        assert_eq!(FLAG_DUPLICATES, 0x0040);
        assert_eq!(FLAG_XATTRS_RAW, 0x0100);
        assert_eq!(FLAG_COMP_OPTS, 0x0400);
        assert_eq!(FLAG_IDS_RAW, 0x0800);
    }
}
