//! exFAT boot region.
//!
//! The main boot sector at offset 0 carries the jump, the
//! `EXFAT   ` OEM string, a 53-byte `MustBeZero` area that must
//! be all zero (a hard check — a zero region catch catches
//! mis-mounted FAT12/16 media), then the volume geometry: FAT and
//! cluster-heap offsets and lengths, first root cluster, serial,
//! revision, flags and the *shift* encodings of bytes/sector and
//! sectors/cluster. A `55 AA` word ends the sector.
//!
//! ```
//! use izanagi_kit::exfat::parse;
//!
//! let mut d = vec![0u8; 512];
//! d[0..3].copy_from_slice(&[0xeb, 0x76, 0x90]);
//! d[3..11].copy_from_slice(b"EXFAT   ");
//! // +11..64 stays zero (MustBeZero)
//! let w32 = |d: &mut [u8], o: usize, v: u32| {
//!     for i in 0..4 { d[o + i] = (v >> (i * 8)) as u8; }
//! };
//! w32(&mut d, 80, 24);        // FAT offset (sectors)
//! w32(&mut d, 88, 128);       // cluster heap offset
//! w32(&mut d, 96, 3);         // first root cluster
//! d[108] = 9;                 // 512 B/sector (2^9)
//! d[109] = 0;                 // 1 cluster per... sector count shift
//! d[110] = 1;                 // one FAT
//! d[510] = 0x55; d[511] = 0xaa;
//! let x = parse(&d).unwrap();
//! assert_eq!(x.bytes_per_sector(), 512);
//! assert_eq!(x.cluster_size(), 512);
//! ```

/// `EXFAT   ` OEM id.
pub const OEM: &[u8; 8] = b"EXFAT   ";
/// The `MustBeZero` area: +11..64 must be all zero.
pub const MUST_BE_ZERO: core::ops::Range<usize> = 11..64;

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
fn le64(d: &[u8], at: usize) -> Option<u64> {
    Some(u64::from(le32(d, at)?) | u64::from(le32(d, at + 4)?) << 32)
}

/// A parsed exFAT main boot sector.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExFat {
    /// `PartitionOffset` (sectors).
    pub partition_offset: u64,
    /// `VolumeLength` (sectors).
    pub volume_length: u64,
    /// `FatOffset` (sectors).
    pub fat_offset: u32,
    /// `FatLength` (sectors).
    pub fat_length: u32,
    /// `ClusterHeapOffset` (sectors).
    pub cluster_heap_offset: u32,
    /// `ClusterCount`.
    pub cluster_count: u32,
    /// `FirstClusterOfRootDirectory`.
    pub first_root_cluster: u32,
    /// `VolumeSerialNumber`.
    pub serial: u32,
    /// `FileSystemRevision` (e.g. 0x0100 = 1.00).
    pub revision: u16,
    /// `VolumeFlags` (bit0 active-fat, 1 dirty, 2 media-failure,
    /// 3 clear-to-zero, 4-11 reserved).
    pub volume_flags: u16,
    /// `BytesPerSectorShift` (9..12).
    pub bytes_per_sector_shift: u8,
    /// `SectorsPerClusterShift` (0..25).
    pub sectors_per_cluster_shift: u8,
    /// `NumberOfFats` (1 or 2).
    pub number_of_fats: u8,
    /// `DriveSelect` (0x80 hard disk).
    pub drive_select: u8,
    /// `PercentInUse` (0..100, 0xFF = unknown).
    pub percent_in_use: u8,
}

impl ExFat {
    /// Bytes per sector.
    pub fn bytes_per_sector(&self) -> u32 {
        1u32.checked_shl(u32::from(self.bytes_per_sector_shift))
            .unwrap_or(0)
    }
    /// Sectors per cluster.
    pub fn sectors_per_cluster(&self) -> u32 {
        1u32.checked_shl(u32::from(self.sectors_per_cluster_shift))
            .unwrap_or(0)
    }
    /// Cluster size in bytes.
    pub fn cluster_size(&self) -> u64 {
        u64::from(self.bytes_per_sector()).saturating_mul(u64::from(self.sectors_per_cluster()))
    }
    /// Byte offset of the root directory cluster.
    pub fn root_cluster_at(&self) -> u64 {
        u64::from(self.cluster_heap_offset)
            .saturating_mul(u64::from(self.bytes_per_sector()))
            .saturating_add(
                u64::from(self.first_root_cluster.saturating_sub(2))
                    .saturating_mul(self.cluster_size()),
            )
    }
    /// Second FAT is active.
    pub fn second_fat_active(&self) -> bool {
        self.volume_flags & 1 != 0
    }
    /// Dirty bit set.
    pub fn is_dirty(&self) -> bool {
        self.volume_flags & 2 != 0
    }
}

/// Parse sector 0. Returns `None` on a bad OEM id, a non-zero
/// `MustBeZero` area, or a missing `55 AA`.
pub fn parse(d: &[u8]) -> Option<ExFat> {
    if d.get(3..11)? != OEM {
        return None;
    }
    if d.get(MUST_BE_ZERO)?.iter().any(|&b| b != 0) {
        return None;
    }
    if d.get(510..512)? != [0x55, 0xaa] {
        return None;
    }
    Some(ExFat {
        partition_offset: le64(d, 64)?,
        volume_length: le64(d, 72)?,
        fat_offset: le32(d, 80)?,
        fat_length: le32(d, 84)?,
        cluster_heap_offset: le32(d, 88)?,
        cluster_count: le32(d, 92)?,
        first_root_cluster: le32(d, 96)?,
        serial: le32(d, 100)?,
        revision: le16(d, 104)?,
        volume_flags: le16(d, 106)?,
        bytes_per_sector_shift: d.get(108).copied()?,
        sectors_per_cluster_shift: d.get(109).copied()?,
        number_of_fats: d.get(110).copied()?,
        drive_select: d.get(111).copied()?,
        percent_in_use: d.get(112).copied()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0u8; 512];
        d[0..3].copy_from_slice(&[0xeb, 0x76, 0x90]);
        d[3..11].copy_from_slice(OEM);
        let w32 = |d: &mut [u8], o: usize, v: u32| {
            for i in 0..4 {
                d[o + i] = (v >> (i * 8)) as u8;
            }
        };
        let w64 = |d: &mut [u8], o: usize, v: u64| {
            for i in 0..8 {
                d[o + i] = (v >> (i * 8)) as u8;
            }
        };
        w64(&mut d, 64, 2048); // partition offset
        w64(&mut d, 72, 1_000_000); // volume length
        w32(&mut d, 80, 24);
        w32(&mut d, 84, 16);
        w32(&mut d, 88, 128);
        w32(&mut d, 92, 50_000);
        w32(&mut d, 96, 4);
        w32(&mut d, 100, 0xDEAD_BEEF);
        d[104] = 0;
        d[105] = 1; // revision 1.00
        d[106] = 3;
        d[107] = 0; // flags: active-fat|dirty
        d[108] = 9;
        d[109] = 3;
        d[110] = 1;
        d[111] = 0x80;
        d[112] = 37;
        d[510] = 0x55;
        d[511] = 0xaa;
        d
    }

    #[test]
    fn fields() {
        let x = parse(&fixture()).unwrap();
        assert_eq!(x.partition_offset, 2048);
        assert_eq!(x.volume_length, 1_000_000);
        assert_eq!(x.fat_offset, 24);
        assert_eq!(x.fat_length, 16);
        assert_eq!(x.cluster_heap_offset, 128);
        assert_eq!(x.cluster_count, 50_000);
        assert_eq!(x.first_root_cluster, 4);
        assert_eq!(x.serial, 0xDEAD_BEEF);
        assert_eq!(x.revision, 0x0100);
        assert_eq!(x.volume_flags, 3);
        assert!(x.second_fat_active());
        assert!(x.is_dirty());
        assert_eq!(x.bytes_per_sector(), 512);
        assert_eq!(x.sectors_per_cluster(), 8);
        assert_eq!(x.cluster_size(), 4096);
        assert_eq!(x.root_cluster_at(), 128 * 512 + 2 * 4096);
        assert_eq!(x.number_of_fats, 1);
        assert_eq!(x.drive_select, 0x80);
        assert_eq!(x.percent_in_use, 37);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 100]).is_none());
        let mut d = fixture();
        d[3] = b'X';
        assert!(parse(&d).is_none());
        let mut d2 = fixture();
        d2[20] = 1; // MustBeZero violation
        assert!(parse(&d2).is_none());
        let mut d3 = fixture();
        d3[510] = 0;
        assert!(parse(&d3).is_none());
    }
}
