//! GLB — the binary glTF container. A 12-byte header carries
//! `glTF`, a u32LE version (2) and the total byte length, then a chain
//! of `{u32 length, u32 type}` chunks: `JSON` (0x4E4F534A) holds the
//! scene description and `BIN\0` (0x004E4942) the geometry buffer.
//! Each chunk's data is 4-byte aligned.
//!
//! ```
//! use izanagi_kit::glb::{parse, chunks, json, bin};
//! let mut d = b"glTF".to_vec();
//! d.extend_from_slice(&2u32.to_le_bytes());
//! d.extend_from_slice(&0u32.to_le_bytes()); // length, patched below
//! d.extend_from_slice(&4u32.to_le_bytes());
//! d.extend_from_slice(b"JSON");
//! d.extend_from_slice(b"{}xx");
//! let total = d.len() as u32;
//! d[8..12].copy_from_slice(&total.to_le_bytes());
//! let g = parse(&d).unwrap();
//! assert_eq!(json(&g, &d), Some(b"{}xx".as_slice()));
//! assert!(bin(&g, &d).is_none());
//! ```

/// Chunk type of the JSON scene description.
pub const JSON: u32 = 0x4E4F534A;
/// Chunk type of the binary buffer.
pub const BIN: u32 = 0x004E4942;

/// A parsed GLB header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Glb {
    /// Container version (2 for glTF 2.0).
    pub version: u32,
    /// Declared total byte length.
    pub length: u32,
}

/// One chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chunk {
    /// Chunk type word (`JSON`, `BIN`, or an extension).
    pub kind: u32,
    /// Byte offset of the chunk data.
    pub at: usize,
    /// Declared data length.
    pub len: u32,
}

fn u32l(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

/// Parse the header; `None` without `glTF`, and `None` when the
/// declared total length exceeds the data.
pub fn parse(d: &[u8]) -> Option<Glb> {
    if d.get(..4)? != b"glTF" {
        return None;
    }
    let version = u32l(d, 4)?;
    let length = u32l(d, 8)?;
    if usize::try_from(length).ok()? > d.len() {
        return None;
    }
    Some(Glb { version, length })
}

/// Iterate chunks until the data runs out.
pub fn chunks<'d>(_g: &Glb, d: &'d [u8]) -> impl Iterator<Item = Chunk> + 'd {
    let mut at = 12usize;
    core::iter::from_fn(move || {
        let len = u32l(d, at)?;
        let kind = u32l(d, at + 4)?;
        let c = Chunk {
            kind,
            at: at + 8,
            len,
        };
        at = at.checked_add(8)?.checked_add(len as usize)?;
        Some(c)
    })
}

fn first<'d>(g: &Glb, d: &'d [u8], kind: u32) -> Option<&'d [u8]> {
    let c = chunks(g, d).find(|c| c.kind == kind)?;
    d.get(c.at..c.at.checked_add(c.len as usize)?)
}

/// The first `JSON` chunk's payload.
pub fn json<'d>(g: &Glb, d: &'d [u8]) -> Option<&'d [u8]> {
    first(g, d, JSON)
}

/// The first `BIN` chunk's payload.
pub fn bin<'d>(g: &Glb, d: &'d [u8]) -> Option<&'d [u8]> {
    first(g, d, BIN)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = b"glTF".to_vec();
        d.extend_from_slice(&2u32.to_le_bytes());
        d.extend_from_slice(&0u32.to_le_bytes());
        d.extend_from_slice(&4u32.to_le_bytes());
        d.extend_from_slice(&JSON.to_le_bytes());
        d.extend_from_slice(b"{}xx");
        d.extend_from_slice(&8u32.to_le_bytes());
        d.extend_from_slice(&BIN.to_le_bytes());
        d.extend_from_slice(&[0; 8]);
        let total = d.len() as u32;
        d[8..12].copy_from_slice(&total.to_le_bytes());
        d
    }

    #[test]
    fn parses_chunks() {
        let d = fixture();
        let g = parse(&d).unwrap();
        assert_eq!(g.version, 2);
        let cs: Vec<_> = chunks(&g, &d).collect();
        assert_eq!(cs.len(), 2);
        assert_eq!(cs[0].kind, JSON);
        assert_eq!(cs[1].kind, BIN);
        assert_eq!(cs[1].at, 12 + 8 + 4 + 8);
        assert_eq!(json(&g, &d), Some(b"{}xx".as_slice()));
        assert_eq!(bin(&g, &d).unwrap().len(), 8);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        let mut d = fixture();
        d[0] = b'X';
        assert!(parse(&d).is_none());
        // declared length beyond the data
        let mut d2 = fixture();
        d2[8..12].copy_from_slice(&(u32::MAX).to_le_bytes());
        assert!(parse(&d2).is_none());
        // no BIN chunk → None
        let mut d3 = fixture();
        d3.truncate(24);
        d3[8..12].copy_from_slice(&24u32.to_le_bytes());
        assert!(bin(&parse(&d3).unwrap(), &d3).is_none());
    }
}
