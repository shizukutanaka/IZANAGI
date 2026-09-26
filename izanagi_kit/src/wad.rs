//! Doom-engine WAD archive (`IWAD`/`PWAD`). The file is a 12-byte
//! header, lump payloads, then a directory of `numlumps` 16-byte
//! entries `{filepos, size, name[8]}`. Marker lumps (like `F_START`)
//! have `size == 0` and delimit namespace blocks.
//!
//! ```
//! let mut w = Vec::new();
//! w.extend_from_slice(b"PWAD");
//! w.extend_from_slice(&2u32.to_le_bytes()); // 2 lumps
//! w.extend_from_slice(&21u32.to_le_bytes()); // directory at 21
//! w.extend_from_slice(b"leveldata"); // 9 bytes of lump 0
//! w.extend_from_slice(&12u32.to_le_bytes()); // lump0: at=12 size=9
//! w.extend_from_slice(&9u32.to_le_bytes());
//! w.extend_from_slice(b"MAP01\0\0\0");
//! w.extend_from_slice(&0u32.to_le_bytes()); // lump1: marker F_START
//! w.extend_from_slice(&0u32.to_le_bytes());
//! w.extend_from_slice(b"F_START\0");
//! let wad = izanagi_kit::wad::parse(&w).unwrap();
//! assert!(izanagi_kit::wad::is_iwad(&wad) == false);
//! let l = izanagi_kit::wad::find(&wad, b"MAP01\0\0\0").unwrap();
//! assert_eq!(izanagi_kit::wad::lump(&w, l), Some(b"leveldata".as_slice()));
//! ```

/// A directory entry.
#[derive(Debug)]
pub struct Lump {
    /// Byte offset of the lump payload.
    pub at: usize,
    /// Payload size in bytes (0 for markers).
    pub size: usize,
    /// Uppercase NUL-padded name (≤8 bytes).
    pub name: [u8; 8],
}

/// A parsed WAD archive.
#[derive(Debug)]
pub struct Wad {
    /// `true` for `IWAD`, `false` for `PWAD`.
    pub iwad: bool,
    /// Directory entries in file order.
    pub lumps: Vec<Lump>,
}

fn le32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        *d.get(at)? as u32
            | ((*d.get(at + 1)? as u32) << 8)
            | ((*d.get(at + 2)? as u32) << 16)
            | ((*d.get(at + 3)? as u32) << 24),
    )
}

/// Parses the header and directory. `None` on a bad magic, a
/// directory outside the file, or a lump range that overruns it.
pub fn parse(d: &[u8]) -> Option<Wad> {
    if d.len() < 12 {
        return None;
    }
    let iwad = match &d[0..4] {
        b"IWAD" => true,
        b"PWAD" => false,
        _ => return None,
    };
    let count = le32(d, 4)? as usize;
    let dir_at = le32(d, 8)? as usize;
    let dir_end = dir_at.checked_add(count.checked_mul(16)?)?;
    if dir_end > d.len() {
        return None;
    }
    let mut lumps = Vec::with_capacity(count.min(1 << 20));
    for i in 0..count {
        let e = dir_at + i * 16;
        let at = le32(d, e)? as usize;
        let size = le32(d, e + 4)? as usize;
        if at.checked_add(size)? > d.len() {
            return None;
        }
        let mut name = [0u8; 8];
        name.copy_from_slice(&d[e + 8..e + 16]);
        lumps.push(Lump { at, size, name });
    }
    Some(Wad { iwad, lumps })
}

/// `true` for `IWAD`, `false` for `PWAD`.
pub fn is_iwad(w: &Wad) -> bool {
    w.iwad
}

/// The payload bytes of a lump. `None` if its range overruns the file.
pub fn lump<'a>(d: &'a [u8], l: &Lump) -> Option<&'a [u8]> {
    d.get(l.at..l.at.checked_add(l.size)?)
}

/// First lump whose name matches (WAD names are uppercase; the
/// comparison is exact — callers pass `b"MAP01\0\0\0"` or use
/// [`find_name`]).
pub fn find<'a>(w: &'a Wad, name8: &[u8; 8]) -> Option<&'a Lump> {
    w.lumps.iter().find(|l| &l.name == name8)
}

/// Find by a short name — NUL pads it to 8 and uppercases ASCII.
pub fn find_name<'a>(w: &'a Wad, name: &str) -> Option<&'a Lump> {
    let mut n = [0u8; 8];
    let b = name.as_bytes();
    if b.len() > 8 {
        return None;
    }
    for (i, &c) in b.iter().enumerate() {
        n[i] = c.to_ascii_uppercase();
    }
    find(w, &n)
}

/// Every lump named `name8` (marker lumps repeat across namespaces).
pub fn find_all<'a>(w: &'a Wad, name8: &[u8; 8]) -> Vec<&'a Lump> {
    w.lumps.iter().filter(|l| &l.name == name8).collect()
}

/// Lump indices strictly between the `open`/`close` marker pair —
/// e.g. `block(&w, b"F_START", b"F_END\0")` for the flat namespace.
/// `None` when either marker is missing or reversed.
pub fn block(w: &Wad, open: &[u8; 8], close: &[u8; 8]) -> Option<Vec<usize>> {
    let lo = w.lumps.iter().position(|l| &l.name == open)?;
    let hi = w.lumps.iter().position(|l| &l.name == close)?;
    if hi <= lo {
        return None;
    }
    Some((lo + 1..hi).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// IWAD with MAP01 data lump + F_START marker + FLAX1 payload.
    fn fixture() -> Vec<u8> {
        // layout: 0..12 header, 12..21 "leveldata", 21..25 "flax",
        // dir at 25 = 3 * 16B
        let mut w = Vec::new();
        w.extend_from_slice(b"IWAD");
        w.extend_from_slice(&3u32.to_le_bytes());
        w.extend_from_slice(&25u32.to_le_bytes());
        w.extend_from_slice(b"leveldata");
        w.extend_from_slice(b"flax");
        // dir entries
        w.extend_from_slice(&12u32.to_le_bytes()); // at
        w.extend_from_slice(&9u32.to_le_bytes()); // size
        w.extend_from_slice(b"MAP01\0\0\0");
        w.extend_from_slice(&0u32.to_le_bytes()); // marker
        w.extend_from_slice(&0u32.to_le_bytes());
        w.extend_from_slice(b"F_START\0");
        w.extend_from_slice(&21u32.to_le_bytes());
        w.extend_from_slice(&4u32.to_le_bytes());
        w.extend_from_slice(b"FLAX1\0\0\0");
        w
    }

    #[test]
    fn parses_and_reads_lumps() {
        let d = fixture();
        let w = parse(&d).unwrap();
        assert!(is_iwad(&w));
        assert_eq!(w.lumps.len(), 3);
        let l = find_name(&w, "map01").unwrap();
        assert_eq!(lump(&d, l), Some(b"leveldata".as_slice()));
        assert_eq!(
            lump(&d, find_name(&w, "flax1").unwrap()),
            Some(b"flax".as_slice())
        );
        assert!(find_name(&w, "nope").is_none());
        assert!(find_name(&w, "waytoolongname").is_none());
        assert_eq!(find_all(&w, b"F_START\0").len(), 1);
    }

    #[test]
    fn blocks_between_markers() {
        let d = fixture();
        let w = parse(&d).unwrap();
        // F_END absent → None
        assert!(block(&w, b"F_START\0", b"F_END\0\0\0").is_none());
        assert!(block(&w, b"MISSING\0", b"F_START\0").is_none());
        // reversed markers → None
        assert!(block(&w, b"FLAX1\0\0\0", b"MAP01\0\0\0").is_none());
    }

    #[test]
    fn malformed_degrades() {
        assert!(parse(&[]).is_none());
        assert!(parse(b"XXXX").is_none());
        let mut bad = fixture();
        bad[8] = 200; // dir far out of range
        assert!(parse(&bad).is_none());
        let mut bad2 = fixture();
        bad2[4] = 99; // dir count overflows file
        assert!(parse(&bad2).is_none());
        let mut bad3 = fixture();
        bad3[29] = 250; // lump0 payload overruns
        assert!(parse(&bad3).is_none());
    }
}
