//! Duke Nukem 3D / Build engine GRP archive.
//!
//! A GRP opens with the 12-byte signature `KenSilverman`, a u32
//! file count, then `count` entries of `(12-byte name, u32
//! size)`; file data follows, each blob contiguous in directory
//! order. `parse` bounds-checks the directory and every blob.
//!
//! ```
//! use izanagi_kit::grp::parse;
//!
//! let mut d = vec![0u8; 16 + 16 + 4];
//! d[..12].copy_from_slice(b"KenSilverman");
//! d[12] = 1;                  // one file
//! d[16..22].copy_from_slice(b"DUKE3D");
//! d[28] = 4;                  // size
//! d[32..36].copy_from_slice(b"DATA");
//! let g = parse(&d).unwrap();
//! assert_eq!(g.entries[0].name, "DUKE3D");
//! assert_eq!(g.file(&d, 0).unwrap(), b"DATA");
//! ```

/// Signature.
pub const MAGIC: &[u8; 12] = b"KenSilverman";
/// Fixed entry size: 12-byte name + u32 size.
pub const ENTRY: usize = 16;

fn le32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(at)?)
            | u32::from(*d.get(at + 1)?) << 8
            | u32::from(*d.get(at + 2)?) << 16
            | u32::from(*d.get(at + 3)?) << 24,
    )
}

/// One GRP directory entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// 12.3-style name (NUL-trimmed, upper case as shipped).
    pub name: String,
    /// Declared blob size.
    pub size: u32,
    /// Resolved byte offset of the blob in the file.
    pub offset: usize,
}

/// A parsed GRP directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grp {
    /// Entries in directory order.
    pub entries: Vec<Entry>,
}

impl Grp {
    /// File blob `i` as a slice into `d`.
    pub fn file<'a>(&self, d: &'a [u8], i: usize) -> Option<&'a [u8]> {
        let e = self.entries.get(i)?;
        d.get(e.offset..e.offset + e.size as usize)
    }
    /// Look up a file by name (case-insensitive).
    pub fn find<'a>(&self, d: &'a [u8], name: &str) -> Option<&'a [u8]> {
        let want = name.to_uppercase();
        self.entries
            .iter()
            .position(|e| e.name == want)
            .and_then(|i| self.file(d, i))
    }
}

/// Parse a GRP directory. Returns `None` on a bad signature, a
/// truncated directory, or a blob that overruns the file.
pub fn parse(d: &[u8]) -> Option<Grp> {
    if d.get(..12)? != MAGIC {
        return None;
    }
    let count = le32(d, 12)? as usize;
    let dir_len = count.checked_mul(ENTRY)?;
    let dir = d.get(16..16 + dir_len)?;
    let mut at = 16 + dir_len;
    let mut entries = Vec::with_capacity(count);
    for i in 0..count {
        let raw = dir.get(i * ENTRY..i * ENTRY + 12)?;
        let end = raw.iter().position(|&b| b == 0).unwrap_or(12);
        let name = core::str::from_utf8(&raw[..end])
            .unwrap_or("")
            .to_uppercase();
        let size = le32(dir, i * ENTRY + 12)? as usize;
        let next = at.checked_add(size)?;
        if next > d.len() {
            return None;
        }
        entries.push(Entry {
            name,
            size: size as u32,
            offset: at,
        });
        at = next;
    }
    Some(Grp { entries })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0u8; 16 + 3 * 16 + 10];
        d[..12].copy_from_slice(MAGIC);
        d[12] = 3;
        let names: [[u8; 12]; 3] = {
            let mut a = [[0u8; 12]; 3];
            a[0][..8].copy_from_slice(b"DEFS.CON");
            a[1][..9].copy_from_slice(b"GAME.CON\0");
            a[2][..9].copy_from_slice(b"TILES.ART");
            a
        };
        let sizes = [3u32, 4, 3];
        for (i, (n, s)) in names.iter().zip(sizes.iter()).enumerate() {
            d[16 + i * 16..16 + i * 16 + 12].copy_from_slice(n);
            for j in 0..4 {
                d[16 + i * 16 + 12 + j] = (*s >> (j * 8)) as u8;
            }
        }
        let data_at = 16 + 3 * 16;
        d[data_at..data_at + 3].copy_from_slice(b"aaa");
        d[data_at + 3..data_at + 7].copy_from_slice(b"bbbb");
        d[data_at + 7..data_at + 10].copy_from_slice(b"ccc");
        d
    }

    #[test]
    fn directory_and_files() {
        let d = fixture();
        let g = parse(&d).unwrap();
        assert_eq!(g.entries.len(), 3);
        assert_eq!(g.entries[0].name, "DEFS.CON");
        assert_eq!(g.entries[1].name, "GAME.CON");
        assert_eq!(g.entries[2].name, "TILES.ART");
        assert_eq!(g.file(&d, 0).unwrap(), b"aaa");
        assert_eq!(g.file(&d, 1).unwrap(), b"bbbb");
        assert_eq!(g.file(&d, 2).unwrap(), b"ccc");
        assert!(g.file(&d, 3).is_none());
        assert_eq!(g.find(&d, "game.con").unwrap(), b"bbbb");
        assert!(g.find(&d, "nope").is_none());
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 8]).is_none());
        let mut d = fixture();
        d[0] = b'X';
        assert!(parse(&d).is_none());
        let mut d2 = fixture();
        d2[12] = 200; // directory overruns file
        assert!(parse(&d2).is_none());
        let mut d3 = fixture();
        // blob overrun: last size huge
        d3[16 + 2 * 16 + 12] = 255;
        d3[16 + 2 * 16 + 15] = 255;
        assert!(parse(&d3).is_none());
    }
}
