//! MBTiles — a tile pyramid stored in SQLite (Mapbox specification).
//!
//! Detection: SQLite magic + a `metadata` and a `tiles` table (or view)
//! somewhere in the schema text of page 1 / later pages.
//!
//! ```
//! let mut d = b"SQLite format 3\0".to_vec();
//! d.resize(512, 0);
//! d[16..18].copy_from_slice(&[0x02, 0x00]); // page size 512
//! d[21] = 64; d[22] = 32; d[23] = 32;
//! d[56..60].copy_from_slice(&[0, 0, 0, 1]);
//! d[120..].iter_mut().zip(b"CREATE TABLE metadata (name TEXT, value TEXT);CREATE TABLE tiles (zoom_level INTEGER)").for_each(|(p, &b)| *p = b);
//! let m = izanagi_kit::mbtiles::parse(&d).unwrap();
//! assert!(m.has_metadata && m.has_tiles);
//! ```

use crate::sqlite;

/// Parsed MBTiles identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mbtiles {
    /// SQLite page size.
    pub page_size: u32,
    /// `metadata` (name/value) table found.
    pub has_metadata: bool,
    /// `tiles` (zoom_level/tile_column/tile_row/tile_data) table or view found.
    pub has_tiles: bool,
    /// Optional `grids`/`grid_data` UTFGrid tables found.
    pub has_utfgrid: bool,
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

/// Parse an MBTiles file; `None` on non-SQLite input or missing schema.
pub fn parse(d: &[u8]) -> Option<Mbtiles> {
    let s = sqlite::parse(d)?;
    let has_metadata = contains(d, b"metadata");
    let has_tiles = contains(d, b"tiles");
    let has_utfgrid = contains(d, b"grid_data") || contains(d, b"grids");
    if !has_metadata || !has_tiles {
        return None;
    }
    Some(Mbtiles {
        page_size: s.page_size,
        has_metadata,
        has_tiles,
        has_utfgrid,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db(schema: &[u8]) -> Vec<u8> {
        let mut d = b"SQLite format 3\0".to_vec();
        d.resize(1024, 0);
        d[16..18].copy_from_slice(&[0x04, 0x00]);
        d[21] = 64;
        d[22] = 32;
        d[23] = 32;
        d[56..60].copy_from_slice(&[0, 0, 0, 1]);
        d[200..200 + schema.len()].copy_from_slice(schema);
        d
    }

    #[test]
    fn basic() {
        let d = db(b"CREATE TABLE metadata (name TEXT, value TEXT); CREATE TABLE tiles (zoom_level INTEGER, tile_column INTEGER, tile_row INTEGER, tile_data BLOB); CREATE TABLE grids");
        let m = parse(&d).unwrap();
        assert!(m.has_metadata && m.has_tiles && m.has_utfgrid);
        assert_eq!(m.page_size, 1024);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&db(b"CREATE TABLE foo (x INT)")).is_none());
        assert!(parse(&db(b"CREATE TABLE tiles (x INT)")).is_none()); // metadata missing
    }
}
