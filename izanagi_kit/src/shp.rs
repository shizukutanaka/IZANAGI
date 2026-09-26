//! ESRI Shapefile `.shp` (the geometry half of the shapefile trio):
//! 100-byte header (BE file code 9994 + length in 16-bit words +
//! LE version/type/bbox) then `[record#][words][content]` records.
//! Doubles are decoded to `Fixed` by hand — no float types.
//!
//! ```
//! use izanagi_kit::shp;
//! let mut f = Vec::new();
//! f.extend_from_slice(&[0, 0, 0x27, 0x0A]);      // file code 9994 BE
//! f.extend_from_slice(&[0; 20]);                 // unused
//! f.extend_from_slice(&[0, 0, 0, 64]);           // file length: 128B = 64 words
//! f.extend_from_slice(&[0xE8, 0x03, 0, 0]);      // version 1000 LE
//! f.extend_from_slice(&[1, 0, 0, 0]);            // type 1 = Point
//! f.extend_from_slice(&[0; 64]);                 // bbox + z + m ranges
//! // record: BE number + BE words, then LE type + two f64
//! let mut rec = Vec::new();
//! rec.extend_from_slice(&[0,0,0,1]);             // record 1
//! rec.extend_from_slice(&[0,0,0,10]);            // 20 bytes = 10 words
//! rec.extend_from_slice(&[1,0,0,0]);             // Point
//! rec.extend_from_slice(&shp::f64_bits(2));      // x = 2.0 LE
//! rec.extend_from_slice(&shp::f64_bits(3));      // y = 3.0 LE
//! f.extend_from_slice(&rec);
//! let s = shp::parse(&f).unwrap();
//! assert_eq!(s.shape_type, 1);
//! let (x, y) = shp::point(&f, &s.records[0]).unwrap();
//! assert_eq!(x.raw(), 2 << 16);
//! assert_eq!(y.raw(), 3 << 16);
//! ```

use crate::fixed::Fixed;
use std::vec::Vec;

fn rb32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        (*d.get(at)? as u32) << 24
            | (*d.get(at + 1)? as u32) << 16
            | (*d.get(at + 2)? as u32) << 8
            | *d.get(at + 3)? as u32,
    )
}
fn rl32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        *d.get(at)? as u32
            | (*d.get(at + 1)? as u32) << 8
            | (*d.get(at + 2)? as u32) << 16
            | (*d.get(at + 3)? as u32) << 24,
    )
}

/// Constructs the 8 little-endian bytes of an IEEE-754 double for
/// small integral values — a fixture helper so callers don't need
/// float literals.
pub fn f64_bits(int_part: u32) -> [u8; 8] {
    let v = int_part as u64;
    let (e, m) = if v == 0 {
        (0u64, 0u64)
    } else {
        let lz = 63 - v.leading_zeros();
        let mant = v & !(1u64 << lz);
        (lz as u64 + 1023, mant << (52 - lz))
    };
    let bits = (e << 52) | m;
    [
        bits as u8,
        (bits >> 8) as u8,
        (bits >> 16) as u8,
        (bits >> 24) as u8,
        (bits >> 32) as u8,
        (bits >> 40) as u8,
        (bits >> 48) as u8,
        (bits >> 56) as u8,
    ]
}

/// Reads a little-endian u64 (the wire form of f64 values).
fn rd64(d: &[u8], at: usize) -> Option<u64> {
    let s = d.get(at..at + 8)?;
    Some(
        s[0] as u64
            | (s[1] as u64) << 8
            | (s[2] as u64) << 16
            | (s[3] as u64) << 24
            | (s[4] as u64) << 32
            | (s[5] as u64) << 40
            | (s[6] as u64) << 48
            | (s[7] as u64) << 56,
    )
}

/// Decodes an f64 bit pattern to `Fixed` (rounds toward zero;
/// `None` on NaN/inf/overflow of the 48-bit range).
pub fn f64_fixed(bits: u64) -> Option<Fixed> {
    let exp = ((bits >> 52) & 0x7FF) as i64;
    let mant = bits & 0xF_FFFF_FFFF_FFFF;
    let sign = if bits >> 63 == 1 { -1i128 } else { 1i128 };
    if exp == 0x7FF {
        return None;
    }
    let val: i128 = if exp == 0 {
        0 // subnormal — far below Q16.16 resolution
    } else {
        let m = (1i128 << 52) | mant as i128;
        let shift = exp - 1023 - 52 + 16;
        if shift >= 0 {
            if shift > 70 {
                return None; // overflow beyond i128
            }
            m << shift as u32
        } else if shift < -128 {
            0
        } else {
            m >> (-shift) as u32
        }
    };
    let raw = sign * val;
    if raw > i32::MAX as i128 || raw < i32::MIN as i128 {
        return None;
    }
    Some(Fixed::from_raw(raw as i32))
}

/// One shapefile record header.
#[derive(Clone, Debug)]
pub struct Record {
    /// Record number (1-based on disk).
    pub number: u32,
    /// File offset of the record content.
    pub offset: usize,
    /// Content length in bytes (from the 16-bit-words field).
    pub size: usize,
    /// The record's own shape type (LE u32).
    pub shape_type: u32,
}

/// A parsed `.shp` file.
#[derive(Clone, Debug)]
pub struct Shp {
    /// File-level shape type (header word; 0 = Null, 1 = Point,
    /// 3 = PolyLine, 5 = Polygon, 8 = MultiPoint…).
    pub shape_type: u32,
    /// Bounding box as four raw f64 bit patterns
    /// (xmin, ymin, xmax, ymax).
    pub bbox_bits: [u64; 4],
    /// Records.
    pub records: Vec<Record>,
}

/// Shape-type name.
pub fn type_name(ty: u32) -> &'static str {
    match ty {
        0 => "null",
        1 => "point",
        3 => "polyline",
        5 => "polygon",
        8 => "multipoint",
        11 => "pointz",
        13 => "polylinez",
        15 => "polygonz",
        18 => "multipointz",
        21 => "pointm",
        23 => "polylinem",
        25 => "polygonm",
        28 => "multipointm",
        31 => "multipatch",
        _ => "unknown",
    }
}

/// Parses the `.shp` container: 100-byte header + record chain.
/// `None` on bad magic/version, length mismatch, or truncation.
pub fn parse(d: &[u8]) -> Option<Shp> {
    if rb32(d, 0)? != 9994 {
        return None;
    }
    let words = rb32(d, 24)? as usize;
    let file_len = words.checked_mul(2)?;
    if file_len != d.len() {
        return None; // the header's length must match exactly
    }
    if rl32(d, 28)? != 1000 {
        return None;
    }
    let shape_type = rl32(d, 32)?;
    let mut bbox_bits = [0u64; 4];
    for (i, b) in bbox_bits.iter_mut().enumerate() {
        *b = rd64(d, 36 + i * 8)?;
    }
    let mut records = Vec::new();
    let mut at = 100usize;
    while at < d.len() {
        let number = rb32(d, at)?;
        let content_words = rb32(d, at + 4)? as usize;
        let size = content_words.checked_mul(2)?;
        let offset = at + 8;
        let end = offset.checked_add(size)?;
        if end > d.len() {
            return None;
        }
        let shape_type = rl32(d, offset)?;
        records.push(Record {
            number,
            offset,
            size,
            shape_type,
        });
        at = end;
    }
    Some(Shp {
        shape_type,
        bbox_bits,
        records,
    })
}

/// `Record` → `(x, y)` for Point records (type 1/11/21).
pub fn point(d: &[u8], r: &Record) -> Option<(Fixed, Fixed)> {
    if r.size < 4 + 16 {
        return None;
    }
    let x = f64_fixed(rd64(d, r.offset + 4)?)?;
    let y = f64_fixed(rd64(d, r.offset + 12)?)?;
    Some((x, y))
}

/// `Record` → all `(x, y)` vertices for MultiPoint/PolyLine/Polygon
/// records (types 3/5/8 and Z variants) — layout: bbox(32) +
/// numParts + numPoints + parts[] + points[].
pub fn points(d: &[u8], r: &Record) -> Option<Vec<(Fixed, Fixed)>> {
    let npts = rl32(d, r.offset + 40)? as usize;
    let nparts = rl32(d, r.offset + 36)? as usize;
    if npts > (1 << 22) || nparts > (1 << 20) {
        return None;
    }
    let pts_at = r
        .offset
        .checked_add(44)?
        .checked_add(nparts.checked_mul(4)?)?;
    if pts_at.checked_add(npts.checked_mul(16)?)? > r.offset + r.size {
        return None;
    }
    let mut out = Vec::with_capacity(npts);
    for i in 0..npts {
        let x = f64_fixed(rd64(d, pts_at + i * 16)?)?;
        let y = f64_fixed(rd64(d, pts_at + i * 16 + 8)?)?;
        out.push((x, y));
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(words_total: u32, ty: u32) -> Vec<u8> {
        let mut f = Vec::new();
        f.extend_from_slice(&[0, 0, 0x27, 0x0A]); // 9994 BE
        f.extend_from_slice(&[0; 20]); // unused
        f.extend_from_slice(&[
            (words_total >> 24) as u8,
            (words_total >> 16) as u8,
            (words_total >> 8) as u8,
            words_total as u8,
        ]);
        f.extend_from_slice(&[0xE8, 0x03, 0, 0]); // 1000 LE
        f.extend_from_slice(&[ty as u8, 0, 0, 0]);
        f.extend_from_slice(&[0; 64]); // bbox + z/m
        f
    }

    fn record(no: u32, body: &[u8]) -> Vec<u8> {
        let words = (body.len() / 2) as u32;
        let mut r = Vec::new();
        r.extend_from_slice(&[
            (no >> 24) as u8,
            (no >> 16) as u8,
            (no >> 8) as u8,
            no as u8,
        ]);
        r.extend_from_slice(&[
            (words >> 24) as u8,
            (words >> 16) as u8,
            (words >> 8) as u8,
            words as u8,
        ]);
        r.extend_from_slice(body);
        r
    }

    #[test]
    fn point_record() {
        let mut body = vec![1, 0, 0, 0]; // type Point
        body.extend_from_slice(&f64_bits(2));
        body.extend_from_slice(&f64_bits(3));
        let total_words = (100 + 8 + body.len()) / 2;
        let mut f = header(total_words as u32, 1);
        f.extend_from_slice(&record(1, &body));
        let s = parse(&f).unwrap();
        assert_eq!(s.shape_type, 1);
        assert_eq!(type_name(s.shape_type), "point");
        let (x, y) = point(&f, &s.records[0]).unwrap();
        assert_eq!(x.raw(), 2 << 16);
        assert_eq!(y.raw(), 3 << 16);
    }

    #[test]
    fn malformed_degrades() {
        assert!(parse(&[]).is_none());
        assert!(parse(&[0; 100]).is_none()); // magic 0
        let mut f = header(50, 1); // 50 words = the 100-byte header alone
        f.extend_from_slice(&[0; 0]); // no records — len must still match
        assert!(parse(&f).is_some()); // header-only file
        let mut bad = header(1_000_000, 1);
        bad.extend_from_slice(&[0; 8]);
        assert!(parse(&bad).is_none()); // declared length ≠ actual
        let mut bad2 = header(60, 1);
        bad2.extend_from_slice(&record(1, &[1, 0, 0, 0]));
        // content words says 2 words=4B but we claim file len 120B…
        bad2[24] = 0;
        bad2[25] = 0;
        bad2[26] = 0;
        bad2[27] = 60; // words=60 → 120B actual is 112 → mismatch
        assert!(parse(&bad2).is_none());
    }

    #[test]
    fn f64_fixed_decodes() {
        // 1.0 = 0x3FF0..0 → Fixed 1<<16
        let bits = 0x3FF0_0000_0000_0000u64;
        assert_eq!(f64_fixed(bits).unwrap().raw(), 1 << 16);
        // -2.5
        let neg = 0xC004_0000_0000_0000u64;
        assert_eq!(f64_fixed(neg).unwrap().raw(), -(5 << 15));
        // NaN/inf rejected
        assert!(f64_fixed(0x7FF0_0000_0000_0001).is_none());
        assert!(f64_fixed(0x7FF0_0000_0000_0000).is_none());
        // zero
        assert_eq!(f64_fixed(0).unwrap().raw(), 0);
    }
}
