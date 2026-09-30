//! ZFS uberblock scanner.
//!
//! Every vdev carries an uberblock array starting at byte offset `0x20000`
//! (128 KiB, after the 16 KiB vdev label ring and the boot region). Each
//! uberblock is 1 KiB and its first word is `0x00BAB10C` ("ba block") for
//! big-endian images or `0x0CB1BA00` for little-endian; byte order is
//! detected from that word.
//!
//! Layout (little-endian after detection):
//! `magic u64`, `version u64`, `txg u64`, `guid_sum u64`,
//! `timestamp u64`, `rootbp (blkptr 128 B)`.
//!
//! This parser scans the uberblock array (up to `UBERBLOCK_COUNT` slots)
//! and returns the one with the highest transaction group.
//!
//! ```
//! let mut f = vec![0u8; 0x20000 + 1024 * 4];
//! let u = 0x20000;
//! f[u..u + 8].copy_from_slice(&0x00BA_B10Cu64.to_le_bytes()); // magic
//! f[u + 8..u + 16].copy_from_slice(&1u64.to_le_bytes()); // version
//! f[u + 16..u + 24].copy_from_slice(&99u64.to_le_bytes()); // txg
//! let z = izanagi_kit::zfs::parse(&f).unwrap();
//! assert_eq!(z.txg, 99);
//! assert!(!z.big_endian);
//! ```
//!
//! Reference: OpenZFS on-disk format (`sys/fs/zfs.h`, `uberblock_impl`),
//! magic `UBERBLOCK_MAGIC = 0x00bab10c`.

/// Number of 1 KiB uberblock slots scanned per vdev (128 in the spec).
pub const UBERBLOCK_COUNT: usize = 128;

/// Byte offset of the first uberblock within a vdev.
pub const UBERBLOCK_OFFSET: usize = 0x20000;

/// A parsed ZFS uberblock (the newest one in the scanned array).
#[derive(Debug, Clone, PartialEq)]
pub struct Zfs {
    /// `true` when the magic was stored big-endian (rare, older Solaris images).
    pub big_endian: bool,
    /// SPA format version.
    pub version: u64,
    /// Transaction group of this uberblock.
    pub txg: u64,
    /// Sum of pool guid + vdev guid; cheap consistency check.
    pub guid_sum: u64,
    /// UTC timestamp (seconds) when this txg was written.
    pub timestamp: u64,
    /// Slot index inside the uberblock array (0-based).
    pub slot: usize,
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

fn u64be(d: &[u8], off: usize) -> u64 {
    ((d[off] as u64) << 56)
        | ((d[off + 1] as u64) << 48)
        | ((d[off + 2] as u64) << 40)
        | ((d[off + 3] as u64) << 32)
        | ((d[off + 4] as u64) << 24)
        | ((d[off + 5] as u64) << 16)
        | ((d[off + 6] as u64) << 8)
        | d[off + 7] as u64
}

/// Parse a vdev image; `None` if no valid uberblock exists.
///
/// `limit` caps how many uberblock slots are examined so tests can feed a
/// truncated image.
pub fn parse(d: &[u8]) -> Option<Zfs> {
    if d.len() < UBERBLOCK_OFFSET + 1024 {
        return None;
    }
    let avail = (d.len() - UBERBLOCK_OFFSET) / 1024;
    let slots = avail.min(UBERBLOCK_COUNT);
    let mut best: Option<Zfs> = None;
    for i in 0..slots {
        let o = UBERBLOCK_OFFSET + i * 1024;
        let magic_le = u64le(d, o);
        let (be, version, txg, guid_sum, timestamp) = if magic_le == 0x00BA_B10C {
            (
                false,
                u64le(d, o + 8),
                u64le(d, o + 16),
                u64le(d, o + 24),
                u64le(d, o + 32),
            )
        } else if u64be(d, o) == 0x00BA_B10C {
            (
                true,
                u64be(d, o + 8),
                u64be(d, o + 16),
                u64be(d, o + 24),
                u64be(d, o + 32),
            )
        } else {
            continue;
        };
        // version 0 and all-zero payloads are not plausible uberblocks.
        if version == 0 || txg == 0 {
            continue;
        }
        let cand = Zfs {
            big_endian: be,
            version,
            txg,
            guid_sum,
            timestamp,
            slot: i,
        };
        let better = match &best {
            None => true,
            Some(b) => cand.txg > b.txg,
        };
        if better {
            best = Some(cand);
        }
    }
    best
}

/// `true` if the image contains at least one plausible uberblock.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vdev(txg: u64, slot: usize) -> Vec<u8> {
        let mut f = vec![0u8; UBERBLOCK_OFFSET + 1024 * (slot + 2)];
        let u = UBERBLOCK_OFFSET + slot * 1024;
        f[u..u + 8].copy_from_slice(&0x00BA_B10Cu64.to_le_bytes());
        f[u + 8..u + 16].copy_from_slice(&5000u64.to_le_bytes());
        f[u + 16..u + 24].copy_from_slice(&txg.to_le_bytes());
        f[u + 24..u + 32].copy_from_slice(&0xCAFEu64.to_le_bytes());
        f[u + 32..u + 40].copy_from_slice(&1_700_000_000u64.to_le_bytes());
        f
    }

    #[test]
    fn picks_highest_txg() {
        let mut f = vdev(10, 0);
        let u2 = UBERBLOCK_OFFSET + 1024;
        f[u2..u2 + 8].copy_from_slice(&0x00BA_B10Cu64.to_le_bytes());
        f[u2 + 8..u2 + 16].copy_from_slice(&5000u64.to_le_bytes());
        f[u2 + 16..u2 + 24].copy_from_slice(&77u64.to_le_bytes());
        f[u2 + 24..u2 + 32].copy_from_slice(&0xBEEFu64.to_le_bytes());
        f[u2 + 32..u2 + 40].copy_from_slice(&1_700_000_001u64.to_le_bytes());
        let z = parse(&f).unwrap();
        assert_eq!(z.txg, 77);
        assert_eq!(z.slot, 1);
        assert_eq!(z.version, 5000);
        assert_eq!(z.guid_sum, 0xBEEF);
        assert_eq!(z.timestamp, 1_700_000_001);
        assert!(!z.big_endian);
    }

    #[test]
    fn big_endian_magic() {
        let mut f = vec![0u8; UBERBLOCK_OFFSET + 1024];
        let u = UBERBLOCK_OFFSET;
        // u64 magic 0x00bab10c stored big-endian
        f[u..u + 8].copy_from_slice(&[0, 0, 0, 0, 0x00, 0xBA, 0xB1, 0x0C]);
        f[u + 8..u + 16].copy_from_slice(&[0, 0, 0, 0, 0, 0, 0x13, 0x88]); // 5000
        f[u + 16..u + 24].copy_from_slice(&[0, 0, 0, 0, 0, 0, 0, 5]);
        let z = parse(&f).unwrap();
        assert!(z.big_endian);
        assert_eq!(z.txg, 5);
    }

    #[test]
    fn skips_invalid_slots() {
        let mut f = vdev(0, 0); // txg 0 → invalid
        let u = UBERBLOCK_OFFSET;
        f[u + 16..u + 24].copy_from_slice(&0u64.to_le_bytes());
        let u1 = UBERBLOCK_OFFSET + 1024;
        f[u1..u1 + 8].copy_from_slice(&0x00BA_B10Cu64.to_le_bytes());
        f[u1 + 8..u1 + 16].copy_from_slice(&5000u64.to_le_bytes());
        f[u1 + 16..u1 + 24].copy_from_slice(&9u64.to_le_bytes());
        assert_eq!(parse(&f).unwrap().slot, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&vec![0u8; UBERBLOCK_OFFSET + 2048]).is_none());
        let mut f = vdev(3, 0);
        f[UBERBLOCK_OFFSET] = 0xFF;
        assert!(parse(&f).is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(&vdev(1, 0)));
        assert!(!detect(b"not zfs"));
    }
}
