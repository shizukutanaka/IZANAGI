//! CramFS (Compressed ROM File System) superblock parsing.
//!
//! 64-byte little-endian superblock: magic `0x28CD3D45`, `size`,
//! `flags`, `future`, signature `"Compressed ROMFS"` (16B), `fsid`
//! (crc/edition/nblocks/nfiles ×4 u32), `name[16]`, then the embedded
//! root inode (`magic`, `mode`, `uid`, `size(3B)`, `gid`, `namelen(6b)`,
//! `offset(26b)` — packed u32/u16 fields). Version bit in flags:
//! bit 0 = sorted, bit 16 = filesystem v2 signature.
//!
//! ```
//! use izanagi_kit::cramfs;
//! let mut d = vec![0u8; 64];
//! d[0..4].copy_from_slice(&0x28CD3D45u32.to_le_bytes());
//! d[4..8].copy_from_slice(&64u32.to_le_bytes());   // size
//! d[8..12].copy_from_slice(&(1u32 << 16).to_le_bytes()); // v2 flag
//! d[16..32].copy_from_slice(b"Compressed ROMFS");
//! d[48..54].copy_from_slice(b"rootfs");
//! let c = cramfs::parse(&d).unwrap();
//! assert_eq!(c.size, 64);
//! ```

use std::vec::Vec;

/// Superblock magic (little-endian).
pub const MAGIC: u32 = 0x28CD_3D45;

/// Superblock size.
pub const SUPER_LEN: usize = 64;

/// A parsed cramfs superblock.
#[derive(Clone, Debug, PartialEq)]
pub struct Cramfs {
    /// `size` — total filesystem bytes.
    pub size: u32,
    /// `flags` bitfield (bit16 set = v2 layout).
    pub flags: u32,
    /// `fsid.edition` counter.
    pub edition: u32,
    /// `fsid.nblocks`.
    pub nblocks: u32,
    /// `fsid.nfiles`.
    pub nfiles: u32,
    /// Volume `name` (NUL-trimmed).
    pub name: Vec<u8>,
    /// Root inode `mode` (embedded inode @64, low 16 of word 0).
    pub root_mode: u16,
    /// Root inode `uid` (high 16 of word 0).
    pub root_uid: u16,
    /// Root inode `size` field (24 bits).
    pub root_size: u32,
    /// Root inode `gid` (high 8 of word 1).
    pub root_gid: u8,
    /// Root inode `namelen` (low 6 of word 2, in 4-byte units).
    pub root_namelen: u32,
    /// Root inode `offset` (high 26 of word 2, in 4-byte units).
    pub root_offset: u32,
}

fn u16le(d: &[u8], at: usize) -> Option<u16> {
    Some(*d.get(at)? as u16 | (*d.get(at + 1)? as u16) << 8)
}

fn u32le(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at.checked_add(4)?)?;
    Some(s[0] as u32 | (s[1] as u32) << 8 | (s[2] as u32) << 16 | (s[3] as u32) << 24)
}

/// Parses the 64-byte superblock; magic and `"Compressed ROMFS"` must
/// match and `size` must fit the input.
pub fn parse(d: &[u8]) -> Option<Cramfs> {
    if d.len() < SUPER_LEN {
        return None;
    }
    if u32le(d, 0)? != MAGIC {
        return None;
    }
    let size = u32le(d, 4)?;
    let flags = u32le(d, 8)?;
    if d.get(16..32)? != b"Compressed ROMFS" {
        return None;
    }
    if size != 0 && size as usize > d.len() {
        return None;
    }
    let mut name: Vec<u8> = d[48..64].to_vec();
    while name.last() == Some(&0) {
        name.pop();
    }
    // The 12-byte root inode follows the superblock at @64:
    // word0 = mode:16|uid:16, word1 = size:24|gid:8, word2 = namelen:6|offset:26.
    let (root_mode, root_uid, root_size, root_gid, root_namelen, root_offset) =
        if flags & (1 << 16) != 0 && d.len() >= SUPER_LEN + 12 {
            (
                u16le(d, SUPER_LEN)?,
                u16le(d, SUPER_LEN + 2)?,
                u32le(d, SUPER_LEN + 4)? & 0x00FF_FFFF,
                (u32le(d, SUPER_LEN + 4)? >> 24) as u8,
                u32le(d, SUPER_LEN + 8)? & 0x3F,
                u32le(d, SUPER_LEN + 8)? >> 6,
            )
        } else {
            (0, 0, 0, 0, 0, 0)
        };
    Some(Cramfs {
        size,
        flags,
        edition: u32le(d, 36)?,
        nblocks: u32le(d, 40)?,
        nfiles: u32le(d, 44)?,
        name,
        root_mode,
        root_uid,
        root_size,
        root_gid,
        root_namelen,
        root_offset,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0u8; 64 + 12];
        d[0..4].copy_from_slice(&MAGIC.to_le_bytes());
        d[4..8].copy_from_slice(&76u32.to_le_bytes());
        d[8..12].copy_from_slice(&(1u32 << 16).to_le_bytes());
        d[16..32].copy_from_slice(b"Compressed ROMFS");
        d[36..40].copy_from_slice(&5u32.to_le_bytes()); // edition
        d[40..44].copy_from_slice(&8u32.to_le_bytes());
        d[44..48].copy_from_slice(&3u32.to_le_bytes());
        d[48..54].copy_from_slice(b"rootfs");
        // root inode at offset 64: mode=dir 0755, uid=0, size=0, gid=0,
        // namelen=0, offset=8 (sectors)
        d[64..66].copy_from_slice(&0x41EDu16.to_le_bytes());
        d[72..76].copy_from_slice(&(8u32 << 6).to_le_bytes());
        d
    }

    #[test]
    fn parses_superblock() {
        let c = parse(&fixture()).unwrap();
        assert_eq!(c.size, 76);
        assert_eq!(c.flags & (1 << 16), 1 << 16);
        assert_eq!(c.edition, 5);
        assert_eq!(c.nblocks, 8);
        assert_eq!(c.name, b"rootfs".to_vec());
        assert_eq!(c.root_mode, 0x41ED);
        assert_eq!(c.root_uid, 0);
        assert_eq!(c.root_offset, 8);
        assert_eq!(c.root_namelen, 0);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 40]).is_none());
        let mut d = fixture();
        d[0] = 0xFF;
        assert!(parse(&d).is_none());
        let mut d = fixture();
        d[16] = b'X'; // signature
        assert!(parse(&d).is_none());
    }
}
