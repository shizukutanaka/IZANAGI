//! Valve VPK (Valve PaK) archive header.
//!
//! A VPK directory file opens with `0x55AA1234`, a version (1 or
//! 2) and the directory-tree size; version 2 appends the file-data,
//! archive-MD5, other-MD5 and signature section sizes. `parse`
//! bounds-checks the declared tree and reports each section.
//!
//! ```
//! use izanagi_kit::vpk::{parse, MAGIC};
//!
//! let mut d = vec![0u8; 64];
//! let w = |d: &mut [u8], o: usize, v: u32| {
//!     for i in 0..4 { d[o + i] = (v >> (i * 8)) as u8; }
//! };
//! w(&mut d, 0, MAGIC);
//! w(&mut d, 4, 2);            // version
//! w(&mut d, 8, 20);           // tree size
//! w(&mut d, 12, 8);           // file data section
//! w(&mut d, 16, 0);           // archive md5
//! w(&mut d, 20, 0);           // other md5
//! w(&mut d, 24, 0);           // signature
//! let v = parse(&d).unwrap();
//! assert_eq!(v.tree_end(), 48);
//! ```

/// Header signature.
pub const MAGIC: u32 = 0x55aa_1234;
/// v1 header size.
pub const HEADER_V1: usize = 12;
/// v2 header size.
pub const HEADER_V2: usize = 28;

fn le32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(at)?)
            | u32::from(*d.get(at + 1)?) << 8
            | u32::from(*d.get(at + 2)?) << 16
            | u32::from(*d.get(at + 3)?) << 24,
    )
}

/// A parsed VPK header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Vpk {
    /// Version (1 or 2).
    pub version: u32,
    /// `tree_size` — byte length of the ext/path/name tree.
    pub tree_size: u32,
    /// v2: file-data section bytes appended after the tree.
    pub file_data_size: u32,
    /// v2: archive-MD5 section bytes.
    pub archive_md5_size: u32,
    /// v2: other-MD5 section bytes.
    pub other_md5_size: u32,
    /// v2: signature section bytes.
    pub signature_size: u32,
}

impl Vpk {
    /// Declared header size by version.
    pub fn header_size(&self) -> usize {
        if self.version >= 2 {
            HEADER_V2
        } else {
            HEADER_V1
        }
    }
    /// Byte offset where the tree ends.
    pub fn tree_end(&self) -> u64 {
        self.header_size() as u64 + u64::from(self.tree_size)
    }
    /// Byte offset of the file-data section (v2).
    pub fn file_data_at(&self) -> u64 {
        self.tree_end()
    }
    /// Byte offset of the archive-MD5 section (v2).
    pub fn archive_md5_at(&self) -> u64 {
        self.tree_end() + u64::from(self.file_data_size)
    }
    /// Byte offset of the signature section (v2).
    pub fn signature_at(&self) -> u64 {
        self.archive_md5_at() + u64::from(self.archive_md5_size) + u64::from(self.other_md5_size)
    }
    /// Total declared file size.
    pub fn total_len(&self) -> u64 {
        self.signature_at() + u64::from(self.signature_size)
    }
}

/// Parse a VPK header. Returns `None` on a bad magic/version or
/// declared sections that escape the buffer.
pub fn parse(d: &[u8]) -> Option<Vpk> {
    if le32(d, 0)? != MAGIC {
        return None;
    }
    let version = le32(d, 4)?;
    let v = if version == 1 {
        Vpk {
            version,
            tree_size: le32(d, 8)?,
            file_data_size: 0,
            archive_md5_size: 0,
            other_md5_size: 0,
            signature_size: 0,
        }
    } else if version == 2 {
        Vpk {
            version,
            tree_size: le32(d, 8)?,
            file_data_size: le32(d, 12)?,
            archive_md5_size: le32(d, 16)?,
            other_md5_size: le32(d, 20)?,
            signature_size: le32(d, 24)?,
        }
    } else {
        return None;
    };
    if v.total_len() > d.len() as u64 {
        return None;
    }
    Some(v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn fixture(ver: u32) -> Vec<u8> {
        let mut d = vec![0u8; 128];
        let w = |d: &mut [u8], o: usize, v: u32| {
            for i in 0..4 {
                d[o + i] = (v >> (i * 8)) as u8;
            }
        };
        w(&mut d, 0, MAGIC);
        w(&mut d, 4, ver);
        w(&mut d, 8, 40);
        if ver == 2 {
            w(&mut d, 12, 8); // file data
            w(&mut d, 16, 4); // archive md5
            w(&mut d, 20, 4); // other md5
            w(&mut d, 24, 4); // signature
        }
        d
    }

    #[test]
    fn v1_fields() {
        let v = parse(&fixture(1)).unwrap();
        assert_eq!(v.version, 1);
        assert_eq!(v.tree_size, 40);
        assert_eq!(v.header_size(), 12);
        assert_eq!(v.tree_end(), 52);
        assert_eq!(v.total_len(), 52);
    }

    #[test]
    fn v2_sections() {
        let v = parse(&fixture(2)).unwrap();
        assert_eq!(v.header_size(), 28);
        assert_eq!(v.tree_end(), 68);
        assert_eq!(v.file_data_at(), 68);
        assert_eq!(v.archive_md5_at(), 76);
        assert_eq!(v.signature_at(), 84);
        assert_eq!(v.total_len(), 88);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 8]).is_none());
        let mut bad = fixture(2);
        bad[0] = 0;
        assert!(parse(&bad).is_none());
        let mut bad2 = fixture(2);
        bad2[4] = 9; // version
        assert!(parse(&bad2).is_none());
        let mut bad3 = fixture(2);
        bad3[8] = 200; // tree past buffer
        assert!(parse(&bad3).is_none());
    }
}
