//! NTFS boot sector (BIOS parameter block + extended BPB).
//!
//! Sector 0 holds the jump instruction, the `NTFS    ` OEM id,
//! bytes-per-sector / sectors-per-cluster, total sectors, the LCNs
//! of `$MFT` and `$MFTMirr`, and the signed cluster factors for
//! file-record and index sizes (negative value `-n` means
//! `2^|n|` bytes, positive means `n` clusters). `parse` reports
//! all of them plus the trailing `55 AA` signature.
//!
//! ```
//! use izanagi_kit::ntfs::parse;
//!
//! let mut d = vec![0u8; 512];
//! d[0..3].copy_from_slice(&[0xeb, 0x52, 0x90]);
//! d[3..11].copy_from_slice(b"NTFS    ");
//! d[11] = 0; d[12] = 2;         // 512 B/sector
//! d[13] = 8;                    // 8 sectors/cluster
//! d[64] = 0xF6;                 // -10 -> 1024-byte file records
//! d[68] = 1;                    // 1 cluster per index buffer
//! d[510] = 0x55; d[511] = 0xaa;
//! let n = parse(&d).unwrap();
//! assert_eq!(n.bytes_per_sector, 512);
//! assert_eq!(n.cluster_size(), 4096);
//! assert_eq!(n.file_record_bytes(), 1024);
//! ```

fn le16(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from(*d.get(at)?) | u16::from(*d.get(at + 1)?) << 8)
}
fn le64(d: &[u8], at: usize) -> Option<u64> {
    Some(
        u64::from(*d.get(at)?)
            | u64::from(*d.get(at + 1)?) << 8
            | u64::from(*d.get(at + 2)?) << 16
            | u64::from(*d.get(at + 3)?) << 24
            | u64::from(*d.get(at + 4)?) << 32
            | u64::from(*d.get(at + 5)?) << 40
            | u64::from(*d.get(at + 6)?) << 48
            | u64::from(*d.get(at + 7)?) << 56,
    )
}

/// A parsed NTFS boot sector.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ntfs {
    /// Bytes per sector (usually 512).
    pub bytes_per_sector: u16,
    /// Sectors per cluster.
    pub sectors_per_cluster: u8,
    /// Total sectors on the volume.
    pub total_sectors: u64,
    /// LCN of `$MFT`.
    pub mft_lcn: u64,
    /// LCN of `$MFTMirr`.
    pub mft_mirr_lcn: u64,
    /// Signed clusters-per-file-record.
    pub clusters_per_file_record: i8,
    /// Signed clusters-per-index-buffer.
    pub clusters_per_index: i8,
    /// Volume serial number.
    pub serial: u64,
}

impl Ntfs {
    /// Cluster size in bytes.
    pub fn cluster_size(&self) -> u64 {
        u64::from(self.bytes_per_sector) * u64::from(self.sectors_per_cluster)
    }
    /// Byte size of one `$MFT` file record.
    pub fn file_record_bytes(&self) -> u64 {
        expand(self.clusters_per_file_record, self.cluster_size())
    }
    /// Byte size of an index buffer.
    pub fn index_buffer_bytes(&self) -> u64 {
        expand(self.clusters_per_index, self.cluster_size())
    }
}

fn expand(clusters: i8, cluster_size: u64) -> u64 {
    if clusters < 0 {
        1u64.checked_shl(u32::from(clusters.unsigned_abs()))
            .unwrap_or(0)
    } else {
        u64::from(clusters as u8) * cluster_size
    }
}

/// Parse sector 0. Returns `None` on a bad OEM id or missing
/// `55 AA`.
pub fn parse(d: &[u8]) -> Option<Ntfs> {
    if d.get(3..11)? != b"NTFS    " {
        return None;
    }
    if d.get(510..512)? != [0x55, 0xaa] {
        return None;
    }
    Some(Ntfs {
        bytes_per_sector: le16(d, 11)?,
        sectors_per_cluster: d.get(13).copied()?,
        total_sectors: le64(d, 40)?,
        mft_lcn: le64(d, 48)?,
        mft_mirr_lcn: le64(d, 56)?,
        clusters_per_file_record: d.get(64).copied()? as i8,
        clusters_per_index: d.get(68).copied()? as i8,
        serial: le64(d, 72)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0u8; 512];
        d[0..3].copy_from_slice(&[0xeb, 0x52, 0x90]);
        d[3..11].copy_from_slice(b"NTFS    ");
        d[11] = 0;
        d[12] = 2;
        d[13] = 8;
        // total sectors = 1_000_000 at 40
        let v = 1_000_000u64;
        for i in 0..8 {
            d[40 + i] = (v >> (i * 8)) as u8;
            d[48 + i] = ((786_432u64) >> (i * 8)) as u8; // mft
            d[56 + i] = ((786_436u64) >> (i * 8)) as u8; // mirr
            d[72 + i] = (0xA1B2_C3D4_E5F6_0718u64 >> (i * 8)) as u8;
        }
        d[64] = (-10i8) as u8;
        d[68] = 1;
        d[510] = 0x55;
        d[511] = 0xaa;
        d
    }

    #[test]
    fn fields() {
        let n = parse(&fixture()).unwrap();
        assert_eq!(n.bytes_per_sector, 512);
        assert_eq!(n.sectors_per_cluster, 8);
        assert_eq!(n.cluster_size(), 4096);
        assert_eq!(n.total_sectors, 1_000_000);
        assert_eq!(n.mft_lcn, 786_432);
        assert_eq!(n.mft_mirr_lcn, 786_436);
        assert_eq!(n.clusters_per_file_record, -10);
        assert_eq!(n.file_record_bytes(), 1024);
        assert_eq!(n.index_buffer_bytes(), 4096);
        assert_eq!(n.serial, 0xA1B2_C3D4_E5F6_0718);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 128]).is_none());
        let mut d = fixture();
        d[3] = b'X';
        assert!(parse(&d).is_none());
        let mut d2 = fixture();
        d2[510] = 0;
        assert!(parse(&d2).is_none());
    }
}
