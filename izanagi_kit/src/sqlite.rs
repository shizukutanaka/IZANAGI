//! SQLite3 database-file header: the fixed 100-byte prefix
//! (`"SQLite format 3\0"`, page size, schema cookie, text encoding,
//! …) plus the page-1 b-tree header so callers can walk
//! `sqlite_master` cells. WAL/journal replay and record decoding
//! are intentionally out of scope — `sqlite` is a *structural* read.
//!
//! All fields are big-endian on disk.
//!
//! ```
//! use izanagi_kit::sqlite;
//! let mut h = b"SQLite format 3\0".to_vec();
//! h.extend_from_slice(&[0x10, 0x00]); // page size 4096
//! h.extend_from_slice(&[2, 2]);       // write/read version: WAL
//! h.extend_from_slice(&[0, 64, 32, 32]); // reserved + payload fractions
//! h.extend_from_slice(&[0; 32]);      // counters/cookies/caches zeroed…
//! h.extend_from_slice(&[0, 0, 0, 1]); // …except text encoding = UTF-8
//! h.extend_from_slice(&[0; 12]);      // user ver + incremental + app id
//! h.extend_from_slice(&[0; 20]);      // reserved, must be zero
//! h.extend_from_slice(&[0; 8]);       // version-valid-for + lib version
//! let s = sqlite::parse(&h).unwrap();
//! assert_eq!(s.page_size, 4096);
//! assert_eq!(s.encoding, "utf-8");
//! assert_eq!(s.write_version, 2); // WAL
//! ```

fn r16(d: &[u8], at: usize) -> Option<u32> {
    Some((*d.get(at)? as u32) << 8 | *d.get(at + 1)? as u32)
}
fn r32(d: &[u8], at: usize) -> Option<u32> {
    Some(r16(d, at)? << 16 | r16(d, at + 2)?)
}

/// The parsed 100-byte SQLite header.
#[derive(Clone, Debug)]
pub struct Sqlite {
    /// Bytes 16–17: page size (1 means 65536).
    pub page_size: u32,
    /// Byte 18: write version (1=legacy rollback, 2=WAL).
    pub write_version: u8,
    /// Byte 19: read version.
    pub read_version: u8,
    /// Byte 20: reserved bytes at the end of each page.
    pub reserved: u8,
    /// Bytes 28–31: database size in pages (as written; may be stale
    /// when the change counter is not version-valid).
    pub page_count: u32,
    /// Bytes 40–43: schema cookie.
    pub schema_cookie: u32,
    /// Bytes 44–47: schema format number (1–4).
    pub schema_format: u32,
    /// Bytes 56–59: text encoding (1 UTF-8, 2 UTF-16le, 3 UTF-16be).
    pub encoding: &'static str,
    /// Bytes 60–63: user version.
    pub user_version: u32,
    /// Bytes 68–71: application ID.
    pub app_id: u32,
    /// Bytes 96–99: SQLITE_VERSION_NUMBER that last wrote the file.
    pub lib_version: u32,
}

/// The page-1 b-tree page header (starts at byte 100).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageTree {
    /// B-tree page kind.
    pub ty: &'static str,
    /// Number of cells on the page.
    pub cells: u16,
    /// Start of cell content area (0 means 65536).
    pub content_start: u32,
    /// Whether the page is interior (has a rightmost pointer).
    pub interior: bool,
}

/// Parse the fixed header. Rejects non-SQLite-magic, impossible
/// payload fractions, and a page size that isn't a power of two
/// in `512..=65536`.
pub fn parse(d: &[u8]) -> Option<Sqlite> {
    if d.get(0..16)? != b"SQLite format 3\0" {
        return None;
    }
    let mut page_size = r16(d, 16)?;
    if page_size == 1 {
        page_size = 65536;
    }
    if !(512..=65536).contains(&page_size) || !page_size.is_power_of_two() {
        return None;
    }
    // payload fractions are fixed by the file format
    if d[21] != 64 || d[22] != 32 || d[23] != 32 {
        return None;
    }
    let encoding = match r32(d, 56)? {
        1 => "utf-8",
        2 => "utf-16le",
        3 => "utf-16be",
        _ => return None,
    };
    Some(Sqlite {
        page_size,
        write_version: d[18],
        read_version: d[19],
        reserved: d[20],
        page_count: r32(d, 28)?,
        schema_cookie: r32(d, 40)?,
        schema_format: r32(d, 44)?,
        encoding,
        user_version: r32(d, 60)?,
        app_id: r32(d, 68)?,
        lib_version: r32(d, 96)?,
    })
}

/// The b-tree header at byte 100 of the file (page 1 = sqlite_master).
/// Types: 0x02 interior index, 0x05 interior table, 0x0A leaf index,
/// 0x0D leaf table.
pub fn page1_tree(d: &[u8]) -> Option<PageTree> {
    let ty = *d.get(100)?;
    let (name, interior) = match ty {
        0x02 => ("interior-index", true),
        0x05 => ("interior-table", true),
        0x0A => ("leaf-index", false),
        0x0D => ("leaf-table", false),
        _ => return None,
    };
    let hdr = if interior { 12 } else { 8 };
    let cells = r16(d, 103)? as u16;
    let mut content_start = r16(d, 105)?;
    if content_start == 0 {
        content_start = 65536;
    }
    let _ = hdr;
    Some(PageTree {
        ty: name,
        cells,
        content_start,
        interior,
    })
}

/// Cell pointer array entries on page 1 (right after the page header:
/// 12 bytes for interior pages, 8 for leaf). Offsets are file-relative
/// (page 1 starts at byte 0).
pub fn page1_cells(d: &[u8]) -> Option<Vec<u16>> {
    let p = page1_tree(d)?;
    let base = if p.interior { 112 } else { 108 };
    let mut out = Vec::with_capacity(p.cells as usize);
    for i in 0..p.cells as usize {
        out.push(r16(d, base + i * 2)? as u16);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal valid header + a page-1 leaf-table b-tree header.
    fn db() -> Vec<u8> {
        let mut f = b"SQLite format 3\0".to_vec();
        f.extend_from_slice(&[0x10, 0x00]); // page 4096
        f.extend_from_slice(&[2, 2]); // WAL both
        f.extend_from_slice(&[0, 64, 32, 32]);
        f.resize(28, 0);
        f.extend_from_slice(&[0, 0, 0, 5]); // 5 pages
        f.resize(40, 0);
        f.extend_from_slice(&[0, 0, 0, 7]); // schema cookie 7
        f.extend_from_slice(&[0, 0, 0, 4]); // schema format 4
        f.resize(56, 0);
        f.extend_from_slice(&[0, 0, 0, 1]); // utf-8
        f.resize(96, 0);
        f.extend_from_slice(&[0x00, 0x2D, 0xC7, 0x2E]); // 3,000,110
                                                        // page-1 b-tree @100: leaf-table, 2 cells, content at 1024
        f.extend_from_slice(&[0x0D, 0, 0, 0, 2, 0x04, 0x00, 0]);
        // cell ptr array
        f.extend_from_slice(&[0x04, 0x10, 0x04, 0x20]);
        f
    }

    #[test]
    fn header_fields() {
        let d = db();
        let s = parse(&d).unwrap();
        assert_eq!(s.page_size, 4096);
        assert_eq!(s.write_version, 2);
        assert_eq!(s.page_count, 5);
        assert_eq!(s.schema_cookie, 7);
        assert_eq!(s.schema_format, 4);
        assert_eq!(s.encoding, "utf-8");
        assert_eq!(s.lib_version, 3_000_110);
    }

    #[test]
    fn page1_btree() {
        let d = db();
        let p = page1_tree(&d).unwrap();
        assert_eq!(p.ty, "leaf-table");
        assert_eq!(p.cells, 2);
        assert_eq!(p.content_start, 1024);
        assert!(!p.interior);
        assert_eq!(page1_cells(&d).unwrap(), vec![0x410, 0x420]);
    }

    #[test]
    fn malformed_rejected() {
        assert!(parse(&[]).is_none());
        assert!(parse(b"SQLite format 3x").is_none());
        let mut d = db();
        d[17] = 0x11; // page size 0x1100 — not pow2
        assert!(parse(&d).is_none());
        let mut e = db();
        e[21] = 63; // payload fraction must be 64
        assert!(parse(&e).is_none());
        let mut g = db();
        g[56] = 0;
        g[57] = 0;
        g[58] = 0;
        g[59] = 9; // bad encoding
        assert!(parse(&g).is_none());
    }
}
