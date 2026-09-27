//! Minimal reader for PMTiles v3 archive headers: a fixed 127-byte
//! header beginning `PMTiles` + version byte, then little-endian u64
//! offsets/sizes for the root directory, JSON metadata, leaf directory,
//! tile data, and index ranges — followed by `tile_type`,
//! `compression`, `min/max zoom`, and center/zoom fields.
//!
//! All floats stay as their raw u32 bit patterns (`*_bits`).
//!
//! ```
//! use izanagi_kit::pmtiles::parse;
//!
//! let mut h = [0u8; 127];
//! h[..7].copy_from_slice(b"PMTiles");
//! h[7] = 3;
//! h[31] = 3; // num tiles = 3 (LE u64 at offset 24..32 would be too)
//! let _ = parse(&h);
//! ```

/// Parsed PMTiles v3 header fields.
#[derive(Debug)]
pub struct Pmtiles {
    /// Root directory byte offset.
    pub root_offset: u64,
    /// Root directory byte length.
    pub root_length: u64,
    /// JSON metadata offset.
    pub metadata_offset: u64,
    /// JSON metadata length.
    pub metadata_length: u64,
    /// Leaf directory offset.
    pub leaf_offset: u64,
    /// Leaf directory length.
    pub leaf_length: u64,
    /// Tile data offset.
    pub tile_offset: u64,
    /// Tile data length.
    pub tile_length: u64,
    /// Number of addressed tiles.
    pub num_tiles: u64,
    /// Number of tile entries.
    pub num_tile_entries: u64,
    /// Number of tile contents.
    pub num_tile_contents: u64,
    /// `true` when `clustered` byte is 1.
    pub clustered: bool,
    /// Internal compression code (1 = none / gzip etc. per spec table).
    pub internal_compression: u8,
    /// Tile compression code.
    pub tile_compression: u8,
    /// Tile type byte (0 unknown, 1 mvt, 2 png, 3 jpeg, 4 webp, 5 avif).
    pub tile_type: u8,
    /// Minimum zoom.
    pub min_zoom: u8,
    /// Maximum zoom.
    pub max_zoom: u8,
    /// `min_lon_e7` .. raw i32 (fixed-point ×10⁷ longitude).
    pub min_lon_e7: i32,
    /// `min_lat_e7`.
    pub min_lat_e7: i32,
    /// `max_lon_e7`.
    pub max_lon_e7: i32,
    /// `max_lat_e7`.
    pub max_lat_e7: i32,
    /// `center_zoom`.
    pub center_zoom: u8,
    /// `center_lon_e7`.
    pub center_lon_e7: i32,
    /// `center_lat_e7`.
    pub center_lat_e7: i32,
}

fn le64(d: &[u8], at: usize) -> Option<u64> {
    let mut v: u64 = 0;
    for i in 0..8 {
        v |= (*d.get(at + i)? as u64) << (8 * i);
    }
    Some(v)
}

fn le32s(d: &[u8], at: usize) -> Option<i32> {
    let mut v: u32 = 0;
    for i in 0..4 {
        v |= (*d.get(at + i)? as u32) << (8 * i);
    }
    Some(v as i32)
}

/// Parse a PMTiles v3 header. `None` on short input, wrong magic, or a
/// version byte other than 3.
pub fn parse(d: &[u8]) -> Option<Pmtiles> {
    if d.len() < 127 || &d[..7] != b"PMTiles" || d[7] != 3 {
        return None;
    }
    Some(Pmtiles {
        root_offset: le64(d, 8)?,
        root_length: le64(d, 16)?,
        metadata_offset: le64(d, 24)?,
        metadata_length: le64(d, 32)?,
        leaf_offset: le64(d, 40)?,
        leaf_length: le64(d, 48)?,
        tile_offset: le64(d, 56)?,
        tile_length: le64(d, 64)?,
        num_tiles: le64(d, 72)?,
        num_tile_entries: le64(d, 80)?,
        num_tile_contents: le64(d, 88)?,
        clustered: *d.get(96)? == 1,
        internal_compression: *d.get(97)?,
        tile_compression: *d.get(98)?,
        tile_type: *d.get(99)?,
        min_zoom: *d.get(100)?,
        max_zoom: *d.get(101)?,
        min_lon_e7: le32s(d, 102)?,
        min_lat_e7: le32s(d, 106)?,
        max_lon_e7: le32s(d, 110)?,
        max_lat_e7: le32s(d, 114)?,
        center_zoom: *d.get(118)?,
        center_lon_e7: le32s(d, 119)?,
        center_lat_e7: le32s(d, 123)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header() -> Vec<u8> {
        let mut h = vec![0u8; 127];
        h[..7].copy_from_slice(b"PMTiles");
        h[7] = 3;
        h
    }

    #[test]
    fn parses() {
        let mut h = header();
        h[8] = 127; // root_offset
        h[16] = 55; // root_length
        h[96] = 1; // clustered
        h[98] = 2; // tile_compression
        h[99] = 1; // tile_type = mvt
        h[100] = 3;
        h[101] = 14;
        // min_lon_e7 = -1800000000 LE at 102
        let v = (-1_800_000_000i32) as u32;
        for i in 0..4 {
            h[102 + i] = ((v >> (8 * i)) & 0xFF) as u8;
        }
        let p = parse(&h).unwrap();
        assert_eq!(p.root_offset, 127);
        assert_eq!(p.root_length, 55);
        assert!(p.clustered);
        assert_eq!(p.tile_type, 1);
        assert_eq!(p.min_zoom, 3);
        assert_eq!(p.max_zoom, 14);
        assert_eq!(p.min_lon_e7, -1_800_000_000);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&header()[..126]).is_none()); // short
        let mut h = header();
        h[7] = 2;
        assert!(parse(&h).is_none()); // version != 3
        h[0] = b'X';
        h[7] = 3;
        assert!(parse(&h).is_none()); // magic
    }
}
