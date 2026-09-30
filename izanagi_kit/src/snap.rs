//! snapd `.snap` package: a SquashFS image whose `meta/snap.yaml`
//! describes the snap. The superblock is the 96-byte LE header at
//! offset 0 (`hsqs` magic).
//!
//! ```
//! let mut f = vec![0u8; 128];
//! f[..4].copy_from_slice(b"hsqs");
//! f[4..8].copy_from_slice(&7u32.to_le_bytes()); // inodes
//! f[12..16].copy_from_slice(&(4096u32).to_le_bytes()); // block size
//! f[20..22].copy_from_slice(&1u16.to_le_bytes()); // gzip
//! f[22..24].copy_from_slice(&12u16.to_le_bytes()); // block log
//! f[28..30].copy_from_slice(&4u16.to_le_bytes()); // major
//! f[40..48].copy_from_slice(&128u64.to_le_bytes()); // bytes used
//! f.extend_from_slice(b"garbage meta/snap.yaml tail");
//! let s = izanagi_kit::snap::parse(&f).unwrap();
//! assert_eq!(s.inodes, 7);
//! assert_eq!(s.compression, 1);
//! assert!(s.looks_like_snap);
//! ```

/// SquashFS `hsqs` magic (little-endian).
pub const MAGIC: u32 = 0x7371_7368;
/// Superblock size in bytes.
pub const SUPERBLOCK: usize = 96;

fn u16l(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(d.get(at..at + 2)?.try_into().ok()?))
}

fn u32l(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

fn u64l(d: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(d.get(at..at + 8)?.try_into().ok()?))
}

/// Compression back-end id stored in the superblock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Compression {
    /// gzip (id 1).
    Gzip,
    /// lzma (id 2).
    Lzma,
    /// lzo (id 3).
    Lzo,
    /// xz (id 4).
    Xz,
    /// lz4 (id 5).
    Lz4,
    /// zstd (id 6).
    Zstd,
    /// Any other id.
    Other(u16),
}

impl Compression {
    /// Classify a raw id.
    pub fn from_u16(v: u16) -> Self {
        match v {
            1 => Compression::Gzip,
            2 => Compression::Lzma,
            3 => Compression::Lzo,
            4 => Compression::Xz,
            5 => Compression::Lz4,
            6 => Compression::Zstd,
            other => Compression::Other(other),
        }
    }
}

/// A snap package's SquashFS superblock view.
#[derive(Clone, Debug)]
pub struct Snap {
    /// Inode count.
    pub inodes: u32,
    /// Filesystem creation timestamp.
    pub mkfs_time: u32,
    /// Data block size (4KiB–1MiB, power of two).
    pub block_size: u32,
    /// Fragment entry count.
    pub fragments: u32,
    /// Compression back-end id as stored.
    pub compression: u16,
    /// `block_size == 1 << block_log` on valid images.
    pub block_log: u16,
    /// Superblock flags.
    pub flags: u16,
    /// uid/gid table size.
    pub no_ids: u16,
    /// Format major/minor (4.0 for SquashFS 4).
    pub major: u16,
    /// Format minor.
    pub minor: u16,
    /// Packed root inode reference.
    pub root_inode: u64,
    /// Total image size in bytes.
    pub bytes_used: u64,
    /// Whether the byte stream contains a `meta/snap.yaml` reference —
    /// the heuristic distinguishing a snap from a generic SquashFS.
    pub looks_like_snap: bool,
}

impl Snap {
    /// Compression back-end classified.
    pub fn codec(&self) -> Compression {
        Compression::from_u16(self.compression)
    }
}

/// Parse a snap/SquashFS superblock. Returns `None` on bad magic, a
/// non-4.0 version, an implausible `block_size`, or a `block_log`
/// that disagrees with it.
pub fn parse(d: &[u8]) -> Option<Snap> {
    if u32l(d, 0)? != MAGIC {
        return None;
    }
    let block_size = u32l(d, 12)?;
    let block_log = u16l(d, 22)?;
    if !(4096..=1_048_576).contains(&block_size) || !block_size.is_power_of_two() {
        return None;
    }
    if block_size != 1u32.checked_shl(u32::from(block_log))? {
        return None;
    }
    let major = u16l(d, 28)?;
    let minor = u16l(d, 30)?;
    if major != 4 || minor != 0 {
        return None;
    }
    let bytes_used = u64l(d, 40)?;
    if usize::try_from(bytes_used).ok()? > d.len() {
        return None;
    }
    let marker: &[u8] = b"meta/snap.yaml";
    let looks_like_snap = d.windows(marker.len()).any(|w| w == marker);
    Some(Snap {
        inodes: u32l(d, 4)?,
        mkfs_time: u32l(d, 8)?,
        block_size,
        fragments: u32l(d, 16)?,
        compression: u16l(d, 20)?,
        block_log,
        flags: u16l(d, 24)?,
        no_ids: u16l(d, 26)?,
        major,
        minor,
        root_inode: u64l(d, 32)?,
        bytes_used,
        looks_like_snap,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut f = vec![0u8; 128];
        f[..4].copy_from_slice(b"hsqs");
        f[4..8].copy_from_slice(&7u32.to_le_bytes());
        f[12..16].copy_from_slice(&4096u32.to_le_bytes());
        f[20..22].copy_from_slice(&6u16.to_le_bytes());
        f[22..24].copy_from_slice(&12u16.to_le_bytes());
        f[28..30].copy_from_slice(&4u16.to_le_bytes());
        f[40..48].copy_from_slice(&128u64.to_le_bytes());
        f
    }

    #[test]
    fn superblock_fields() {
        let s = parse(&fixture()).unwrap();
        assert_eq!(s.inodes, 7);
        assert_eq!(s.block_size, 4096);
        assert_eq!(s.compression, 6);
        assert_eq!(s.codec(), Compression::Zstd);
        assert_eq!(s.major, 4);
        assert!(!s.looks_like_snap);
    }

    #[test]
    fn marker_detection() {
        let mut f = fixture();
        f.extend_from_slice(b"junk meta/snap.yaml here");
        let s = parse(&f).unwrap();
        assert!(s.looks_like_snap);
        assert_eq!(Compression::from_u16(4), Compression::Xz);
        assert_eq!(Compression::from_u16(9), Compression::Other(9));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"sqsh").is_none()); // big-endian magic rejected
        let mut f = fixture();
        f[12] = 0; // block_size = 4096 -> 0x1000; zero the low byte of 4096? make it 0
        f[13] = 0; // block_size now 0
        assert!(parse(&f).is_none());
        let mut g = fixture();
        g[28] = 3; // major 3
        assert!(parse(&g).is_none());
        let mut h = fixture();
        h[22] = 9; // block_log 9 -> 512 != 4096
        assert!(parse(&h).is_none());
    }
}
