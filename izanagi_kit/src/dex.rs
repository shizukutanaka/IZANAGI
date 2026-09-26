//! Dalvik/Android `DEX` header.
//!
//! A `.dex` file opens with `dex\n` plus a version string and NUL,
//! an Adler-32 checksum, a SHA-1 signature, then the little-endian
//! map of the file: `file_size`, `header_size` (0x70), endian tag
//! `0x12345678`, the link and map offsets, and `(count, offset)`
//! pairs for the string/type/proto/field/method/class-definition
//! index tables plus the data section.
//!
//! ```
//! use izanagi_kit::dex::{parse, HEADER, ENDIAN_TAG};
//!
//! let mut d = vec![0u8; HEADER];
//! d[..4].copy_from_slice(b"dex\n");
//! d[4..7].copy_from_slice(b"035");
//! let put = |d: &mut [u8], at: usize, v: u32| {
//!     d[at] = v as u8; d[at + 1] = (v >> 8) as u8;
//!     d[at + 2] = (v >> 16) as u8; d[at + 3] = (v >> 24) as u8;
//! };
//! put(&mut d, 32, HEADER as u32);        // file_size
//! put(&mut d, 36, HEADER as u32);        // header_size
//! put(&mut d, 40, ENDIAN_TAG);
//! put(&mut d, 56, 12); put(&mut d, 60, 0x100); // string_ids
//! let x = parse(&d).unwrap();
//! assert_eq!(x.version(), "035");
//! assert_eq!(x.string_ids, (12, 0x100));
//! ```

/// Header size in bytes.
pub const HEADER: usize = 0x70;
/// Little-endian tag at offset 40.
pub const ENDIAN_TAG: u32 = 0x1234_5678;
/// Reversed-endian tag.
pub const ENDIAN_TAG_REV: u32 = 0x7856_3412;

fn le32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(at)?)
            | u32::from(*d.get(at + 1)?) << 8
            | u32::from(*d.get(at + 2)?) << 16
            | u32::from(*d.get(at + 3)?) << 24,
    )
}

/// A parsed DEX header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dex {
    /// Three-byte version tag, e.g. `b"035"`.
    pub ver: [u8; 3],
    /// Adler-32 checksum of everything after offset 12.
    pub checksum: u32,
    /// SHA-1 signature of everything after offset 32.
    pub signature: [u8; 20],
    /// Declared file size.
    pub file_size: u32,
    /// Endianness tag (normally `ENDIAN_TAG`).
    pub endian: u32,
    /// Link section `(size, offset)`.
    pub link: (u32, u32),
    /// Map list offset.
    pub map_at: u32,
    /// String index `(count, offset)`.
    pub string_ids: (u32, u32),
    /// Type index `(count, offset)`.
    pub type_ids: (u32, u32),
    /// Prototype index `(count, offset)`.
    pub proto_ids: (u32, u32),
    /// Field index `(count, offset)`.
    pub field_ids: (u32, u32),
    /// Method index `(count, offset)`.
    pub method_ids: (u32, u32),
    /// Class definition `(count, offset)`.
    pub class_defs: (u32, u32),
    /// Data section `(size, offset)`.
    pub data: (u32, u32),
}

impl Dex {
    /// The version string ("035", "036", "037", "038", "039", ...).
    pub fn version(&self) -> &str {
        core::str::from_utf8(&self.ver).unwrap_or("")
    }
    /// True for the standard little-endian tag.
    pub fn little_endian(&self) -> bool {
        self.endian == ENDIAN_TAG
    }
}

/// Parse a header. Returns `None` on a bad magic, a wrong
/// `header_size`, or a truncated header.
pub fn parse(d: &[u8]) -> Option<Dex> {
    if d.get(..4)? != b"dex\n" {
        return None;
    }
    if d.len() < HEADER {
        return None;
    }
    if le32(d, 36)? != HEADER as u32 {
        return None;
    }
    let mut ver = [0u8; 3];
    ver.copy_from_slice(d.get(4..7)?);
    let mut signature = [0u8; 20];
    signature.copy_from_slice(d.get(12..32)?);
    Some(Dex {
        ver,
        checksum: le32(d, 8)?,
        signature,
        file_size: le32(d, 32)?,
        endian: le32(d, 40)?,
        link: (le32(d, 44)?, le32(d, 48)?),
        map_at: le32(d, 52)?,
        string_ids: (le32(d, 56)?, le32(d, 60)?),
        type_ids: (le32(d, 64)?, le32(d, 68)?),
        proto_ids: (le32(d, 72)?, le32(d, 76)?),
        field_ids: (le32(d, 80)?, le32(d, 84)?),
        method_ids: (le32(d, 88)?, le32(d, 92)?),
        class_defs: (le32(d, 96)?, le32(d, 100)?),
        data: (le32(d, 104)?, le32(d, 108)?),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0u8; 0x200];
        d[..4].copy_from_slice(b"dex\n");
        d[4..7].copy_from_slice(b"039");
        let w = |d: &mut [u8], o: usize, v: u32| {
            d[o] = v as u8;
            d[o + 1] = (v >> 8) as u8;
            d[o + 2] = (v >> 16) as u8;
            d[o + 3] = (v >> 24) as u8;
        };
        w(&mut d, 8, 0xaabb_ccdd);
        for i in 0..20 {
            d[12 + i] = i as u8;
        }
        w(&mut d, 32, 0x200);
        w(&mut d, 36, HEADER as u32);
        w(&mut d, 40, ENDIAN_TAG);
        w(&mut d, 44, 0);
        w(&mut d, 48, 0);
        w(&mut d, 52, 0x80);
        w(&mut d, 56, 10);
        w(&mut d, 60, 0x70);
        w(&mut d, 64, 5);
        w(&mut d, 68, 0xa0);
        w(&mut d, 72, 3);
        w(&mut d, 76, 0xb0);
        w(&mut d, 80, 2);
        w(&mut d, 84, 0xc0);
        w(&mut d, 88, 4);
        w(&mut d, 92, 0xd0);
        w(&mut d, 96, 1);
        w(&mut d, 100, 0xe0);
        w(&mut d, 104, 0x100);
        w(&mut d, 108, 0x100);
        d
    }

    #[test]
    fn fields_decode() {
        let d = fixture();
        let x = parse(&d).unwrap();
        assert_eq!(x.version(), "039");
        assert_eq!(x.checksum, 0xaabb_ccdd);
        assert_eq!(x.signature[0], 0);
        assert_eq!(x.signature[19], 19);
        assert_eq!(x.file_size, 0x200);
        assert!(x.little_endian());
        assert_eq!(x.map_at, 0x80);
        assert_eq!(x.string_ids, (10, 0x70));
        assert_eq!(x.type_ids, (5, 0xa0));
        assert_eq!(x.proto_ids, (3, 0xb0));
        assert_eq!(x.field_ids, (2, 0xc0));
        assert_eq!(x.method_ids, (4, 0xd0));
        assert_eq!(x.class_defs, (1, 0xe0));
        assert_eq!(x.data, (0x100, 0x100));
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 32]).is_none());
        let mut d = fixture();
        d[0] = b'x';
        assert!(parse(&d).is_none());
        d[0] = b'd';
        d[37] = 0x60; // wrong header_size
        assert!(parse(&d).is_none());
    }
}
