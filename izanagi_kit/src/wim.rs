//! Microsoft WIM (Windows Imaging Format, `.wim`): `MSWIM\0\0\0`
//! magic + `u32le` header — cbSize, dwVersion, dwFlags,
//! dwCompressionType, GUID, part numbers, dwImageCount and the
//! `RESHDR` resource table offsets (offset table, XML data,
//! integrity table).
//!
//! ```
//! let mut d = b"MSWIM\0\0\0".to_vec();
//! d.extend_from_slice(&[208, 0, 0, 0]); // cbSize
//! d.extend_from_slice(&[0xD0, 0x01, 0x00, 0x00]); // version
//! d.extend_from_slice(&[0; 4]); // flags
//! d.extend_from_slice(&[2, 0, 0, 0]); // LZX
//! d.extend_from_slice(&[0; 16]); // GUID
//! d.extend_from_slice(&[1, 0, 1, 0]); // part/total
//! d.extend_from_slice(&[2, 0, 0, 0]); // image count
//! d.resize(208, 0);
//! let p = izanagi_kit::wim::parse(&d).unwrap();
//! assert_eq!(p.image_count, 2);
//! assert_eq!(p.compression, izanagi_kit::wim::Compression::Lzx);
//! assert!(izanagi_kit::wim::detect(&d));
//! ```

/// WIM `dwCompressionType` value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
    /// `RESERVED` (uncompressed).
    None,
    /// XPRESS (LZX-fast, 4K chunks).
    Xpress,
    /// LZX.
    Lzx,
    /// LZMS.
    Lzms,
    /// Solid LZX.
    LzxSolid,
    /// Solid XPRESS.
    XpressSolid,
    /// Any other value.
    Other(u32),
}

/// Census of a WIM archive header.
#[derive(Debug, Clone, PartialEq)]
pub struct Wim {
    /// `cbSize` header size (normally 208).
    pub header_size: u32,
    /// `dwVersion` raw version word.
    pub version: u32,
    /// `dwFlags` feature flags (integrity/RP_FIX/spanned…).
    pub flags: u32,
    /// Compression type.
    pub compression: Compression,
    /// `usPartNumber` (split archives).
    pub part_number: u16,
    /// `usTotalParts`.
    pub total_parts: u16,
    /// `dwImageCount` images in the archive.
    pub image_count: u32,
    /// Offset-table resource offset.
    pub offset_table_off: u64,
    /// Offset-table resource size.
    pub offset_table_size: u64,
    /// XML data resource offset.
    pub xml_data_off: u64,
    /// XML data resource size.
    pub xml_data_size: u64,
    /// Integrity-table resource present.
    pub has_integrity: bool,
    /// `RH_` resource flag word of the offset table (high byte region).
    pub offset_table_flags: u64,
    /// Declared XML data overruns the buffer.
    pub truncated: bool,
}

fn le32(b: &[u8], i: usize) -> u32 {
    b[i] as u32 | ((b[i + 1] as u32) << 8) | ((b[i + 2] as u32) << 16) | ((b[i + 3] as u32) << 24)
}
fn le16(b: &[u8], i: usize) -> u16 {
    b[i] as u16 | ((b[i + 1] as u16) << 8)
}
fn le64(b: &[u8], i: usize) -> u64 {
    let mut v = 0u64;
    for (s, &c) in b[i..i + 8].iter().enumerate() {
        v |= (c as u64) << (s * 8);
    }
    v
}
/// RESHDR_DISK_SHORT: `liOffset`(8) + `liSize`(7) + `usFlags`(1) → packed 16B.
fn reshdr(b: &[u8], i: usize) -> (u64, u64, u64) {
    let off = le64(b, i);
    let mut size = 0u64;
    for (s, &c) in b[i + 8..i + 15].iter().enumerate() {
        size |= (c as u64) << (s * 8);
    }
    (off, size, b[i + 15] as u64)
}

/// `true` on `MSWIM\0\0\0` (or the `MSWIMOLD` legacy magic).
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 12 && (b[..8] == *b"MSWIM\0\0\0" || b[..8] == *b"MSWIMOLD")
}

/// Census; `None` without the magic. Fields follow the 208-byte
/// `WIMHEADER_V1_PACKED` layout.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Wim> {
    if !detect(b) || b.len() < 48 {
        return None;
    }
    let compression = match le32(b, 20) {
        0 => Compression::None,
        1 => Compression::Xpress,
        2 => Compression::Lzx,
        4 => Compression::Lzms,
        8 => Compression::LzxSolid,
        16 => Compression::XpressSolid,
        other => Compression::Other(other),
    };
    let mut w = Wim {
        header_size: le32(b, 8),
        version: le32(b, 12),
        flags: le32(b, 16),
        compression,
        part_number: le16(b, 40),
        total_parts: le16(b, 42),
        image_count: le32(b, 44),
        offset_table_off: 0,
        offset_table_size: 0,
        offset_table_flags: 0,
        xml_data_off: 0,
        xml_data_size: 0,
        has_integrity: false,
        truncated: false,
    };
    // RESHDR entries start at 48 (each 24 bytes on disk: 8+8+8 padded).
    // Offsets table @48, XML @72, boot metadata @96, integrity @120.
    if b.len() >= 72 {
        let (off, size, fl) = reshdr(b, 48);
        w.offset_table_off = off;
        w.offset_table_size = size;
        w.offset_table_flags = fl;
    }
    if b.len() >= 96 {
        let (off, size, _) = reshdr(b, 72);
        w.xml_data_off = off;
        w.xml_data_size = size;
        if off != 0 && off.saturating_add(size) > b.len() as u64 {
            w.truncated = true;
        }
    }
    if b.len() >= 144 {
        let (off, _, _) = reshdr(b, 120);
        w.has_integrity = off != 0;
    }
    if w.offset_table_off != 0
        && w.offset_table_off.saturating_add(w.offset_table_size) > b.len() as u64
    {
        w.truncated = true;
    }
    Some(w)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = b"MSWIM\0\0\0".to_vec();
        d.extend_from_slice(&[208, 0, 0, 0]);
        d.extend_from_slice(&[0xD0, 0x01, 0x00, 0x00]);
        d.extend_from_slice(&[0x10, 0, 0, 0]); // flags: integrity
        d.extend_from_slice(&[2, 0, 0, 0]); // LZX
        d.extend_from_slice(&[0; 16]);
        d.extend_from_slice(&[1, 0, 1, 0]);
        d.extend_from_slice(&[2, 0, 0, 0]);
        d.resize(208, 0);
        // offset table reshdr @48: off 1024, size 512
        d[48..56].copy_from_slice(&1024u64.to_le_bytes());
        d[56..63].copy_from_slice(&512u64.to_le_bytes()[..7]);
        // integrity reshdr @120: nonzero offset
        d[120] = 7;
        d
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        assert!(!detect(b"MSWIMX"));
        assert!(!detect(&fixture()[..4]));
    }

    #[test]
    fn parses_header() {
        let p = parse(&fixture()).unwrap();
        assert_eq!(p.header_size, 208);
        assert_eq!(p.compression, Compression::Lzx);
        assert_eq!(p.image_count, 2);
        assert_eq!(p.part_number, 1);
        assert_eq!(p.offset_table_off, 1024);
        assert_eq!(p.offset_table_size, 512);
        assert!(p.has_integrity);
        assert!(p.truncated); // offsets point past the fixture
    }

    #[test]
    fn short_header_none() {
        assert!(parse(&fixture()[..40]).is_none());
    }

    #[test]
    fn rejects_plain() {
        assert!(parse(b"not wim").is_none());
    }
}
