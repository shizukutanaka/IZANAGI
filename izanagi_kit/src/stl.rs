//! STL meshes — [`obj`](crate::obj)'s triangle-soup sibling: ASCII
//! `solid/facet normal/outer loop/vertex/endloop/endfacet/endsolid`
//! and the binary form (80-byte header + u32 count + 50 bytes/tri).
//!
//! Binary coordinates are IEEE-754 `f32` bit patterns; the kit bans
//! float types, so they are decoded to [`Fixed`] by hand (`f32_to_fixed`),
//! rejecting inf/nan and magnitudes beyond Q16.16. ASCII coordinates go
//! through digit arithmetic likewise. [`emit`] writes canonical ASCII.
//!
//! ```
//! use izanagi_kit::stl::{parse_ascii, Stl, Tri};
//! use izanagi_kit::fixed::Fixed;
//! let t = parse_ascii("solid s\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 1 0 0\nvertex 0 1 0\nendloop\nendfacet\nendsolid s\n").unwrap();
//! assert_eq!(t.tris.len(), 1);
//! assert_eq!(t.tris[0].v[1][0], Fixed::ONE);
//! ```

use crate::fixed::Fixed;
use std::string::String;
use std::vec::Vec;

/// One STL facet: normal + 3 vertices.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tri {
    /// Facet normal (`[0,0,0]` when the file leaves it zero).
    pub n: [Fixed; 3],
    /// The three vertices.
    pub v: [[Fixed; 3]; 3],
}

/// A mesh: the triangle list.
#[derive(Clone, Debug, PartialEq)]
pub struct Stl {
    /// Triangles in file order.
    pub tris: Vec<Tri>,
}

/// IEEE-754 binary32 bits → `Fixed`; `None` for inf/nan and values
/// outside Q16.16.
fn f32_to_fixed(bits: u32) -> Option<Fixed> {
    let sign = bits >> 31;
    let exp = (bits >> 23) & 0xFF;
    let frac = bits & 0x7FFFFF;
    if exp == 0xFF {
        return None;
    }
    let (m, s): (i128, i32) = if exp == 0 {
        // subnormal: frac * 2^-149 * 2^16 = frac * 2^-133
        (frac as i128, -133)
    } else {
        ((0x800000 + frac) as i128, exp as i32 - 134)
    };
    let raw: i128 = if s >= 0 {
        m.checked_shl(s as u32)?
    } else if -s >= 127 {
        0 // magnitude below Fixed resolution
    } else {
        m >> (-s)
    };
    if raw > i32::MAX as i128 {
        return None;
    }
    let raw = if sign == 1 { -raw } else { raw };
    Some(Fixed::from_raw(raw as i32))
}

/// Decimal text → `Fixed` (integer + fraction digits, optional sign/exponent).
fn num(s: &str) -> Option<Fixed> {
    let s = s.trim();
    let (s, neg) = match s.strip_prefix('-') {
        Some(r) => (r, true),
        None => (s.strip_prefix('+').unwrap_or(s), false),
    };
    let (s, exp): (&str, i32) = match s.find(['e', 'E']) {
        Some(p) => {
            let e: i32 = s[p + 1..].trim().parse().ok()?;
            if !(-20..=20).contains(&e) {
                return None;
            }
            (&s[..p], e)
        }
        None => (s, 0),
    };
    let (ip, fp) = match s.split_once('.') {
        Some((a, b)) => (a, b),
        None => (s, ""),
    };
    if ip.is_empty() && fp.is_empty() {
        return None;
    }
    if !ip.bytes().all(|b| b.is_ascii_digit()) || !fp.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let mut v: i128 = 0;
    for b in ip.bytes() {
        v = v.checked_mul(10)?.checked_add((b - b'0') as i128)?;
        if v > i32::MAX as i128 * 2 {
            return None;
        }
    }
    let mut fr: i128 = 0;
    let mut fd: i128 = 1;
    for b in fp.bytes().take(9) {
        fr = fr * 10 + (b - b'0') as i128;
        fd *= 10;
    }
    let mut raw = v * 65536 + fr * 65536 / fd;
    let mut e = exp;
    while e > 0 {
        raw = raw.checked_mul(10)?;
        e -= 1;
    }
    while e < 0 {
        raw /= 10;
        e += 1;
    }
    if raw > i32::MAX as i128 {
        return None;
    }
    Some(Fixed::from_raw(if neg { -raw } else { raw } as i32))
}

/// Parse the ASCII form; `None` on missing keywords or bad numbers.
pub fn parse_ascii(src: &str) -> Option<Stl> {
    let mut tris = Vec::new();
    let mut it = src.lines().map(str::trim).filter(|l| !l.is_empty());
    if !it.next()?.starts_with("solid") {
        return None;
    }
    let mut cur: Option<Tri> = None;
    let mut verts = 0;
    for line in it {
        if let Some(r) = line.strip_prefix("facet") {
            let w: Vec<&str> = r.split_whitespace().collect();
            match w.as_slice() {
                ["normal", x, y, z] => {
                    cur = Some(Tri {
                        n: [num(x)?, num(y)?, num(z)?],
                        v: [[Fixed::ZERO; 3]; 3],
                    });
                    verts = 0;
                }
                [] => {
                    cur = Some(Tri {
                        n: [Fixed::ZERO; 3],
                        v: [[Fixed::ZERO; 3]; 3],
                    });
                    verts = 0;
                }
                _ => return None,
            }
            continue;
        }
        if let Some(r) = line.strip_prefix("vertex") {
            let w: Vec<&str> = r.split_whitespace().collect();
            if w.len() != 3 {
                return None;
            }
            let t = cur.as_mut()?;
            if verts >= 3 {
                return None;
            }
            t.v[verts] = [num(w[0])?, num(w[1])?, num(w[2])?];
            verts += 1;
            continue;
        }
        if line == "outer loop" || line == "endloop" {
            continue;
        }
        if line == "endfacet" {
            let t = cur.take()?;
            if verts != 3 {
                return None;
            }
            tris.push(t);
            continue;
        }
        if line.starts_with("endsolid") {
            return Some(Stl { tris });
        }
        return None;
    }
    // ended without endsolid → still accept if at least one tri?
    // Strict STL requires endsolid.
    None
}

fn le32(d: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_le_bytes([
        *d.get(i)?,
        *d.get(i + 1)?,
        *d.get(i + 2)?,
        *d.get(i + 3)?,
    ]))
}

/// Parse the binary form: 80-byte header + u32 count + 50 bytes/tri
/// (12 `f32` + u16 attribute). `None` on size/count mismatch or
/// non-finite/out-of-range coordinates.
pub fn parse_bin(d: &[u8]) -> Option<Stl> {
    if d.len() < 84 {
        return None;
    }
    let n = le32(d, 80)? as usize;
    if d.len() != 84 + n.checked_mul(50)? {
        return None;
    }
    let mut tris = Vec::with_capacity(n);
    for t in 0..n {
        let o = 84 + t * 50;
        let mut f = [Fixed::ZERO; 12];
        for (k, slot) in f.iter_mut().enumerate() {
            *slot = f32_to_fixed(le32(d, o + k * 4)?)?;
        }
        tris.push(Tri {
            n: [f[0], f[1], f[2]],
            v: [[f[3], f[4], f[5]], [f[6], f[7], f[8]], [f[9], f[10], f[11]]],
        });
    }
    Some(Stl { tris })
}

/// Auto-detecting parse: exact binary size match wins, else ASCII.
pub fn parse(d: &[u8]) -> Option<Stl> {
    if let Ok(s) = std::str::from_utf8(d) {
        if s.trim_start().starts_with("solid") {
            if let Some(v) = parse_ascii(s) {
                return Some(v);
            }
        }
    }
    parse_bin(d)
}

/// Exact decimal emission: Q16.16 fractions are `k/65536`, which
/// always terminates in ≤16 digits.
fn emit_f(f: Fixed, s: &mut String) {
    let raw = f.raw();
    if raw < 0 {
        s.push('-');
    }
    let a = raw.unsigned_abs() as u64;
    s.push_str(&std::format!("{}", a / 65536));
    let mut fr = a % 65536;
    if fr != 0 {
        s.push('.');
        while fr != 0 {
            fr *= 10;
            s.push((b'0' + (fr / 65536) as u8) as char);
            fr %= 65536;
        }
    }
}

/// Canonical ASCII emission.
pub fn emit(stl: &Stl) -> String {
    let mut s = String::from("solid izanagi\n");
    for t in &stl.tris {
        s.push_str(" facet normal ");
        for k in 0..3 {
            emit_f(t.n[k], &mut s);
            s.push(' ');
        }
        s.push_str("\n  outer loop\n");
        for v in t.v.iter() {
            s.push_str("   vertex ");
            for c in v.iter() {
                emit_f(*c, &mut s);
                s.push(' ');
            }
            s.push('\n');
        }
        s.push_str("  endloop\n endfacet\n");
    }
    s.push_str("endsolid izanagi\n");
    s
}

/// Axis-aligned bounds; `None` when empty.
pub fn bbox(stl: &Stl) -> Option<([Fixed; 3], [Fixed; 3])> {
    let mut lo = [i32::MAX; 3];
    let mut hi = [i32::MIN; 3];
    for t in &stl.tris {
        for v in t.v.iter() {
            for k in 0..3 {
                lo[k] = lo[k].min(v[k].raw());
                hi[k] = hi[k].max(v[k].raw());
            }
        }
    }
    if stl.tris.is_empty() {
        return None;
    }
    Some((
        [
            Fixed::from_raw(lo[0]),
            Fixed::from_raw(lo[1]),
            Fixed::from_raw(lo[2]),
        ],
        [
            Fixed::from_raw(hi[0]),
            Fixed::from_raw(hi[1]),
            Fixed::from_raw(hi[2]),
        ],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TETRA: &str = "solid tet\nfacet normal 0 0 -1\nouter loop\nvertex 0 0 0\nvertex 1 0 0\nvertex 0 1 0\nendloop\nendfacet\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 0 1 0\nvertex 0 0 1\nendloop\nendfacet\nendsolid tet\n";

    fn bin_tri(verts: &[[u32; 3]; 4]) -> Vec<u8> {
        let mut d = vec![0u8; 84 + 50];
        d[80..84].copy_from_slice(&1u32.to_le_bytes());
        for (i, v) in verts.iter().enumerate() {
            for (k, &x) in v.iter().enumerate() {
                d[84 + i * 12 + k * 4..][..4].copy_from_slice(&x.to_le_bytes());
            }
        }
        d
    }

    #[test]
    fn ascii_parse() {
        let s = parse_ascii(TETRA).unwrap();
        assert_eq!(s.tris.len(), 2);
        assert_eq!(s.tris[0].v[1], [Fixed::ONE, Fixed::ZERO, Fixed::ZERO]);
        assert_eq!(s.tris[1].v[2][2], Fixed::ONE);
        let (lo, hi) = bbox(&s).unwrap();
        assert_eq!(lo, [Fixed::ZERO; 3]);
        assert_eq!(hi, [Fixed::ONE; 3]);
    }

    #[test]
    fn binary_parse() {
        // tri with verts (0,0,0) (1,0,0) (0,1,0): f32 1.0 = 0x3F800000
        let one = 0x3F80_0000u32;
        let d = bin_tri(&[[0, 0, 0], [one, 0, 0], [0, one, 0], [0, 0, one]]);
        let s = parse_bin(&d).unwrap();
        assert_eq!(s.tris[0].v[0][0], Fixed::ONE);
        assert_eq!(s.tris[0].v[1][1], Fixed::ONE);
        assert_eq!(s.tris[0].v[2][2], Fixed::ONE);
        assert_eq!(s.tris[0].n[2], Fixed::ZERO);
    }

    #[test]
    fn binary_sizes_checked() {
        let d = bin_tri(&[[0; 3]; 4]);
        assert_eq!(parse_bin(&d[..80]), None); // short header
        let mut t = d.clone();
        t[83] = 2; // count lies
        assert_eq!(parse_bin(&t), None);
        assert_eq!(parse_bin(&d[..d.len() - 1]), None);
        // NaN/inf rejected
        let mut bad = bin_tri(&[[0x7FC0_0000; 3]; 4]);
        assert_eq!(parse_bin(&bad), None);
        bad = bin_tri(&[[0x7F80_0000; 3]; 4]); // +inf
        assert_eq!(parse_bin(&bad), None);
    }

    #[test]
    fn autodetect() {
        assert_eq!(parse(TETRA.as_bytes()).unwrap().tris.len(), 2);
        let one = 0x3F80_0000u32;
        let d = bin_tri(&[[0, 0, 0], [one, 0, 0], [0, one, 0], [0, 0, one]]);
        assert_eq!(parse(&d).unwrap().tris.len(), 1);
    }

    #[test]
    fn emit_roundtrip_ascii() {
        let s = parse_ascii(TETRA).unwrap();
        let s2 = parse_ascii(&emit(&s)).unwrap();
        assert_eq!(s.tris.len(), s2.tris.len());
        assert_eq!(s.tris[0].v, s2.tris[0].v);
    }

    #[test]
    fn malformed_ascii_rejected() {
        for bad in [
            "",
            "facet normal 0 0\n", // short normal
            "solid s\nfacet normal 0 0 1\nouter loop\nvertex 0 0\nendloop\nendfacet\nendsolid",
            "solid s\nvertex 0 0 0\nendsolid",
        ] {
            assert_eq!(parse_ascii(bad), None, "{bad}");
        }
        // empty-but-wellformed is a valid empty mesh
        assert_eq!(parse_ascii("solid s\nendsolid\n").unwrap().tris.len(), 0);
    }

    #[test]
    fn determinism_twice() {
        assert_eq!(parse_ascii(TETRA), parse_ascii(TETRA));
        assert_eq!(
            emit(&parse_ascii(TETRA).unwrap()),
            emit(&parse_ascii(TETRA).unwrap())
        );
    }
}
