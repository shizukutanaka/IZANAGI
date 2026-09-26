//! FBX — the Kaydara binary interchange file. Bytes 0..23 are the
//! magic `Kaydara FBX Binary  \x00\x1A\x00` and a u32LE version. Node
//! records follow: version < 7500 uses 32-bit `{end_offset,
//! property_count, property_bytes}` + u8 name length; 7500+ widens all
//! three to u64. A node whose `end_offset` is 0 (the 13/25-byte null
//! record) ends its level.
//!
//! ```
//! use izanagi_kit::fbx::{parse, nodes};
//! let mut d = b"Kaydara FBX Binary  \x00\x1A\x00".to_vec();
//! d.extend_from_slice(&7400u32.to_le_bytes());
//! // one node "Objects" covering the rest, then the null record
//! let end = 27 + 13 + 7 + 13;
//! d.extend_from_slice(&(end as u32).to_le_bytes());
//! d.extend_from_slice(&0u32.to_le_bytes());
//! d.extend_from_slice(&0u32.to_le_bytes());
//! d.push(7);
//! d.extend_from_slice(b"Objects");
//! d.extend_from_slice(&[0; 13]); // level terminator
//! d.extend_from_slice(&[0; 13]); // file footer null record
//! let f = parse(&d).unwrap();
//! assert_eq!(f.version, 7400);
//! assert_eq!(nodes(&f, &d).next().unwrap().name, b"Objects");
//! ```

/// Magic bytes at offset 0 (21 bytes + `\x1A\x00`).
pub const MAGIC: &[u8; 23] = b"Kaydara FBX Binary  \x00\x1A\x00";
/// First format version that uses 64-bit node fields.
pub const WIDE_VERSION: u32 = 7500;

/// A parsed FBX header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fbx {
    /// Format version (e.g. 7400, 7700).
    pub version: u32,
    /// True when node headers use 64-bit fields (`version >= 7500`).
    pub wide: bool,
}

/// One node header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    /// Node name bytes.
    pub name: Vec<u8>,
    /// Byte offset of this node record.
    pub at: usize,
    /// Absolute offset where the node (children + own content) ends.
    pub end: u64,
    /// Declared number of properties.
    pub properties: u64,
    /// Byte length of the property list.
    pub property_bytes: u64,
}

fn u64l(d: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(d.get(at..at + 8)?.try_into().ok()?))
}

/// Parse the magic + version.
pub fn parse(d: &[u8]) -> Option<Fbx> {
    if d.get(..MAGIC.len())? != MAGIC {
        return None;
    }
    let version = u32::from_le_bytes(d.get(23..27)?.try_into().ok()?);
    Some(Fbx {
        version,
        wide: version >= WIDE_VERSION,
    })
}

/// Iterate top-level node headers; stops at the null record or EOF.
pub fn nodes<'d>(f: &'d Fbx, d: &'d [u8]) -> impl Iterator<Item = Node> + 'd {
    let mut at = MAGIC.len() + 4;
    core::iter::from_fn(move || {
        let (end, props, plen, name_at) = if f.wide {
            (
                u64l(d, at)?,
                u64l(d, at + 8)?,
                u64l(d, at + 16)?,
                at.checked_add(25)?,
            )
        } else {
            (
                u64::from(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?)),
                u64::from(u32::from_le_bytes(d.get(at + 4..at + 8)?.try_into().ok()?)),
                u64::from(u32::from_le_bytes(d.get(at + 8..at + 12)?.try_into().ok()?)),
                at.checked_add(13)?,
            )
        };
        if end == 0 {
            return None; // null record
        }
        let name_len = *d.get(if f.wide { at + 24 } else { at + 12 })? as usize;
        let name = d.get(name_at..name_at + name_len)?.to_vec();
        let n = Node {
            name,
            at,
            end,
            properties: props,
            property_bytes: plen,
        };
        at = usize::try_from(end).ok()?;
        Some(n)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = b"Kaydara FBX Binary  \x00\x1A\x00".to_vec();
        d.extend_from_slice(&7400u32.to_le_bytes());
        let end = 27 + 13 + 3 + 13;
        d.extend_from_slice(&(end as u32).to_le_bytes());
        d.extend_from_slice(&1u32.to_le_bytes());
        d.extend_from_slice(&9u32.to_le_bytes());
        d.push(3);
        d.extend_from_slice(b"Foo");
        d.extend_from_slice(&[0; 13]);
        d.extend_from_slice(&[0; 13]);
        d
    }

    #[test]
    fn parses_nodes() {
        let d = fixture();
        let f = parse(&d).unwrap();
        assert_eq!(f.version, 7400);
        assert!(!f.wide);
        let ns: Vec<_> = nodes(&f, &d).collect();
        assert_eq!(ns.len(), 1);
        assert_eq!(ns[0].name, b"Foo");
        assert_eq!(ns[0].properties, 1);
        assert_eq!(ns[0].property_bytes, 9);
        assert_eq!(ns[0].at, 27);
    }

    #[test]
    fn wide_headers() {
        let mut d = MAGIC.to_vec();
        d.extend_from_slice(&7500u32.to_le_bytes());
        let end = 27 + 25 + 2;
        d.extend_from_slice(&(end as u64).to_le_bytes());
        d.extend_from_slice(&0u64.to_le_bytes());
        d.extend_from_slice(&0u64.to_le_bytes());
        d.push(2);
        d.extend_from_slice(b"OK");
        d.extend_from_slice(&[0; 25]);
        let f = parse(&d).unwrap();
        assert!(f.wide);
        assert_eq!(nodes(&f, &d).next().unwrap().name, b"OK");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        let mut d = fixture();
        d[0] = b'X';
        assert!(parse(&d).is_none());
        // zero end_offset → empty stream
        let mut d2 = fixture();
        d2[27..31].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(nodes(&parse(&d2).unwrap(), &d2).count(), 0);
    }
}
