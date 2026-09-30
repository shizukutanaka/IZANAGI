//! JFS (IBM/OS-2 Journaling File System, JFS1) superblock scanner.
//!
//! The primary superblock sits at byte offset `0x8000` (32 KiB) of the
//! aggregate and begins with `"JFS1"`. All fields are little-endian:
//!
//! `s_magic[4]` \@0, `s_version u32` \@4, `s_size u64` \@8 (aggregate
//! size in physical blocks), `s_bsize u32` \@16, `s_l2bsize u16` \@20,
//! `s_l2bfactor u16` \@22, `s_pbsize u16` \@24, `s_l2pbsize u16` \@26,
//! `pad u16` \@28, `s_agsize u32` \@32, `s_flag u32` \@36,
//! `s_state u32` \@40 (`0` = clean, `1` = mounted, `2` = dirty),
//! `s_compress u32` \@44, then `s_uuid[16]` \@88 and `s_label[16]` \@104.
//!
//! ```
//! let mut f = vec![0u8; 0x8000 + 0x200];
//! let s = 0x8000;
//! f[s..s + 4].copy_from_slice(b"JFS1");
//! f[s + 4..s + 8].copy_from_slice(&1u32.to_le_bytes());        // version
//! f[s + 8..s + 16].copy_from_slice(&0x8000u64.to_le_bytes());  // s_size
//! f[s + 16..s + 20].copy_from_slice(&4096u32.to_le_bytes());   // s_bsize
//! f[s + 20..s + 22].copy_from_slice(&12u16.to_le_bytes());     // s_l2bsize
//! f[s + 24..s + 26].copy_from_slice(&512u16.to_le_bytes());    // s_pbsize
//! f[s + 26..s + 28].copy_from_slice(&9u16.to_le_bytes());      // s_l2pbsize
//! f[s + 104..s + 108].copy_from_slice(b"jfs1");                // label
//! let j = izanagi_kit::jfs::parse(&f).unwrap();
//! assert_eq!(j.block_size, 4096);
//! assert_eq!(j.label.as_deref(), Some("jfs1"));
//! ```
//!
//! Reference: Linux `fs/jfs/jfs_superblock.h` (`JFS_MAGIC "JFS1"`,
//! `PSIZE`/`sb` at aggregate offset `0x8000`).

/// Parsed JFS superblock fields.
#[derive(Debug, Clone, PartialEq)]
pub struct Jfs {
    /// Filesystem layout version (`s_version`).
    pub version: u32,
    /// Aggregate size counted in physical blocks (`s_size`).
    pub size: u64,
    /// Logical block size in bytes (`s_bsize`).
    pub block_size: u32,
    /// `log2` of `block_size` (`s_l2bsize`).
    pub log_block_size: u16,
    /// Physical block size in bytes (`s_pbsize`).
    pub phys_block_size: u16,
    /// Allocation group size in aggregate blocks (`s_agsize`).
    pub ag_size: u32,
    /// Aggregate flags (`s_flag`).
    pub flags: u32,
    /// Mount state (`s_state`): 0 clean, 1 mounted, 2 dirty.
    pub state: u32,
    /// Filesystem UUID.
    pub uuid: [u8; 16],
    /// Volume label (`s_label`, NUL-padded, up to 16 bytes).
    pub label: Option<String>,
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

/// Parse a device image; `None` if the `JFS1` magic is absent or fields
/// are implausible.
pub fn parse(d: &[u8]) -> Option<Jfs> {
    const SB: usize = 0x8000;
    if d.len() < SB + 0x80 {
        return None;
    }
    if &d[SB..SB + 4] != b"JFS1" {
        return None;
    }
    let block_size = u32le(d, SB + 16);
    let log_block_size = u16le(d, SB + 20);
    let phys_block_size = u16le(d, SB + 24);
    // block size must be a power of two ≥ physical block size.
    if !block_size.is_power_of_two() || block_size < 512 {
        return None;
    }
    if !phys_block_size.is_power_of_two() || phys_block_size < 512 {
        return None;
    }
    if (1u64 << log_block_size) as u32 != block_size {
        return None;
    }
    let mut uuid = [0u8; 16];
    uuid.copy_from_slice(&d[SB + 88..SB + 104]);
    let label_len = d[SB + 104..SB + 120]
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(16);
    let label = if label_len == 0 {
        None
    } else {
        core::str::from_utf8(&d[SB + 104..SB + 104 + label_len])
            .ok()
            .map(|s| s.to_string())
    };
    Some(Jfs {
        version: u32le(d, SB + 4),
        size: u64le(d, SB + 8),
        block_size,
        log_block_size,
        phys_block_size,
        ag_size: u32le(d, SB + 32),
        flags: u32le(d, SB + 36),
        state: u32le(d, SB + 40),
        uuid,
        label,
    })
}

/// `true` if the image looks like a JFS filesystem.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jfs() -> Vec<u8> {
        let mut f = vec![0u8; 0x8000 + 0x200];
        let s = 0x8000;
        f[s..s + 4].copy_from_slice(b"JFS1");
        f[s + 4..s + 8].copy_from_slice(&1u32.to_le_bytes());
        f[s + 8..s + 16].copy_from_slice(&0x8000u64.to_le_bytes());
        f[s + 16..s + 20].copy_from_slice(&4096u32.to_le_bytes());
        f[s + 20..s + 22].copy_from_slice(&12u16.to_le_bytes());
        f[s + 24..s + 26].copy_from_slice(&512u16.to_le_bytes());
        f[s + 26..s + 28].copy_from_slice(&9u16.to_le_bytes());
        f[s + 32..s + 36].copy_from_slice(&8192u32.to_le_bytes());
        f[s + 36..s + 40].copy_from_slice(&0x4000_0044u32.to_le_bytes());
        f[s + 40..s + 44].copy_from_slice(&1u32.to_le_bytes());
        f[s + 88..s + 91].copy_from_slice(&[0xAA, 0xBB, 0xCC]);
        f[s + 104..s + 108].copy_from_slice(b"jfs1");
        f
    }

    #[test]
    fn parses() {
        let j = parse(&jfs()).unwrap();
        assert_eq!(j.version, 1);
        assert_eq!(j.size, 0x8000);
        assert_eq!(j.block_size, 4096);
        assert_eq!(j.log_block_size, 12);
        assert_eq!(j.phys_block_size, 512);
        assert_eq!(j.ag_size, 8192);
        assert_eq!(j.state, 1);
        assert_eq!(j.uuid[0], 0xAA);
        assert_eq!(j.label.as_deref(), Some("jfs1"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&vec![0u8; 0x8200]).is_none());
        let mut f = jfs();
        f[0x8000] = b'X';
        assert!(parse(&f).is_none());
        // l2bsize inconsistent with bsize
        let mut g = jfs();
        g[0x8000 + 20] = 11;
        assert!(parse(&g).is_none());
        // non-power-of-two block size
        let mut h = jfs();
        h[0x8000 + 16] = 100;
        h[0x8000 + 17] = 0;
        assert!(parse(&h).is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(&jfs()));
        assert!(!detect(b"JFS1"));
    }
}
