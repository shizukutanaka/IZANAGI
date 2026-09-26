//! GUID Partition Table header and entries (UEFI disks).
//!
//! The GPT header lives at LBA 1 behind a protective MBR: the
//! `EFI PART` magic, a revision word, the header size (92..=512), a
//! CRC32 of the header with the CRC field zeroed, the primary and
//! backup LBA pointers, the usable range, a disk GUID, and the
//! partition-entry array descriptor (`lba`, count, `entry_size`).
//! Each entry is `type GUID | unique GUID | first_lba | last_lba |
//! attrs | 72-byte UTF-16LE name`.
//!
//! `parse` consumes a slice that starts *at* the GPT header; use
//! `entry(d, i, sector_size)` to index the entry array.
//!
//! ```
//! use izanagi_kit::gpt::{parse, HEADER_MIN};
//!
//! let mut d = vec![0u8; HEADER_MIN];
//! d[..8].copy_from_slice(b"EFI PART");
//! d[8..12].copy_from_slice(&0x00010000u32.to_le_bytes());
//! d[12..16].copy_from_slice(&(HEADER_MIN as u32).to_le_bytes());
//! d[24..32].copy_from_slice(&1u64.to_le_bytes());   // current LBA
//! d[32..40].copy_from_slice(&100u64.to_le_bytes());  // backup LBA
//! d[80..84].copy_from_slice(&128u32.to_le_bytes());  // n entries
//! d[84..88].copy_from_slice(&128u32.to_le_bytes());  // entry size
//! let g = parse(&d).unwrap();
//! assert_eq!(g.revision, 0x00010000);
//! assert_eq!(g.n_entries, 128);
//! assert!(!g.header_crc_ok(&d)); // crc field is zero in the fixture
//! ```

use crate::crc::crc32;
use std::string::String;

/// Minimum valid header size in bytes.
pub const HEADER_MIN: usize = 92;
/// Maximum valid header size in bytes.
pub const HEADER_MAX: usize = 512;
/// Minimum partition-entry size in bytes.
pub const ENTRY_MIN: usize = 128;
/// Bytes reserved for the UTF-16LE partition name in an entry.
pub const NAME_LEN: usize = 72;
/// The 8-byte signature at the start of the header.
pub const MAGIC: &[u8; 8] = b"EFI PART";

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

/// One partition-table entry (at least `ENTRY_MIN` bytes on disk).
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    /// Partition type GUID (all zeros = unused slot).
    pub type_guid: [u8; 16],
    /// Unique GUID of this partition.
    pub guid: [u8; 16],
    /// First LBA (inclusive).
    pub first_lba: u64,
    /// Last LBA (inclusive).
    pub last_lba: u64,
    /// Attribute flags.
    pub attrs: u64,
    /// UTF-16LE code units of the partition name (NUL-terminated).
    pub name: [u16; NAME_LEN / 2],
}

impl Entry {
    /// True when the slot carries a partition (nonzero type GUID).
    pub fn used(&self) -> bool {
        self.type_guid.iter().any(|&b| b != 0)
    }

    /// The UTF-16LE name decoded up to the first NUL code unit.
    pub fn name_str(&self) -> String {
        let mut s = String::new();
        for &u in &self.name {
            if u == 0 {
                break;
            }
            if let Some(c) = char::from_u32(u32::from(u)) {
                s.push(c);
            }
        }
        s
    }
}

/// A parsed GPT header.
#[derive(Clone, Debug, PartialEq)]
pub struct Gpt {
    /// Revision (`0x00010000` for GPT v1).
    pub revision: u32,
    /// Header size in bytes.
    pub header_size: u32,
    /// Stored CRC32 of the header.
    pub header_crc: u32,
    /// LBA of this header.
    pub current_lba: u64,
    /// LBA of the backup header.
    pub backup_lba: u64,
    /// First usable LBA for partitions.
    pub first_usable: u64,
    /// Last usable LBA.
    pub last_usable: u64,
    /// Disk GUID.
    pub disk_guid: [u8; 16],
    /// LBA of the partition-entry array.
    pub entries_lba: u64,
    /// Number of entries in the array.
    pub n_entries: u32,
    /// Size of one entry in bytes (>= `ENTRY_MIN`, multiple of 8).
    pub entry_size: u32,
    /// Stored CRC32 of the entry array.
    pub entries_crc: u32,
}

impl Gpt {
    /// Verify the header CRC32 against `d` (which starts at the header).
    pub fn header_crc_ok(&self, d: &[u8]) -> bool {
        let n = self.header_size as usize;
        match d.get(..n) {
            Some(h) if n >= 16 => {
                let mut h2: Vec<u8> = h.to_vec();
                h2[16] = 0;
                h2[17] = 0;
                h2[18] = 0;
                h2[19] = 0;
                crc32(&h2) == self.header_crc
            }
            _ => false,
        }
    }

    /// Verify the CRC32 of the entry array. `d` is the whole disk image
    /// and `sector_size` converts `entries_lba` to a byte offset.
    pub fn entries_crc_ok(&self, d: &[u8], sector_size: usize) -> bool {
        let at = usize::try_from(self.entries_lba)
            .ok()
            .and_then(|l| l.checked_mul(sector_size));
        let len = usize::try_from(self.n_entries)
            .ok()
            .and_then(|n| n.checked_mul(self.entry_size as usize));
        match (at, len) {
            (Some(at), Some(len)) => match d.get(at..at + len) {
                Some(buf) => crc32(buf) == self.entries_crc,
                None => false,
            },
            _ => false,
        }
    }

    /// Byte offset of entry `i` within `d` (a whole-disk image), or
    /// `None` when out of range or out of bounds.
    fn entry_at(&self, i: usize, sector_size: usize) -> Option<usize> {
        if i >= self.n_entries as usize {
            return None;
        }
        usize::try_from(self.entries_lba)
            .ok()?
            .checked_mul(sector_size)?
            .checked_add(i.checked_mul(self.entry_size as usize)?)
    }

    /// Read partition entry `i` out of a whole-disk image.
    pub fn entry(&self, d: &[u8], i: usize, sector_size: usize) -> Option<Entry> {
        let at = self.entry_at(i, sector_size)?;
        let mut e = Entry {
            type_guid: [0; 16],
            guid: [0; 16],
            first_lba: u64le(d, at + 32)?,
            last_lba: u64le(d, at + 40)?,
            attrs: u64le(d, at + 48)?,
            name: [0; NAME_LEN / 2],
        };
        e.type_guid.copy_from_slice(d.get(at..at + 16)?);
        e.guid.copy_from_slice(d.get(at + 16..at + 32)?);
        for (j, u) in e.name.iter_mut().enumerate() {
            *u = u16::from(*d.get(at + 56 + j * 2)?) | u16::from(*d.get(at + 57 + j * 2)?) << 8;
        }
        Some(e)
    }
}

/// Parse a GPT header from `d` (which starts at the header). Returns
/// `None` on a bad magic, a revision that is not `0x0001xxxx`, or an
/// out-of-range header size.
pub fn parse(d: &[u8]) -> Option<Gpt> {
    if d.get(..8)? != MAGIC {
        return None;
    }
    let revision = u32le(d, 8)?;
    if revision >> 16 != 1 {
        return None;
    }
    let header_size = u32le(d, 12)?;
    if u32le(d, 20)? != 0 {
        return None;
    }
    if !(HEADER_MIN as u32..=HEADER_MAX as u32).contains(&header_size) {
        return None;
    }
    let entry_size = u32le(d, 84)?;
    if entry_size < ENTRY_MIN as u32 || entry_size % 8 != 0 {
        return None;
    }
    let mut disk_guid = [0u8; 16];
    disk_guid.copy_from_slice(d.get(56..72)?);
    Some(Gpt {
        revision,
        header_size,
        header_crc: u32le(d, 16)?,
        current_lba: u64le(d, 24)?,
        backup_lba: u64le(d, 32)?,
        first_usable: u64le(d, 40)?,
        last_usable: u64le(d, 48)?,
        disk_guid,
        entries_lba: u64le(d, 72)?,
        n_entries: u32le(d, 80)?,
        entry_size,
        entries_crc: u32le(d, 88)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    const SECTOR: usize = 512;

    fn header() -> Vec<u8> {
        let mut d = vec![0u8; SECTOR];
        d[..8].copy_from_slice(MAGIC);
        d[8..12].copy_from_slice(&0x00010000u32.to_le_bytes());
        d[12..16].copy_from_slice(&(HEADER_MIN as u32).to_le_bytes());
        d[24..32].copy_from_slice(&1u64.to_le_bytes());
        d[32..40].copy_from_slice(&99u64.to_le_bytes());
        d[40..48].copy_from_slice(&34u64.to_le_bytes());
        d[48..56].copy_from_slice(&98u64.to_le_bytes());
        for i in 0..16 {
            d[56 + i] = i as u8;
        }
        d[72..80].copy_from_slice(&2u64.to_le_bytes());
        d[80..84].copy_from_slice(&4u32.to_le_bytes());
        d[84..88].copy_from_slice(&128u32.to_le_bytes());
        let crc = {
            d[16..20].copy_from_slice(&[0; 4]);
            crc32(&d[..HEADER_MIN])
        };
        d[16..20].copy_from_slice(&crc.to_le_bytes());
        d
    }

    #[test]
    fn parse_reads_every_field() {
        let d = header();
        let g = parse(&d).unwrap();
        assert_eq!(g.revision, 0x00010000);
        assert_eq!(g.header_size, HEADER_MIN as u32);
        assert_eq!(g.current_lba, 1);
        assert_eq!(g.backup_lba, 99);
        assert_eq!(g.first_usable, 34);
        assert_eq!(g.last_usable, 98);
        assert_eq!(g.disk_guid[3], 3);
        assert_eq!(g.entries_lba, 2);
        assert_eq!(g.n_entries, 4);
        assert_eq!(g.entry_size, 128);
    }

    #[test]
    fn header_crc_verifies() {
        let d = header();
        let g = parse(&d).unwrap();
        assert!(g.header_crc_ok(&d));
        let mut bad = d.clone();
        bad[90] ^= 0xFF;
        assert!(!g.header_crc_ok(&bad));
        assert!(!g.header_crc_ok(&d[..8]));
    }

    #[test]
    fn entries_round_trip() {
        let mut d = header();
        d.resize(SECTOR * 4, 0);
        let g = parse(&d).unwrap();
        let at = SECTOR * 2;
        d[at] = 1;
        d[at + 16] = 2;
        d[at + 32..at + 40].copy_from_slice(&40u64.to_le_bytes());
        d[at + 40..at + 48].copy_from_slice(&80u64.to_le_bytes());
        d[at + 56..at + 60].copy_from_slice(&[b'H', 0, b'i', 0]);
        let e = g.entry(&d, 0, SECTOR).unwrap();
        assert!(e.used());
        assert_eq!(e.type_guid[0], 1);
        assert_eq!(e.guid[0], 2);
        assert_eq!(e.first_lba, 40);
        assert_eq!(e.last_lba, 80);
        assert_eq!(e.name_str(), "Hi");
        assert_eq!(g.entry(&d, 4, SECTOR), None);
        let unused = g.entry(&d, 1, SECTOR).unwrap();
        assert!(!unused.used());
        assert_eq!(unused.name_str(), "");
    }

    #[test]
    fn entries_crc_verifies() {
        let mut d = header();
        d.resize(SECTOR * 4, 0);
        let at = SECTOR * 2;
        d[at] = 1;
        let crc = crc32(&d[at..at + 4 * 128]);
        d[88..92].copy_from_slice(&crc.to_le_bytes());
        let g = parse(&d).unwrap();
        assert!(g.entries_crc_ok(&d, SECTOR));
        d[at + 200] = 9;
        assert!(!g.entries_crc_ok(&d, SECTOR));
    }

    #[test]
    fn rejects_bad_header() {
        assert_eq!(parse(&[0u8; 8]), None);
        let mut d = header();
        d[0] = b'X';
        assert_eq!(parse(&d), None);
        let d2 = header();
        let mut d3 = d2.clone();
        d3[12..16].copy_from_slice(&10u32.to_le_bytes());
        assert_eq!(parse(&d3), None);
        let mut d4 = d2;
        d4[8..12].copy_from_slice(&0x00020000u32.to_le_bytes());
        assert_eq!(parse(&d4), None);
    }

    #[test]
    fn rejects_bad_entry_size() {
        let mut d = header();
        d[84..88].copy_from_slice(&64u32.to_le_bytes());
        assert_eq!(parse(&d), None);
        d[84..88].copy_from_slice(&130u32.to_le_bytes());
        assert_eq!(parse(&d), None);
    }

    #[test]
    fn constants() {
        assert_eq!(HEADER_MIN, 92);
        assert_eq!(HEADER_MAX, 512);
        assert_eq!(ENTRY_MIN, 128);
        assert_eq!(NAME_LEN, 72);
        assert_eq!(MAGIC, b"EFI PART");
    }
}
