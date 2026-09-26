//! VirtualBox VDI (Virtual Disk Image) header, format v1.1+.
//!
//! The file starts with a 64-byte text banner (`<<< Oracle VM
//! VirtualBox Disk Image >>>`, NUL-padded), then a little-endian
//! header: signature `0xBEDA107F`, version, header size, image type
//! (1 = dynamic, 2 = static), block/data offsets, legacy CHS
//! geometry, disk size, block sizing and four UUIDs.
//!
//! ```
//! use izanagi_kit::vdi::{parse, Kind, SIGNATURE};
//!
//! let mut d = vec![0u8; 456];
//! d[..39].copy_from_slice(b"<<< Oracle VM VirtualBox Disk Image >>>");
//! let put = |d: &mut [u8], at: usize, v: u32| d[at..at + 4].copy_from_slice(&v.to_le_bytes());
//! put(&mut d, 64, SIGNATURE);
//! put(&mut d, 68, 0x0001_0001);   // v1.1
//! put(&mut d, 72, 392);           // header size
//! put(&mut d, 76, 1);             // dynamic
//! put(&mut d, 340, 2048);         // block map offset
//! put(&mut d, 344, 4096);         // data offset
//! d[368..376].copy_from_slice(&0x0010_0000u64.to_le_bytes()); // 1 MiB disk
//! put(&mut d, 376, 0x0010_0000);  // 1 MiB blocks
//! let v = parse(&d).unwrap();
//! assert_eq!(v.kind, Kind::Dynamic);
//! assert_eq!(v.disk_size, 0x10_0000);
//! assert_eq!(v.block_size, 0x10_0000);
//! ```

use std::string::String;

/// Header signature (`u32` little-endian at offset 64).
pub const SIGNATURE: u32 = 0xBED_A107F;
/// Minimum header size (`signature`..`parent uuid` end).
pub const HEADER: usize = 456;

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

/// Image type.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    /// `1` — dynamically allocated blocks.
    Dynamic,
    /// `2` — fully allocated static image.
    Static,
    /// Any other type id.
    Other(u32),
}

/// A parsed VDI header.
#[derive(Clone, Debug, PartialEq)]
pub struct Vdi {
    /// Banner text (first 64 bytes, NUL-trimmed).
    pub banner: String,
    /// Format version (`0x00010001` for v1.1).
    pub version: u32,
    /// Header size in bytes.
    pub header_size: u32,
    /// Image type.
    pub kind: Kind,
    /// Header flags.
    pub flags: u32,
    /// Human-readable description (256 bytes, NUL-trimmed).
    pub description: String,
    /// Byte offset of the block allocation table.
    pub blocks_at: u32,
    /// Byte offset of the image data region.
    pub data_at: u32,
    /// Legacy cylinders.
    pub cylinders: u32,
    /// Legacy heads.
    pub heads: u32,
    /// Legacy sectors.
    pub sectors: u32,
    /// Sector size in bytes.
    pub sector_size: u32,
    /// Total disk capacity in bytes.
    pub disk_size: u64,
    /// Block size in bytes (usually 1 MiB).
    pub block_size: u32,
    /// Extra bytes prepended to each block.
    pub block_extra: u32,
    /// Blocks the disk can hold.
    pub blocks_in_image: u32,
    /// Blocks currently allocated.
    pub blocks_allocated: u32,
    /// Image UUID (raw 16 bytes).
    pub uuid: [u8; 16],
    /// Snapshot UUID.
    pub uuid_snap: [u8; 16],
    /// Linked-image UUID.
    pub uuid_link: [u8; 16],
    /// Parent-image UUID.
    pub uuid_parent: [u8; 16],
}

fn text(d: &[u8], at: usize, n: usize) -> String {
    let raw = d.get(at..at + n).unwrap_or(&[]);
    let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
    String::from_utf8_lossy(raw.get(..end).unwrap_or(&[])).into_owned()
}

/// Parse a VDI header. Returns `None` on a bad signature or a buffer
/// shorter than the 456-byte v1.1 header.
pub fn parse(d: &[u8]) -> Option<Vdi> {
    if le32(d, 64)? != SIGNATURE {
        return None;
    }
    let mut uuid = [0u8; 16];
    uuid.copy_from_slice(d.get(392..408)?);
    let mut uuid_snap = [0u8; 16];
    uuid_snap.copy_from_slice(d.get(408..424)?);
    let mut uuid_link = [0u8; 16];
    uuid_link.copy_from_slice(d.get(424..440)?);
    let mut uuid_parent = [0u8; 16];
    uuid_parent.copy_from_slice(d.get(440..456)?);
    Some(Vdi {
        banner: text(d, 0, 64),
        version: le32(d, 68)?,
        header_size: le32(d, 72)?,
        kind: match le32(d, 76)? {
            1 => Kind::Dynamic,
            2 => Kind::Static,
            v => Kind::Other(v),
        },
        flags: le32(d, 80)?,
        description: text(d, 84, 256),
        blocks_at: le32(d, 340)?,
        data_at: le32(d, 344)?,
        cylinders: le32(d, 348)?,
        heads: le32(d, 352)?,
        sectors: le32(d, 356)?,
        sector_size: le32(d, 360)?,
        disk_size: le64(d, 368)?,
        block_size: le32(d, 376)?,
        block_extra: le32(d, 380)?,
        blocks_in_image: le32(d, 384)?,
        blocks_allocated: le32(d, 388)?,
        uuid,
        uuid_snap,
        uuid_link,
        uuid_parent,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn put(d: &mut [u8], at: usize, v: u32) {
        d[at..at + 4].copy_from_slice(&v.to_le_bytes());
    }

    fn header() -> Vec<u8> {
        let mut d = vec![0u8; HEADER];
        d[..39].copy_from_slice(b"<<< Oracle VM VirtualBox Disk Image >>>");
        put(&mut d, 64, SIGNATURE);
        put(&mut d, 68, 0x0001_0001);
        put(&mut d, 72, 392);
        put(&mut d, 76, 1);
        put(&mut d, 80, 0);
        let desc = b"test disk";
        d[84..84 + desc.len()].copy_from_slice(desc);
        put(&mut d, 340, 2048);
        put(&mut d, 344, 4096);
        put(&mut d, 348, 100);
        put(&mut d, 352, 16);
        put(&mut d, 356, 63);
        put(&mut d, 360, 512);
        d[368..376].copy_from_slice(&0x0010_0000u64.to_le_bytes());
        put(&mut d, 376, 0x0010_0000);
        put(&mut d, 380, 0);
        put(&mut d, 384, 1);
        put(&mut d, 388, 1);
        for i in 0..16 {
            d[392 + i] = i as u8;
            d[440 + i] = 0xF0 - i as u8;
        }
        d
    }

    #[test]
    fn parse_reads_every_field() {
        let d = header();
        let v = parse(&d).unwrap();
        assert_eq!(v.banner, "<<< Oracle VM VirtualBox Disk Image >>>");
        assert_eq!(v.version, 0x0001_0001);
        assert_eq!(v.header_size, 392);
        assert_eq!(v.kind, Kind::Dynamic);
        assert_eq!(v.description, "test disk");
        assert_eq!(v.blocks_at, 2048);
        assert_eq!(v.data_at, 4096);
        assert_eq!((v.cylinders, v.heads, v.sectors), (100, 16, 63));
        assert_eq!(v.sector_size, 512);
        assert_eq!(v.disk_size, 0x10_0000);
        assert_eq!(v.block_size, 0x10_0000);
        assert_eq!(v.blocks_in_image, 1);
        assert_eq!(v.blocks_allocated, 1);
        assert_eq!(v.uuid[15], 15);
        assert_eq!(v.uuid_parent[0], 0xF0);
    }

    #[test]
    fn static_and_other_kinds() {
        let mut d = header();
        put(&mut d, 76, 2);
        assert_eq!(parse(&d).unwrap().kind, Kind::Static);
        put(&mut d, 76, 7);
        assert_eq!(parse(&d).unwrap().kind, Kind::Other(7));
    }

    #[test]
    fn rejects() {
        assert_eq!(parse(&[]), None);
        assert_eq!(parse(&[0u8; 100]), None);
        let mut d = header();
        d[64] = 0;
        assert_eq!(parse(&d), None);
    }

    #[test]
    fn constants() {
        assert_eq!(SIGNATURE, 0xBED_A107F);
        assert_eq!(HEADER, 456);
    }
}
