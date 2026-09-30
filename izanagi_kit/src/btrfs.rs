//! Btrfs superblock scanner.
//!
//! The Btrfs superblock lives at byte offset `0x10000` of the device and
//! begins with a 32-byte CRC, then `fsid`, `bytenr`, `flags`, and the magic
//! `_BHRfS_M`. Relevant scalar fields follow: `generation`,
//! `sys_chunk_array_size`, `num_devices`, `sectorsize`, `nodesize`,
//! `leafsize` (alias of nodesize), `stripesize`.
//!
//! ```
//! let mut f = vec![0u8; 0x10000 + 0x1000];
//! let s = 0x10000;
//! f[s + 0x40..s + 0x48].copy_from_slice(b"_BHRfS_M");
//! f[s + 0x48..s + 0x50].copy_from_slice(&7u64.to_le_bytes()); // generation
//! f[s + 0x88..s + 0x90].copy_from_slice(&5u64.to_le_bytes()); // num_devices
//! f[s + 0x90..s + 0x94].copy_from_slice(&4096u32.to_le_bytes()); // sectorsize
//! f[s + 0x94..s + 0x98].copy_from_slice(&16384u32.to_le_bytes()); // nodesize
//! let b = izanagi_kit::btrfs::parse(&f).unwrap();
//! assert_eq!(b.num_devices, 5);
//! assert_eq!(b.sector_size, 4096);
//! ```
//!
//! Reference: btrfs.readthedocs.io "On-disk format" — superblock at 64 KiB,
//! magic `_BHRfS_M` at offset `0x10040`.

/// Parsed Btrfs superblock fields.
#[derive(Debug, Clone, PartialEq)]
pub struct Btrfs {
    /// Filesystem UUID (16 bytes).
    pub fsid: [u8; 16],
    /// Current tree generation.
    pub generation: u64,
    /// Number of devices in the filesystem.
    pub num_devices: u64,
    /// Sector size in bytes.
    pub sector_size: u32,
    /// B-tree node size in bytes.
    pub node_size: u32,
    /// Leaf size (legacy alias of node size).
    pub leaf_size: u32,
    /// Stripe size in bytes.
    pub stripe_size: u32,
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

/// Parse a device image; `None` if too small or the magic is absent.
pub fn parse(d: &[u8]) -> Option<Btrfs> {
    const SB: usize = 0x10000;
    if d.len() < SB + 0xA0 {
        return None;
    }
    if &d[SB + 0x40..SB + 0x48] != b"_BHRfS_M" {
        return None;
    }
    let mut fsid = [0u8; 16];
    fsid.copy_from_slice(&d[SB + 0x20..SB + 0x30]);
    let sector_size = u32le(d, SB + 0x90);
    let node_size = u32le(d, SB + 0x94);
    // Sector sizes must be a sane power of two.
    if !sector_size.is_power_of_two() || sector_size < 512 {
        return None;
    }
    Some(Btrfs {
        fsid,
        generation: u64le(d, SB + 0x48),
        num_devices: u64le(d, SB + 0x88),
        sector_size,
        node_size,
        leaf_size: u32le(d, SB + 0x98),
        stripe_size: u32le(d, SB + 0x9C),
    })
}

/// `true` if a byte buffer looks like a Btrfs filesystem.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn btrfs() -> Vec<u8> {
        let mut f = vec![0u8; 0x10000 + 0x200];
        let s = 0x10000;
        for (i, b) in f[s + 0x20..s + 0x30].iter_mut().enumerate() {
            *b = i as u8;
        }
        f[s + 0x40..s + 0x48].copy_from_slice(b"_BHRfS_M");
        f[s + 0x48..s + 0x50].copy_from_slice(&42u64.to_le_bytes());
        f[s + 0x88..s + 0x90].copy_from_slice(&3u64.to_le_bytes());
        f[s + 0x90..s + 0x94].copy_from_slice(&4096u32.to_le_bytes());
        f[s + 0x94..s + 0x98].copy_from_slice(&16384u32.to_le_bytes());
        f[s + 0x98..s + 0x9c].copy_from_slice(&16384u32.to_le_bytes());
        f[s + 0x9c..s + 0xa0].copy_from_slice(&4096u32.to_le_bytes());
        f
    }

    #[test]
    fn parses_fields() {
        let b = parse(&btrfs()).unwrap();
        assert_eq!(b.fsid[0], 0);
        assert_eq!(b.fsid[15], 15);
        assert_eq!(b.generation, 42);
        assert_eq!(b.num_devices, 3);
        assert_eq!(b.node_size, 16384);
        assert_eq!(b.leaf_size, 16384);
        assert_eq!(b.stripe_size, 4096);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&vec![0u8; 0x11000]).is_none());
        let mut f = btrfs();
        f[0x10040] = b'X';
        assert!(parse(&f).is_none());
        // Sector size not a power of two.
        let mut g = btrfs();
        g[0x10090] = 3;
        g[0x10091] = 0;
        g[0x10092] = 0;
        g[0x10093] = 0;
        assert!(parse(&g).is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(&btrfs()));
        assert!(!detect(b"no"));
    }
}
