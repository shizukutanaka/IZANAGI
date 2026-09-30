//! ASTM E2807 E57 — 3D imaging system point-cloud container.
//!
//! 48-byte fixed header (all little-endian): `ASTM-E57` magic, u32 major,
//! u32 minor, u64 file_length, u64 xml_offset, u32 xml_length, u64 page_size.
//!
//! ```
//! let mut d = b"ASTM-E57".to_vec();
//! d.extend_from_slice(&1u32.to_le_bytes()); // major
//! d.extend_from_slice(&0u32.to_le_bytes()); // minor
//! d.extend_from_slice(&48u64.to_le_bytes()); // file_length
//! d.extend_from_slice(&0u64.to_le_bytes()); // xml_offset
//! d.extend_from_slice(&0u32.to_le_bytes()); // xml_length
//! d.extend_from_slice(&1024u64.to_le_bytes()); // page_size
//! d.extend_from_slice(&0u32.to_le_bytes()); // reserved
//! let e = izanagi_kit::e57::parse(&d).unwrap();
//! assert_eq!((e.major, e.minor), (1, 0));
//! assert_eq!(e.page_size, 1024);
//! ```

/// Parsed E57 file header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct E57 {
    /// Format major version (1 today).
    pub major: u32,
    /// Format minor version.
    pub minor: u32,
    /// Declared total file length in bytes.
    pub file_len: u64,
    /// Offset of the XML section.
    pub xml_offset: u64,
    /// Byte length of the XML section.
    pub xml_len: u32,
    /// Binary page size (should be a multiple of 4; commonly 1024).
    pub page_size: u64,
}

fn le32(d: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(o..o + 4)?.try_into().ok()?))
}

fn le64(d: &[u8], o: usize) -> Option<u64> {
    Some(u64::from_le_bytes(d.get(o..o + 8)?.try_into().ok()?))
}

/// Parse the 48-byte E57 header; `None` on short input or bad magic.
pub fn parse(d: &[u8]) -> Option<E57> {
    if !d.starts_with(b"ASTM-E57") {
        return None;
    }
    let major = le32(d, 8)?;
    let minor = le32(d, 12)?;
    let file_len = le64(d, 16)?;
    let xml_offset = le64(d, 24)?;
    let xml_len = le32(d, 32)?;
    let page_size = le64(d, 36)?;
    if page_size == 0 || page_size % 4 != 0 {
        return None;
    }
    Some(E57 {
        major,
        minor,
        file_len,
        xml_offset,
        xml_len,
        page_size,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hdr(major: u32, minor: u32, len: u64, xoff: u64, xlen: u32, page: u64) -> Vec<u8> {
        let mut d = b"ASTM-E57".to_vec();
        d.extend_from_slice(&major.to_le_bytes());
        d.extend_from_slice(&minor.to_le_bytes());
        d.extend_from_slice(&len.to_le_bytes());
        d.extend_from_slice(&xoff.to_le_bytes());
        d.extend_from_slice(&xlen.to_le_bytes());
        d.extend_from_slice(&page.to_le_bytes());
        d.extend_from_slice(&0u32.to_le_bytes()); // reserved → 48-byte header
        d
    }

    #[test]
    fn basic() {
        let e = parse(&hdr(1, 0, 4096, 2048, 512, 1024)).unwrap();
        assert_eq!((e.major, e.minor), (1, 0));
        assert_eq!(e.file_len, 4096);
        assert_eq!((e.xml_offset, e.xml_len), (2048, 512));
        assert_eq!(e.page_size, 1024);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"ASTM-E56").is_none());
        assert!(parse(&hdr(1, 0, 48, 0, 0, 0)).is_none()); // page_size 0
        assert!(parse(&hdr(1, 0, 48, 0, 0, 999)).is_none()); // not a multiple of 4
    }
}
