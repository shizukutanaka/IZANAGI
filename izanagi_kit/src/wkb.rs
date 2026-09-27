//! Minimal reader for WKB (Well-Known Binary, OGC): `endian(1B)` +
//! `type(u32)` + payload. This module reports the geometry type and — for
//! container-free types — the coordinate count; coordinates are read as
//! raw `u32` bit-pairs only for counting purposes (never as floats).
//!
//! Types: 1 Point, 2 LineString, 3 Polygon, 4–6 Multi*, 7
//! GeometryCollection. Z/M flag bits (0x80000000 range or +1000/2000
//! conventions) are masked off into `z`/`m` booleans.
//!
//! ```
//! use izanagi_kit::wkb::{parse, Wkb};
//!
//! // Point(LE): 01 | 1 | x | y
//! let d = [1u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
//! let w = parse(&d).unwrap();
//! assert_eq!(w.kind, Wkb::Point);
//! assert!(!w.z && !w.m);
//! ```

/// Recognised WKB geometry types (base code, without Z/M/EWKB flags).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wkb {
    /// `1` — Point
    Point,
    /// `2` — LineString
    LineString,
    /// `3` — Polygon
    Polygon,
    /// `4` — MultiPoint
    MultiPoint,
    /// `5` — MultiLineString
    MultiLineString,
    /// `6` — MultiPolygon
    MultiPolygon,
    /// `7` — GeometryCollection
    GeometryCollection,
}

/// A parsed WKB geometry header (payload structure is summarised, not
/// materialised).
#[derive(Debug)]
pub struct WkbGeom {
    /// Base geometry kind.
    pub kind: Wkb,
    /// Little-endian (ISO WKB `0` is big, `1` is little — this field is
    /// `true` for LE).
    pub little_endian: bool,
    /// Z ordinate present (EWKB `0x8000_0000` or `+1000` convention).
    pub z: bool,
    /// M ordinate present (EWKB `0x4000_0000` or `+2000` convention).
    pub m: bool,
    /// Element/point count where cheap to read: LineString → npoints,
    /// Polygon → nrings, Multi*/Collection → ngeoms, Point → 1.
    pub count: u32,
}

fn read32(d: &[u8], at: usize, le: bool) -> Option<u32> {
    let b0 = *d.get(at)? as u32;
    let b1 = *d.get(at + 1)? as u32;
    let b2 = *d.get(at + 2)? as u32;
    let b3 = *d.get(at + 3)? as u32;
    Some(if le {
        b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
    } else {
        (b0 << 24) | (b1 << 16) | (b2 << 8) | b3
    })
}

fn base_type(raw: u32) -> (Option<Wkb>, bool, bool) {
    // EWKB carries Z/M/SRID as high flag bits; ISO adds 1000(Z), 2000(M),
    // 3000(ZM) to the base type code.
    let (base, z, m) = if raw & 0xE000_0000 != 0 {
        (raw & 0xFFFF, raw & 0x8000_0000 != 0, raw & 0x4000_0000 != 0)
    } else if raw >= 3000 {
        (raw - 3000, true, true)
    } else if raw >= 2000 {
        (raw - 2000, false, true)
    } else if raw >= 1000 {
        (raw - 1000, true, false)
    } else {
        (raw, false, false)
    };
    let kind = match base {
        1 => Some(Wkb::Point),
        2 => Some(Wkb::LineString),
        3 => Some(Wkb::Polygon),
        4 => Some(Wkb::MultiPoint),
        5 => Some(Wkb::MultiLineString),
        6 => Some(Wkb::MultiPolygon),
        7 => Some(Wkb::GeometryCollection),
        _ => None,
    };
    (kind, z, m)
}

/// Parse the first WKB geometry header. `None` on a bad endian byte,
/// unknown base type, or a truncated count field.
pub fn parse(d: &[u8]) -> Option<WkbGeom> {
    let le = match *d.first()? {
        0 => false,
        1 => true,
        _ => return None,
    };
    let raw = read32(d, 1, le)?;
    let (kind, z, m) = base_type(raw);
    let kind = kind?;
    let count = match kind {
        Wkb::Point => 1,
        _ => read32(d, 5, le)?,
    };
    Some(WkbGeom {
        kind,
        little_endian: le,
        z,
        m,
        count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        // LE Point
        let d = [1u8, 1, 0, 0, 0, 0, 0, 0, 0];
        let w = parse(&d).unwrap();
        assert_eq!(w.kind, Wkb::Point);
        assert!(w.little_endian);
        assert_eq!(w.count, 1);
        // BE LineString with 3 points: 00 | 2 | 3
        let d = [0u8, 0, 0, 0, 2, 0, 0, 0, 3];
        let w = parse(&d).unwrap();
        assert_eq!(w.kind, Wkb::LineString);
        assert!(!w.little_endian);
        assert_eq!(w.count, 3);
        // EWKB Z Point: 0x80000001 LE
        let d = [1u8, 1, 0, 0, 0x80, 0, 0, 0, 0];
        let w = parse(&d).unwrap();
        assert_eq!(w.kind, Wkb::Point);
        assert!(w.z);
        // ISO 3000-style MZ LineString: 3002
        let d = [1u8, 0xBA, 0x0B, 0, 0, 9, 0, 0, 0];
        let w = parse(&d).unwrap();
        assert_eq!(w.kind, Wkb::LineString);
        assert!(w.z && w.m);
        assert_eq!(w.count, 9);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&[2u8]).is_none()); // endian byte must be 0/1
        assert!(parse(&[1u8]).is_none()); // truncated type
        let mut d = [1u8, 99, 0, 0, 0, 0];
        assert!(parse(&d).is_none()); // unknown type 99
        d[1] = 3;
        assert!(parse(&d).is_none()); // polygon count field truncated
    }
}
