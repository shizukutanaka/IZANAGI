//! glTF 2.0 binary container (`GLB`): a 12-byte header
//! (`glTF` magic, version, total length) followed by length-prefixed
//! chunks — `JSON` (scene description, delegated to
//! [`crate::json`]) and `BIN\0` (buffer data).
//!
//! ```
//! // GLB: version 2, one JSON chunk "{}", one 4-byte BIN chunk.
//! let mut g = Vec::new();
//! g.extend_from_slice(b"glTF");
//! g.extend_from_slice(&2u32.to_le_bytes());
//! g.extend_from_slice(&60u32.to_le_bytes()); // total length
//! g.extend_from_slice(&28u32.to_le_bytes());
//! g.extend_from_slice(b"JSON");
//! g.extend_from_slice(b"{\"asset\":{\"version\":\"2\x2e0\"}} ");
//! g.extend_from_slice(&4u32.to_le_bytes());
//! g.extend_from_slice(b"BIN\0");
//! g.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]);
//! let f = izanagi_kit::gltf::parse(&g).unwrap();
//! assert_eq!(izanagi_kit::gltf::json_len(&f), 28);
//! assert_eq!(izanagi_kit::gltf::bin(&g, &f).unwrap(), &[0xDE, 0xAD, 0xBE, 0xEF]);
//! ```

/// A GLB chunk reference.
#[derive(Debug)]
pub struct Chunk {
    /// `b"JSON"` or `b"BIN\0"` (others tolerated).
    pub ty: [u8; 4],
    /// Payload offset.
    pub at: usize,
    /// Payload size.
    pub len: usize,
}

/// A parsed GLB file.
#[derive(Debug)]
pub struct Glb {
    /// glTF container version (must be 2 for GLB).
    pub version: u32,
    /// Chunks in file order.
    pub chunks: Vec<Chunk>,
}

fn le32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        *d.get(at)? as u32
            | ((*d.get(at + 1)? as u32) << 8)
            | ((*d.get(at + 2)? as u32) << 16)
            | ((*d.get(at + 3)? as u32) << 24),
    )
}

/// Parses the GLB header and chunk table. `None` on bad magic,
/// non-v2 version, length mismatch, or chunk overrun.
pub fn parse(d: &[u8]) -> Option<Glb> {
    if d.len() < 12 || &d[0..4] != b"glTF" {
        return None;
    }
    let version = le32(d, 4)?;
    if version != 2 {
        return None;
    }
    let total = le32(d, 8)? as usize;
    if total > d.len() {
        return None;
    }
    let mut chunks = Vec::new();
    let mut at = 12;
    while at + 8 <= total {
        let len = le32(d, at)? as usize;
        let mut ty = [0u8; 4];
        ty.copy_from_slice(&d[at + 4..at + 8]);
        let data_at = at + 8;
        if data_at.checked_add(len)? > total {
            return None;
        }
        chunks.push(Chunk {
            ty,
            at: data_at,
            len,
        });
        at = data_at + len;
    }
    if chunks.is_empty() || &chunks[0].ty != b"JSON" {
        return None;
    }
    Some(Glb { version, chunks })
}

/// Payload of chunk `i`.
pub fn chunk_data<'a>(d: &'a [u8], f: &Glb, i: usize) -> Option<&'a [u8]> {
    let c = f.chunks.get(i)?;
    d.get(c.at..c.at + c.len)
}

/// JSON chunk payload bytes.
pub fn json_bytes<'a>(d: &'a [u8], f: &Glb) -> Option<&'a [u8]> {
    let c = f.chunks.first()?;
    d.get(c.at..c.at + c.len)
}

/// JSON chunk length.
pub fn json_len(f: &Glb) -> usize {
    f.chunks.first().map(|c| c.len).unwrap_or(0)
}

/// BIN chunk payload (first `BIN\0` chunk).
pub fn bin<'a>(d: &'a [u8], f: &Glb) -> Option<&'a [u8]> {
    f.chunks
        .iter()
        .find(|c| &c.ty == b"BIN\0")
        .and_then(|c| d.get(c.at..c.at + c.len))
}

/// Parses the JSON chunk via [`crate::json`].
pub fn json(d: &[u8], f: &Glb) -> Option<crate::json::Json> {
    crate::json::parse(json_bytes(d, f)?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let json = b"{\"asset\":{\"version\":\"2\x2e0\"},\"buffers\":[{\"byteLength\":4}]}";
        let mut g = Vec::new();
        g.extend_from_slice(b"glTF");
        g.extend_from_slice(&2u32.to_le_bytes());
        let pad = (4 - json.len() % 4) % 4;
        let jl = json.len() + pad;
        g.extend_from_slice(&((12 + 8 + jl + 8 + 4) as u32).to_le_bytes());
        g.extend_from_slice(&(jl as u32).to_le_bytes());
        g.extend_from_slice(b"JSON");
        g.extend_from_slice(json);
        g.extend_from_slice(&vec![b' '; pad]);
        g.extend_from_slice(&4u32.to_le_bytes());
        g.extend_from_slice(b"BIN\0");
        g.extend_from_slice(&[1, 2, 3, 4]);
        g
    }

    #[test]
    fn parses_chunks_and_json() {
        let d = fixture();
        let f = parse(&d).unwrap();
        assert_eq!(f.version, 2);
        assert_eq!(f.chunks.len(), 2);
        assert!(json(&d, &f).is_some());
        assert!(json_bytes(&d, &f).unwrap().starts_with(b"{"));
        assert_eq!(bin(&d, &f).unwrap(), &[1, 2, 3, 4]);
        assert_eq!(json_len(&f) % 4, 0);
        assert!(chunk_data(&d, &f, 0).is_some());
        assert!(chunk_data(&d, &f, 2).is_none());
    }

    #[test]
    fn malformed_degrades() {
        assert!(parse(&[]).is_none());
        let mut bad = fixture();
        bad[0] = b'X'; // magic
        assert!(parse(&bad).is_none());
        let mut bad2 = fixture();
        bad2[4] = 1; // version 1 not supported
        assert!(parse(&bad2).is_none());
        let mut bad3 = fixture();
        bad3[12] = 0xFF; // JSON chunk len huge
        assert!(parse(&bad3).is_none());
        let mut bad4 = fixture();
        bad4[16] = b'B'; // first chunk must be JSON
        bad4[17] = b'I';
        bad4[18] = b'N';
        bad4[19] = 0;
        assert!(parse(&bad4).is_none());
    }
}
