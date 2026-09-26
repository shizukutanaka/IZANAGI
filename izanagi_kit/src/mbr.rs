//! Master Boot Record partition table (PC/BIOS disks).
//!
//! The first 512 bytes hold up to four 16-byte partition entries at
//! offset `446`, a disk signature word at `440`, and the boot
//! signature `55 AA` at `510/511`. Each entry carries a status byte
//! (`80` = bootable), a CHS first/last pair, a partition type byte,
//! and the LBA start plus sector count.
//!
//! ```
//! use izanagi_kit::mbr::{parse, TYPE_GPT_PROTECTIVE};
//!
//! let mut d = vec![0u8; 512];
//! d[446] = 0x80;                          // bootable
//! d[446 + 4] = 0x83;                      // Linux
//! d[446 + 8..446 + 12].copy_from_slice(&2048u32.to_le_bytes());
//! d[446 + 12..446 + 16].copy_from_slice(&4096u32.to_le_bytes());
//! d[510] = 0x55; d[511] = 0xAA;
//! let m = parse(&d).unwrap();
//! assert!(m.entries[0].bootable());
//! assert_eq!(m.entries[0].kind, 0x83);
//! assert_eq!(m.entries[0].end_lba(), 2048 + 4096);
//! assert_eq!(TYPE_GPT_PROTECTIVE, 0xEE);
//! ```

/// Sector size the table is embedded in.
pub const SECTOR: usize = 512;
/// Entry size in bytes.
pub const ENTRY_SIZE: usize = 16;
/// Number of entries in the primary table.
pub const ENTRIES: usize = 4;
/// Offset of the first entry.
pub const TABLE_AT: usize = 446;
/// Offset of the 32-bit disk signature (Windows NT and later).
pub const DISK_SIG_AT: usize = 440;
/// Status byte marking a bootable partition.
pub const STATUS_BOOTABLE: u8 = 0x80;
/// Partition type used by a protective MBR in front of a GPT disk.
pub const TYPE_GPT_PROTECTIVE: u8 = 0xEE;

fn u32le(d: &[u8], at: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(at)?)
            | u32::from(*d.get(at + 1)?) << 8
            | u32::from(*d.get(at + 2)?) << 16
            | u32::from(*d.get(at + 3)?) << 24,
    )
}

/// One 16-byte partition entry.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    /// Status byte; `0x80` marks the bootable partition.
    pub status: u8,
    /// CHS address of the first sector (three packed bytes).
    pub chs_first: [u8; 3],
    /// Partition type byte (`0x83` Linux, `0x07` NTFS/exFAT, `0xEE` GPT protective, ...).
    pub kind: u8,
    /// CHS address of the last sector.
    pub chs_last: [u8; 3],
    /// LBA of the first sector.
    pub lba: u32,
    /// Sector count.
    pub sectors: u32,
}

impl Entry {
    /// True when the status byte marks this entry bootable.
    pub fn bootable(&self) -> bool {
        self.status == STATUS_BOOTABLE
    }

    /// True for an empty slot (type `0` and no sectors).
    pub fn empty(&self) -> bool {
        self.kind == 0 && self.sectors == 0
    }

    /// First LBA past the partition (`lba + sectors`).
    pub fn end_lba(&self) -> u64 {
        u64::from(self.lba) + u64::from(self.sectors)
    }
}

/// A parsed MBR: disk signature plus the four partition entries.
#[derive(Clone, Debug, PartialEq)]
pub struct Mbr {
    /// 32-bit disk signature at offset `440` (zero when unused).
    pub disk_sig: u32,
    /// The four partition entries.
    pub entries: [Entry; ENTRIES],
}

impl Mbr {
    /// True when every non-empty entry is a GPT protective partition —
    /// the layout a GPT disk presents to legacy tools.
    pub fn protective(&self) -> bool {
        self.entries
            .iter()
            .all(|e| e.empty() || e.kind == TYPE_GPT_PROTECTIVE)
            && self.entries.iter().any(|e| e.kind == TYPE_GPT_PROTECTIVE)
    }
}

fn entry_at(d: &[u8], at: usize) -> Option<Entry> {
    Some(Entry {
        status: *d.get(at)?,
        chs_first: [*d.get(at + 1)?, *d.get(at + 2)?, *d.get(at + 3)?],
        kind: *d.get(at + 4)?,
        chs_last: [*d.get(at + 5)?, *d.get(at + 6)?, *d.get(at + 7)?],
        lba: u32le(d, at + 8)?,
        sectors: u32le(d, at + 12)?,
    })
}

/// Parse an MBR sector. Returns `None` when the buffer is shorter than
/// 512 bytes or lacks the `55 AA` boot signature.
pub fn parse(d: &[u8]) -> Option<Mbr> {
    if d.len() < SECTOR || d[SECTOR - 2] != 0x55 || d[SECTOR - 1] != 0xAA {
        return None;
    }
    Some(Mbr {
        disk_sig: u32le(d, DISK_SIG_AT)?,
        entries: [
            entry_at(d, TABLE_AT)?,
            entry_at(d, TABLE_AT + ENTRY_SIZE)?,
            entry_at(d, TABLE_AT + ENTRY_SIZE * 2)?,
            entry_at(d, TABLE_AT + ENTRY_SIZE * 3)?,
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn disk() -> Vec<u8> {
        let mut d = vec![0u8; 512];
        d[440..444].copy_from_slice(&0xDEADBEEFu32.to_le_bytes());
        d[446] = 0x80;
        d[446 + 4] = 0x83;
        d[446 + 8..446 + 12].copy_from_slice(&2048u32.to_le_bytes());
        d[446 + 12..446 + 16].copy_from_slice(&4096u32.to_le_bytes());
        d[462 + 4] = TYPE_GPT_PROTECTIVE;
        d[462 + 8..462 + 12].copy_from_slice(&1u32.to_le_bytes());
        d[462 + 12..462 + 16].copy_from_slice(&0xFFFFu32.to_le_bytes());
        d[510] = 0x55;
        d[511] = 0xAA;
        d
    }

    #[test]
    fn parse_reads_every_field() {
        let d = disk();
        let m = parse(&d).unwrap();
        assert_eq!(m.disk_sig, 0xDEADBEEF);
        let e = &m.entries[0];
        assert!(e.bootable());
        assert!(!e.empty());
        assert_eq!(e.kind, 0x83);
        assert_eq!(e.lba, 2048);
        assert_eq!(e.sectors, 4096);
        assert_eq!(e.end_lba(), 6144);
        assert_eq!(e.chs_first, [0, 0, 0]);
        assert_eq!(e.chs_last, [0, 0, 0]);
    }

    #[test]
    fn protective_detects_gpt_front() {
        let mut d = disk();
        // wipe the linux entry so only the protective one remains
        for b in d.iter_mut().take(462).skip(446) {
            *b = 0;
        }
        assert!(parse(&d).unwrap().protective());
        assert!(!parse(&disk()).unwrap().protective());
    }

    #[test]
    fn rejects_short_and_unsigned() {
        assert_eq!(parse(&[0u8; 511]), None);
        assert_eq!(parse(&vec![0u8; 512]), None);
        let mut d = disk();
        d[511] = 0xAB;
        assert_eq!(parse(&d), None);
    }

    #[test]
    fn empty_entry() {
        let e = Entry {
            status: 0,
            chs_first: [0; 3],
            kind: 0,
            chs_last: [0; 3],
            lba: 0,
            sectors: 0,
        };
        assert!(e.empty());
        assert!(!e.bootable());
        assert_eq!(e.end_lba(), 0);
    }

    #[test]
    fn constants() {
        assert_eq!(SECTOR, 512);
        assert_eq!(ENTRY_SIZE, 16);
        assert_eq!(ENTRIES, 4);
        assert_eq!(TABLE_AT, 446);
        assert_eq!(DISK_SIG_AT, 440);
        assert_eq!(STATUS_BOOTABLE, 0x80);
    }
}
