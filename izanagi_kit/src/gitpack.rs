//! Git `PACK` file header + object-entry header parsing.
//!
//! Header: `PACK` + u32BE `version` (2 or 3) + u32BE `num_objects`.
//! Each entry then starts with a variable-length type+size field:
//! first byte `cont:1 | type:3 | size:4`, continuation bytes add 7
//! bits each (MSB = continue). Object data (zlib) is left opaque —
//! compressed sizes aren't encoded, so only entry *headers* decode.
//!
//! ```
//! use izanagi_kit::gitpack;
//! let mut d = b"PACK".to_vec();
//! d.extend_from_slice(&2u32.to_be_bytes()); // version
//! d.extend_from_slice(&7u32.to_be_bytes()); // count
//! // one entry header: type=1 (commit), size=20
//! // (low nibble 4, continuation 20>>4 = 1)
//! d.extend_from_slice(&[0x94, 0x01]);
//! let p = gitpack::parse(&d).unwrap();
//! assert_eq!(p.num_objects, 7);
//! let (ty, size, hdr_len) = gitpack::entry_header(&d, 12).unwrap();
//! assert_eq!((size, hdr_len), (20, 2));
//! assert_eq!(ty, gitpack::ObjType::Commit);
//! ```

/// File magic.
pub const MAGIC: &[u8; 4] = b"PACK";

/// Object type ids.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjType {
    /// 1 — commit.
    Commit,
    /// 2 — tree.
    Tree,
    /// 3 — blob.
    Blob,
    /// 4 — annotated tag.
    Tag,
    /// 6 — OFS_DELTA.
    OfsDelta,
    /// 7 — REF_DELTA.
    RefDelta,
    /// Reserved/unknown.
    Reserved(u8),
}

fn kind(t: u8) -> ObjType {
    match t {
        1 => ObjType::Commit,
        2 => ObjType::Tree,
        3 => ObjType::Blob,
        4 => ObjType::Tag,
        6 => ObjType::OfsDelta,
        7 => ObjType::RefDelta,
        o => ObjType::Reserved(o),
    }
}

/// A parsed `PACK` header.
#[derive(Clone, Debug, PartialEq)]
pub struct Pack {
    /// Pack format version (2 or 3).
    pub version: u32,
    /// Object count.
    pub num_objects: u32,
    /// Offset of the first object entry (12).
    pub entries_offset: usize,
}

fn u32be(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at.checked_add(4)?)?;
    Some((s[0] as u32) << 24 | (s[1] as u32) << 16 | (s[2] as u32) << 8 | s[3] as u32)
}

/// Parses a `PACK` header; requires magic and version 2 or 3.
pub fn parse(d: &[u8]) -> Option<Pack> {
    if d.len() < 12 {
        return None;
    }
    if d.get(..4)? != MAGIC {
        return None;
    }
    let version = u32be(d, 4)?;
    if !(2..=3).contains(&version) {
        return None;
    }
    Some(Pack {
        version,
        num_objects: u32be(d, 8)?,
        entries_offset: 12,
    })
}

/// Reads one object-entry header at `at`: `(type, unpacked_size,
/// header_len)`. The caller consumes compressed payload separately.
pub fn entry_header(d: &[u8], at: usize) -> Option<(ObjType, u64, usize)> {
    let b0 = *d.get(at)?;
    let ty = (b0 >> 4) & 7;
    let mut size = (b0 & 0x0F) as u64;
    let mut shift = 4u32;
    let mut i = 1usize;
    if b0 & 0x80 == 0 {
        return Some((kind(ty), size, i));
    }
    loop {
        let b = *d.get(at.checked_add(i)?)?;
        i += 1;
        size |= ((b & 0x7F) as u64) << shift;
        shift += 7;
        if shift > 63 {
            return None;
        }
        if b & 0x80 == 0 {
            break;
        }
    }
    Some((kind(ty), size, i))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_header() {
        let mut d = MAGIC.to_vec();
        d.extend_from_slice(&2u32.to_be_bytes());
        d.extend_from_slice(&5u32.to_be_bytes());
        let p = parse(&d).unwrap();
        assert_eq!(p.version, 2);
        assert_eq!(p.num_objects, 5);
    }

    #[test]
    fn decodes_entry_headers() {
        // single byte: type=3 blob, size=12
        let (t, s, n) = entry_header(&[0x3C], 0).unwrap();
        assert_eq!((t, s, n), (ObjType::Blob, 12, 1));
        // two bytes: cont|type=2|low4=1 then 0x40 → size = 1 | 0x40<<4
        let (t, s, n) = entry_header(&[0xA1, 0x40], 0).unwrap();
        assert_eq!((t, s, n), (ObjType::Tree, 1 | (0x40 << 4), 2));
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 8]).is_none());
        let mut d = b"PACK".to_vec();
        d.extend_from_slice(&4u32.to_be_bytes());
        d.extend_from_slice(&0u32.to_be_bytes());
        assert!(parse(&d).is_none()); // version 4
    }
}
