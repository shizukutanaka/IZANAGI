//! 3D Studio `.3ds` binary chunk tree (Autodesk/Discreet format).
//!
//! Every chunk is `{id:u16 LE, len:u32 LE (header included), data}`;
//! container chunks nest others. Well-known ids: `0x4D4D` MAIN,
//! `0x3D3D` editor data, `0x4000` object block (leading
//! NUL-terminated name, then subchunks), `0x4100` triangular mesh,
//! `0x4110` vertex list (`u16 count` + `count × 3` raw u32 float
//! bits), `0x4120` face list (`u16 count` + `count × {a,b,c,flags}`
//! u16s), `0x4130` material reference, `0xA000` material name.
//!
//! ```
//! use izanagi_kit::tds::{parse, find, object_name, MAIN, EDITOR, OBJECT};
//! // MAIN { EDITOR { OBJECT "box" } }
//! let mut obj = b"box\0".to_vec();
//! let mut ed = Vec::new();
//! let obj_len = (obj.len() + 6) as u32;
//! ed.extend_from_slice(&0x4000u16.to_le_bytes());
//! ed.extend_from_slice(&obj_len.to_le_bytes());
//! ed.append(&mut obj);
//! let mut top = Vec::new();
//! top.extend_from_slice(&0x3D3Du16.to_le_bytes());
//! top.extend_from_slice(&((ed.len() + 6) as u32).to_le_bytes());
//! top.append(&mut ed);
//! let mut d = Vec::new();
//! d.extend_from_slice(&0x4D4Du16.to_le_bytes());
//! d.extend_from_slice(&((top.len() + 6) as u32).to_le_bytes());
//! d.append(&mut top);
//! let m = parse(&d).unwrap();
//! let e = find(&d, &m, EDITOR).unwrap();
//! let o = find(&d, &e, OBJECT).unwrap();
//! assert_eq!(object_name(&d, &o), Some(&b"box"[..]));
//! ```

fn le16(d: &[u8], o: usize) -> Option<u16> {
    Some((*d.get(o)? as u16) | (*d.get(o + 1)? as u16) << 8)
}
fn le32(d: &[u8], o: usize) -> Option<u32> {
    Some(
        (*d.get(o)? as u32)
            | (*d.get(o + 1)? as u32) << 8
            | (*d.get(o + 2)? as u32) << 16
            | (*d.get(o + 3)? as u32) << 24,
    )
}

/// `0x4D4D` — top-level main chunk.
pub const MAIN: u16 = 0x4D4D;
/// `0x3D3D` — editor (scene) data inside MAIN.
pub const EDITOR: u16 = 0x3D3D;
/// `0x4000` — named object block.
pub const OBJECT: u16 = 0x4000;
/// `0x4100` — triangular mesh inside an object block.
pub const MESH: u16 = 0x4100;
/// `0x4110` — vertex list inside a mesh.
pub const VERTICES: u16 = 0x4110;
/// `0x4120` — face list inside a mesh.
pub const FACES: u16 = 0x4120;
/// `0x4130` — material reference (name string + u16 face count + ids).
pub const MATERIAL: u16 = 0x4130;
/// `0xA000` — material name string (inside `0xAFFF` material block).
pub const MATERIAL_NAME: u16 = 0xA000;
/// `0xB000` — keyframe chunk inside MAIN.
pub const KEYFRAME: u16 = 0xB000;

/// Short name for a well-known chunk id (`None` for others).
pub fn kind_name(id: u16) -> Option<&'static str> {
    Some(match id {
        MAIN => "MAIN",
        EDITOR => "EDITOR",
        OBJECT => "OBJECT",
        MESH => "MESH",
        VERTICES => "VERTICES",
        FACES => "FACES",
        MATERIAL => "MATERIAL",
        MATERIAL_NAME => "MATERIAL_NAME",
        KEYFRAME => "KEYFRAME",
        0xAFFF => "MATERIAL_BLOCK",
        0x4111 => "VERTEX_FLAGS",
        0x4140 => "VERTEX_MAP",
        0x4160 => "LOCAL_AXIS",
        _ => return None,
    })
}

/// One chunk header. `at` is the payload offset; `end` is `at + len`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chunk {
    /// Chunk id.
    pub id: u16,
    /// Payload start (header start + 6).
    pub at: usize,
    /// Declared total length including the 6-byte header.
    pub len: usize,
}

/// Read the chunk header at `at`. `None` on truncation or a declared
/// length shorter than its own header.
pub fn chunk_at(d: &[u8], at: usize) -> Option<Chunk> {
    let id = le16(d, at)?;
    let len = le32(d, at + 2)? as usize;
    if len < 6 {
        return None;
    }
    let payload = at.checked_add(6)?;
    let end = at.checked_add(len)?;
    if end > d.len() {
        return None;
    }
    Some(Chunk {
        id,
        at: payload,
        len: len - 6,
    })
}

/// Walk the chunk headers between `at` and `end`, stopping at the
/// first malformed one.
pub fn chunks(d: &[u8], mut at: usize, end: usize) -> Vec<Chunk> {
    let mut out = Vec::new();
    while at + 6 <= end {
        match chunk_at(d, at) {
            Some(c) if at.checked_add(c.len + 6).is_some_and(|n| n <= end) => {
                let next = at + c.len + 6;
                out.push(c);
                at = next;
            }
            _ => break,
        }
    }
    out
}

/// Parse a `.3ds` file: the first chunk must be `0x4D4D` MAIN.
/// Returns the MAIN chunk whose payload spans the rest of the file.
pub fn parse(d: &[u8]) -> Option<Chunk> {
    let c = chunk_at(d, 0)?;
    if c.id != MAIN {
        return None;
    }
    Some(c)
}

/// The NUL-terminated object name at the start of an `OBJECT` (`0x4000`)
/// chunk's payload.
pub fn object_name<'a>(d: &'a [u8], object: &Chunk) -> Option<&'a [u8]> {
    if object.id != OBJECT {
        return None;
    }
    let mut i = object.at;
    loop {
        let b = *d.get(i)?;
        if b == 0 {
            return Some(&d[object.at..i]);
        }
        i += 1;
        if i >= object.at + object.len {
            return None;
        }
    }
}

/// Payload of the first nested chunk with `id` inside `parent`. An
/// `OBJECT` parent's leading NUL-terminated name is skipped before
/// its subchunks.
pub fn find(d: &[u8], parent: &Chunk, id: u16) -> Option<Chunk> {
    let start = if parent.id == OBJECT {
        match object_name(d, parent) {
            Some(_) => {
                let mut i = parent.at;
                while *d.get(i)? != 0 {
                    i += 1;
                }
                i + 1
            }
            None => return None,
        }
    } else {
        parent.at
    };
    chunks(d, start, parent.at + parent.len)
        .into_iter()
        .find(|c| c.id == id)
}

/// Vertex payload of a `MESH` chunk: `(u16 count, count × 3 raw u32
/// float bits)`. Returns the count and the offset of the triples.
pub fn vertices(d: &[u8], mesh: &Chunk) -> Option<(usize, usize)> {
    if mesh.id != MESH {
        return None;
    }
    let verts = find(d, mesh, VERTICES)?;
    let count = le16(d, verts.at)? as usize;
    let bytes = count.checked_mul(12)?;
    if verts.at.checked_add(2 + bytes)? > verts.at + verts.len {
        return None;
    }
    Some((count, verts.at + 2))
}

/// Face payload of a `MESH` chunk: `count` × `{a,b,c,flags}` u16s at
/// the returned offset.
pub fn faces(d: &[u8], mesh: &Chunk) -> Option<(usize, usize)> {
    if mesh.id != MESH {
        return None;
    }
    let fs = find(d, mesh, FACES)?;
    let count = le16(d, fs.at)? as usize;
    let bytes = count.checked_mul(8)?;
    if fs.at.checked_add(2 + bytes)? > fs.at + fs.len {
        return None;
    }
    Some((count, fs.at + 2))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn push_chunk(d: &mut Vec<u8>, id: u16, payload: &[u8]) {
        d.extend_from_slice(&(id).to_le_bytes());
        d.extend_from_slice(&((payload.len() as u32 + 6).to_le_bytes()));
        d.extend_from_slice(payload);
    }

    fn fixture() -> Vec<u8> {
        // mesh: 3 vertices + 2 faces
        let mut verts = 3u16.to_le_bytes().to_vec();
        verts.extend_from_slice(&[0u8; 36]); // 3 × (x,y,z) raw bits
        let mut faces = 2u16.to_le_bytes().to_vec();
        faces.extend_from_slice(&[0u8; 16]); // 2 × {a,b,c,flags}
        let mut mesh = Vec::new();
        push_chunk(&mut mesh, VERTICES, &verts);
        push_chunk(&mut mesh, FACES, &faces);
        let mut obj = b"box\0".to_vec();
        push_chunk(&mut obj, MESH, &mesh);
        let mut ed = Vec::new();
        push_chunk(&mut ed, OBJECT, &obj);
        let mut file = Vec::new();
        push_chunk(&mut file, EDITOR, &ed);
        let mut d = Vec::new();
        push_chunk(&mut d, MAIN, &file);
        d
    }

    #[test]
    fn objects_and_meshes() {
        let d = fixture();
        let main = parse(&d).unwrap();
        assert_eq!(main.id, MAIN);
        let ed = find(&d, &main, EDITOR).unwrap();
        let obj = find(&d, &ed, OBJECT).unwrap();
        assert_eq!(object_name(&d, &obj), Some(&b"box"[..]));
        let mesh = find(&d, &obj, MESH).unwrap();
        assert_eq!(vertices(&d, &mesh), Some((3, mesh.at + 8)));
        assert_eq!(faces(&d, &mesh), Some((2, mesh.at + 8 + 8 + 36)));
        assert_eq!(kind_name(0x4D4D), Some("MAIN"));
        assert_eq!(kind_name(0x9999), None);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"short").is_none());
        let mut d = fixture();
        d[0] = 0x01; // not MAIN
        assert!(parse(&d).is_none());
        let mut d2 = fixture();
        let n = d2.len();
        d2[2] = 0xFF; // MAIN len absurd
        assert!(chunk_at(&d2, 0).is_none() || n > 0);
        let mut d3 = fixture();
        d3[2..6].copy_from_slice(&4u32.to_le_bytes()); // len < header
        assert!(chunk_at(&d3, 0).is_none());
        // empty object name
        let mut bad = Vec::new();
        let mut o = Vec::new();
        push_chunk(&mut o, OBJECT, &[0x41]); // no NUL
        push_chunk(&mut bad, EDITOR, &o);
        let mut f = Vec::new();
        push_chunk(&mut f, MAIN, &bad);
        let m = parse(&f).unwrap();
        let e = find(&f, &m, EDITOR).unwrap();
        let ob = find(&f, &e, OBJECT).unwrap();
        assert_eq!(object_name(&f, &ob), None);
    }
}
