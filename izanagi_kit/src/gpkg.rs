//! GeoPackage (OGC 12-128) — a SQLite database stamped with the
//! `GPKG` (or legacy `GP10`/`GP11`) application-id at header offset 68.
//!
//! ```
//! let mut d = b"SQLite format 3\0".to_vec();
//! d.resize(100, 0);
//! d[16..18].copy_from_slice(&[4, 0]); // page size 1024
//! d[21] = 64; d[22] = 32; d[23] = 32;
//! d[56..60].copy_from_slice(&[0, 0, 0, 1]); // utf-8
//! d[68..72].copy_from_slice(b"GPKG");
//! let g = izanagi_kit::gpkg::parse(&d).unwrap();
//! assert!(g.modern_id);
//! ```

use crate::sqlite;

/// Parsed GeoPackage identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gpkg {
    /// SQLite page size in bytes.
    pub page_size: u32,
    /// Application ID read at offset 68 (`0x47504B47` = `GPKG`).
    pub app_id: u32,
    /// True for the post-1.2 `GPKG` id; false for legacy `GP10`/`GP11`.
    pub modern_id: bool,
}

/// Four ASCII bytes as a big-endian u32.
fn code(s: &[u8; 4]) -> u32 {
    ((s[0] as u32) << 24) | ((s[1] as u32) << 16) | ((s[2] as u32) << 8) | s[3] as u32
}

/// Parse a GeoPackage; `None` when it isn't SQLite or lacks the app id.
pub fn parse(d: &[u8]) -> Option<Gpkg> {
    let s = sqlite::parse(d)?;
    let modern = s.app_id == code(b"GPKG");
    let legacy = s.app_id == code(b"GP10") || s.app_id == code(b"GP11");
    if !modern && !legacy {
        return None;
    }
    Some(Gpkg {
        page_size: s.page_size,
        app_id: s.app_id,
        modern_id: modern,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db(app: &[u8; 4]) -> Vec<u8> {
        let mut d = b"SQLite format 3\0".to_vec();
        d.resize(100, 0);
        d[16..18].copy_from_slice(&[0x10, 0x00]); // 4096
        d[21] = 64;
        d[22] = 32;
        d[23] = 32;
        d[56..60].copy_from_slice(&[0, 0, 0, 1]);
        d[68..72].copy_from_slice(app);
        d
    }

    #[test]
    fn modern() {
        let g = parse(&db(b"GPKG")).unwrap();
        assert!(g.modern_id);
        assert_eq!(g.page_size, 4096);
        assert_eq!(g.app_id, 0x47504B47);
    }

    #[test]
    fn legacy() {
        assert!(!parse(&db(b"GP10")).unwrap().modern_id);
        assert!(!parse(&db(b"GP11")).unwrap().modern_id);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&db(b"MBTM")).is_none()); // not a GPKG id
        assert!(parse(&db(b"    ")).is_none());
    }
}
