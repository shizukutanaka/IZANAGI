//! Wavefront OBJ geometry — the plain-text 3-D mesh format: `v x y z`
//! vertices, `vt u v` texture coords, `vn x y z` normals, and `f`
//! faces whose vertices are `v`/`v/vt`/`v//vn`/`v/vt/vn` triples,
//! 1-indexed (negative = relative to the most recent element).
//! `o`/`g`/`s`/`usemtl`/`mtllib` lines are skipped, `#` comments too.
//!
//! Coordinates are [`Fixed`], parsed by decimal-digit arithmetic — no
//! floats. Parsing is total (`None` on malformed lines); faces are
//! stored triangulated-by-fan so downstream code sees only triangles.
//!
//! ```
//! use izanagi_kit::obj::{parse, Obj};
//!
//! let m = parse("v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1 2 3 4\n").unwrap();
//! assert_eq!(m.v.len(), 4);
//! assert_eq!(m.triangles().len(), 2); // quad → fan
//! ```

use crate::fixed::Fixed;
use std::vec::Vec;

/// A face vertex: `(v, vt, vn)` resolved to 0-based indices; absent
/// attributes are `u32::MAX`? — no, `Option`.
pub type FaceVertex = (u32, Option<u32>, Option<u32>);

/// The parsed model.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Obj {
    /// `v` positions.
    pub v: Vec<[Fixed; 3]>,
    /// `vt` texture coordinates.
    pub vt: Vec<[Fixed; 2]>,
    /// `vn` normals.
    pub vn: Vec<[Fixed; 3]>,
    /// `f` faces (polygons, ≥3 vertices each, in file order).
    pub f: Vec<Vec<FaceVertex>>,
}

impl Obj {
    /// Fan-triangulated faces as index triples into `v`.
    pub fn triangles(&self) -> Vec<[u32; 3]> {
        let mut out = Vec::new();
        for face in &self.f {
            if face.len() < 3 {
                continue;
            }
            for i in 1..face.len() - 1 {
                out.push([face[0].0, face[i].0, face[i + 1].0]);
            }
        }
        out
    }
    /// Axis-aligned bounding box; `None` when there are no vertices.
    pub fn bbox(&self) -> Option<([Fixed; 3], [Fixed; 3])> {
        let mut it = self.v.iter();
        let first = *it.next()?;
        let mut lo = first;
        let mut hi = first;
        for &p in it {
            for i in 0..3 {
                if p[i] < lo[i] {
                    lo[i] = p[i];
                }
                if p[i] > hi[i] {
                    hi[i] = p[i];
                }
            }
        }
        Some((lo, hi))
    }
}

fn num(s: &str) -> Option<Fixed> {
    let b = s.as_bytes();
    let mut i = 0;
    let neg = match b.first() {
        Some(b'-') => {
            i = 1;
            true
        }
        Some(b'+') => {
            i = 1;
            false
        }
        _ => false,
    };
    let mut ip: i64 = 0;
    let mut saw = false;
    while i < b.len() && b[i].is_ascii_digit() {
        ip = ip.checked_mul(10)?.checked_add((b[i] - b'0') as i64)?;
        if ip > 30_000 {
            return None;
        }
        saw = true;
        i += 1;
    }
    let mut fn_: i64 = 0;
    let mut fd: i64 = 1;
    if i < b.len() && b[i] == b'.' {
        i += 1;
        while i < b.len() && b[i].is_ascii_digit() {
            fn_ = fn_.checked_mul(10)?.checked_add((b[i] - b'0') as i64)?;
            fd = fd.checked_mul(10)?;
            if fd > 1_000_000_000_000 {
                return None;
            }
            saw = true;
            i += 1;
        }
    }
    // Optional exponent `e±d`
    let mut exp: i64 = 0;
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        i += 1;
        let eneg = match b.get(i) {
            Some(b'-') => {
                i += 1;
                true
            }
            Some(b'+') => {
                i += 1;
                false
            }
            _ => false,
        };
        let mut esaw = false;
        while i < b.len() && b[i].is_ascii_digit() {
            exp = exp.checked_mul(10)?.checked_add((b[i] - b'0') as i64)?;
            esaw = true;
            i += 1;
        }
        if !esaw || exp > 20 {
            return None;
        }
        if eneg {
            exp = -exp;
        }
    }
    if !saw || i != b.len() {
        return None;
    }
    let mut num128 = (ip as i128) * (fd as i128) + fn_ as i128;
    let mut den = fd as i128;
    if exp >= 0 {
        for _ in 0..exp {
            num128 = num128.checked_mul(10)?;
        }
    } else {
        for _ in 0..-exp {
            den = den.checked_mul(10)?;
        }
    }
    let raw = num128.checked_mul(65536)? / den;
    if raw > i32::MAX as i128 || raw < i32::MIN as i128 {
        return None;
    }
    Some(Fixed::from_raw(if neg {
        -(raw as i32)
    } else {
        raw as i32
    }))
}

/// `1`/`2/3`/`2//3`/`2/3/4` → `(v, vt, vn)` resolved to 0-based.
/// `n` is the count of the attribute's elements so far (for negative
/// indexing).
fn face_vertex(tok: &str, nv: usize, nt: usize, nn: usize) -> Option<FaceVertex> {
    let mut parts = tok.split('/');
    let v = idx(parts.next()?, nv)?;
    let vt = match parts.next() {
        Some("") => None,
        Some(s) => Some(idx(s, nt)?),
        None => None,
    };
    let vn = match parts.next() {
        Some("") | None => None,
        Some(s) => Some(idx(s, nn)?),
    };
    if parts.next().is_some() {
        return None;
    }
    Some((v, vt, vn))
}

fn idx(s: &str, count: usize) -> Option<u32> {
    let i: i64 = s.parse().ok()?;
    let r = if i > 0 {
        i.checked_sub(1)?
    } else if i < 0 {
        (count as i64).checked_add(i)?
    } else {
        return None;
    };
    if r < 0 || r >= count as i64 || r > u32::MAX as i64 {
        return None;
    }
    Some(r as u32)
}

/// Parse the whole document; `None` on any malformed directive line.
pub fn parse(src: &str) -> Option<Obj> {
    let mut o = Obj::default();
    for raw in src.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut it = line.split_whitespace();
        let kw = it.next()?;
        match kw {
            "v" => {
                let x = num(it.next()?)?;
                let y = num(it.next()?)?;
                let z = num(it.next()?)?;
                o.v.push([x, y, z]);
            }
            "vt" => {
                let u = num(it.next()?)?;
                let v = num(it.next()?)?;
                o.vt.push([u, v]);
            }
            "vn" => {
                let x = num(it.next()?)?;
                let y = num(it.next()?)?;
                let z = num(it.next()?)?;
                o.vn.push([x, y, z]);
            }
            "f" => {
                let mut face = Vec::new();
                for tok in it {
                    face.push(face_vertex(tok, o.v.len(), o.vt.len(), o.vn.len())?);
                }
                if face.len() < 3 {
                    return None;
                }
                o.f.push(face);
            }
            "o" | "g" | "s" | "usemtl" | "mtllib" | "l" | "p" | "curv" | "surf" => {}
            _ => return None,
        }
    }
    Some(o)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quad_fans_into_two_triangles() {
        let m = parse("v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1 2 3 4\n").unwrap();
        assert_eq!(m.v.len(), 4);
        assert_eq!(m.triangles(), vec![[0, 1, 2], [0, 2, 3]]);
        let (lo, hi) = m.bbox().unwrap();
        assert_eq!(lo, [Fixed::ZERO; 3]);
        assert_eq!(hi, [Fixed::ONE, Fixed::ONE, Fixed::ZERO]);
    }

    #[test]
    fn full_vertex_forms() {
        let m = parse(
            "v 0 0 0\nv 1 0 0\nv 1 1 0\nvt 0 0\nvt 1 0\nvt 1 1\nvn 0 0 1\n\
             f 1/1/1 2/2/1 3/3/1\nf 1/1 2/2 3/3\nf 1//1 2//1 3//1\nf 1 2 3\n",
        )
        .unwrap();
        assert_eq!(m.f.len(), 4);
        assert_eq!(m.f[0][0], (0, Some(0), Some(0)));
        assert_eq!(m.f[1][0], (0, Some(0), None));
        assert_eq!(m.f[2][0], (0, None, Some(0)));
        assert_eq!(m.f[3][0], (0, None, None));
    }

    #[test]
    fn negative_indices_are_relative() {
        let m = parse("v 0 0 0\nv 1 0 0\nv 1 1 0\nf -3 -2 -1\n").unwrap();
        assert_eq!(
            m.f[0],
            vec![(0, None, None), (1, None, None), (2, None, None)]
        );
    }

    #[test]
    fn comments_and_skipped_directives() {
        let m = parse(
            "# cube\no Cube\nmtllib m.mtl\ng g1\ns off\nusemtl M\nv 0 0 0\nv 1 0 0\nv 1 1 0\nf 1 2 3\n",
        )
        .unwrap();
        assert_eq!(m.v.len(), 3);
    }

    #[test]
    fn malformed_rejected() {
        for bad in [
            "v 0 0\n",
            "v x 0 0\n",
            "v 0 0 0\nf 1 2\n",
            "v 0 0 0\nf 0 1 2\n",
            "v 0 0 0\nf 2 1 1\n", // index 2 out of range
            "v 0 0 0\nf 1/9 1 1\n",
            "v 0 0 0\nf -4 1 1\n",
            "q 1 2 3\n",
            "v 0 0 0\nf 1/2/3/4 1 1\n",
        ] {
            assert_eq!(parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn empty_is_empty() {
        let m = parse("").unwrap();
        assert_eq!(m.bbox(), None);
        assert!(m.triangles().is_empty());
    }

    #[test]
    fn exponent_and_precision() {
        let m = parse("v 1e0 -2.5 0.125\n").unwrap();
        assert_eq!(m.v[0][0], Fixed::ONE);
        assert_eq!(m.v[0][1], Fixed::from_ratio(-5, 2));
        assert_eq!(m.v[0][2], Fixed::from_ratio(1, 8));
    }

    #[test]
    fn determinism_twice() {
        let src = "v 0 0 0\nv 1 0 0\nv 1 1 0\nf 1 2 3\n";
        assert_eq!(parse(src), parse(src));
    }
}
