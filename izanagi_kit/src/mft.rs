//! NTFS `$MFT` FILE record parsing.
//!
//! Each 1024-byte record starts `FILE` (or `BAAD`/`CHKD`/`HOLE` on
//! bad sectors). The header carries `usa_offset`/`usa_count` (the
//! fixup array), `sequence u16`, `link_count u16`,
//! `attr_offset u16`, `flags u16` (bit0 in-use, bit1 directory),
//! `used_size`/`alloc_size u32`, `base_record u64`,
//! `next_attr_id u16` and `record_number u32` (v2+). Attributes
//! follow at `attr_offset`: `type u32LE` + `len u32` each,
//! terminated by `0xFFFFFFFF`.
//!
//! ```
//! use izanagi_kit::mft;
//! let mut d = vec![0u8; 1024];
//! d[0..4].copy_from_slice(b"FILE");
//! d[4..6].copy_from_slice(&0x30u16.to_le_bytes()); // usa offset
//! d[6..8].copy_from_slice(&3u16.to_le_bytes()); // usa count
//! d[20..22].copy_from_slice(&0x38u16.to_le_bytes()); // attrs at 0x38
//! d[22..24].copy_from_slice(&3u16.to_le_bytes()); // in-use | dir
//! d[0x38..0x3C].copy_from_slice(&0xFFFFFFFFu32.to_le_bytes()); // end
//! let r = mft::parse(&d).unwrap();
//! assert!(r.in_use && r.is_dir);
//! ```

use std::vec::Vec;

/// FILE record signature.
pub const MAGIC: &[u8; 4] = b"FILE";
/// Attribute list terminator.
pub const ATTR_END: u32 = 0xFFFF_FFFF;

/// Attribute type ids seen in a record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Attr {
    /// 0x10 — `$STANDARD_INFORMATION`.
    StandardInfo,
    /// 0x20 — `$ATTRIBUTE_LIST`.
    AttributeList,
    /// 0x30 — `$FILE_NAME`.
    FileName,
    /// 0x80 — `$DATA`.
    Data,
    /// 0x90 — `$INDEX_ROOT`.
    IndexRoot,
    /// 0xA0 — `$INDEX_ALLOCATION`.
    IndexAlloc,
    /// 0xB0 — `$BITMAP`.
    Bitmap,
    /// Other type id.
    Other(u32),
}

/// One attribute header (resident or non-resident).
#[derive(Clone, Debug, PartialEq)]
pub struct Attribute {
    /// Type id.
    pub ty: u32,
    /// Total attribute length.
    pub len: u32,
    /// Non-resident flag byte (0 = resident).
    pub non_resident: u8,
    /// Offset of this attribute within the record.
    pub offset: usize,
}

/// A parsed FILE record.
#[derive(Clone, Debug, PartialEq)]
pub struct Mft {
    /// `flags & 1` — record in use.
    pub in_use: bool,
    /// `flags & 2` — record is a directory.
    pub is_dir: bool,
    /// `sequence` number.
    pub sequence: u16,
    /// `link_count`.
    pub link_count: u16,
    /// `used_size` — bytes of the record in use.
    pub used_size: u32,
    /// `alloc_size` — record allocation (usually 1024).
    pub alloc_size: u32,
    /// `record_number` (only when the record is long enough).
    pub record_number: u32,
    /// Attribute headers until `ATTR_END`.
    pub attrs: Vec<Attribute>,
}

fn u16le(d: &[u8], at: usize) -> Option<u16> {
    let s = d.get(at..at.checked_add(2)?)?;
    Some((s[0] as u16) | (s[1] as u16) << 8)
}

fn u32le(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at.checked_add(4)?)?;
    Some((s[0] as u32) | (s[1] as u32) << 8 | (s[2] as u32) << 16 | (s[3] as u32) << 24)
}

/// Parses a FILE record: `FILE` magic, USA window inside the record,
/// `attr_offset` inside the record, attribute chain bounds-checked,
/// `used_size ≤ alloc_size ≤ record length`.
pub fn parse(d: &[u8]) -> Option<Mft> {
    if d.len() < 48 || d.get(..4)? != MAGIC {
        return None;
    }
    let usa_offset = u16le(d, 4)? as usize;
    let usa_count = u16le(d, 6)? as usize;
    if usa_offset == 0 || usa_offset.checked_add(usa_count.checked_mul(2)?)? > d.len() {
        return None;
    }
    let attr_offset = u16le(d, 20)? as usize;
    if attr_offset >= d.len() {
        return None;
    }
    let flags = u16le(d, 22)?;
    let used_size = u32le(d, 24)?;
    let alloc_size = u32le(d, 28)?;
    if used_size > alloc_size || alloc_size as usize > d.len() {
        return None;
    }
    let record_number = u32le(d, 44).unwrap_or(0);
    let mut attrs = Vec::new();
    let mut at = attr_offset;
    loop {
        let ty = u32le(d, at)?;
        if ty == ATTR_END {
            break;
        }
        let len = u32le(d, at + 4)?;
        if len < 16 || at.checked_add(len as usize)? > alloc_size.min(d.len() as u32) as usize {
            return None;
        }
        let non_resident = *d.get(at + 8)?;
        attrs.push(Attribute {
            ty,
            len,
            non_resident,
            offset: at,
        });
        at += len as usize;
        if at >= d.len() {
            return None;
        }
        if attrs.len() > 256 {
            return None;
        }
    }
    Some(Mft {
        in_use: flags & 1 != 0,
        is_dir: flags & 2 != 0,
        sequence: u16le(d, 16)?,
        link_count: u16le(d, 18)?,
        used_size,
        alloc_size,
        record_number,
        attrs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0u8; 1024];
        d[0..4].copy_from_slice(MAGIC);
        d[4..6].copy_from_slice(&0x30u16.to_le_bytes());
        d[6..8].copy_from_slice(&3u16.to_le_bytes());
        d[16..18].copy_from_slice(&7u16.to_le_bytes()); // seq
        d[20..22].copy_from_slice(&0x38u16.to_le_bytes());
        d[22..24].copy_from_slice(&1u16.to_le_bytes()); // in use
        d[24..28].copy_from_slice(&512u32.to_le_bytes());
        d[28..32].copy_from_slice(&1024u32.to_le_bytes());
        // one $FILE_NAME attribute (0x30), then end
        d[0x38..0x3C].copy_from_slice(&0x30u32.to_le_bytes());
        d[0x3C..0x40].copy_from_slice(&0x68u32.to_le_bytes());
        d[0x38 + 0x68..0x38 + 0x6C].copy_from_slice(&ATTR_END.to_le_bytes());
        d
    }

    #[test]
    fn parses_record() {
        let r = parse(&fixture()).unwrap();
        assert!(r.in_use);
        assert!(!r.is_dir);
        assert_eq!(r.sequence, 7);
        assert_eq!(r.attrs.len(), 1);
        assert_eq!(r.attrs[0].ty, 0x30);
        assert_eq!(r.alloc_size, 1024);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 64]).is_none());
        let mut d = fixture();
        d[0..4].copy_from_slice(b"BAAD"); // bad-sector signature
        assert!(parse(&d).is_none());
        let mut d = fixture();
        d[28..32].copy_from_slice(&200u32.to_le_bytes()); // alloc < used
        d[24..28].copy_from_slice(&512u32.to_le_bytes());
        assert!(parse(&d).is_none());
    }
}
