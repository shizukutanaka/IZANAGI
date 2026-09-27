//! Gaussian cube files — two comment lines, `natoms x0 y0 z0`, three
//! axis lines `nvx vx vy vz`, one `Z q x y z` line per atom, then the
//! volumetric grid values.
//!
//! All numbers are folded to ×10⁶ micro-units (integer-only);
//! `charges`/`coords` are micro too. Grid values are only counted, not
//! stored.
//!
//! ```
//! use izanagi_kit::cube::parse;
//!
//! let c = parse(b"c1\nc2\n   1    0.0    0.0    0.0\n   2    1.0    0.0    0.0\n   2    0.0    1.0    0.0\n   1    0.0    0.0    1.0\n    8    0.0    0.0    0.0    0.0\n 0.1 0.2 0.3 0.4 0.5 0.6 0.7 0.8\n").unwrap();
//! assert_eq!(c.atoms[0].z, 8);
//! assert_eq!(c.voxels, 8);
//! ```

use std::string::String;
use std::vec::Vec;

/// One atom line.
#[derive(Clone, Debug)]
pub struct Atom {
    /// Atomic number.
    pub z: u32,
    /// Charge ×10⁶.
    pub charge: i64,
    /// X ×10⁶.
    pub x: i64,
    /// Y ×10⁶.
    pub y: i64,
    /// Z ×10⁶.
    pub zpos: i64,
}

/// A parsed `.cube` file.
#[derive(Clone, Debug)]
pub struct Cube {
    /// First comment line.
    pub comment1: String,
    /// Second comment line.
    pub comment2: String,
    /// Declared atom count (negative = orbital dataset ids follow atoms).
    pub natoms: i64,
    /// Origin ×10⁶.
    pub origin: [i64; 3],
    /// Axis `(count, vector×10⁶)` triples.
    pub axes: [(u32, [i64; 3]); 3],
    /// Atom lines.
    pub atoms: Vec<Atom>,
    /// Volumetric values seen (count only).
    pub voxels: usize,
}

fn num(s: &str) -> Option<i64> {
    // supports optional exponent: mantissa ×10⁶ shifted by e
    let (mant, exp) = match s.find(['e', 'E']) {
        Some(p) => {
            let (m, e) = s.split_at(p);
            (m, e[1..].parse::<i64>().ok()?)
        }
        None => (s, 0),
    };
    let v = micro(mant)?;
    if exp >= 0 {
        v.checked_mul(10i64.checked_pow(exp as u32)?)
    } else {
        // divide by 10^-exp keeping micro precision when possible
        let mut v = v;
        for _ in 0..(-exp) {
            if v % 10 != 0 {
                return None;
            }
            v /= 10;
        }
        Some(v)
    }
}

fn micro(s: &str) -> Option<i64> {
    let (neg, s) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    let (int, frac) = match s.split_once('.') {
        Some((a, b)) => (a, b),
        None => (s, ""),
    };
    if int.is_empty() && frac.is_empty() || frac.len() > 6 {
        return None;
    }
    if !int.bytes().all(|c| c.is_ascii_digit()) || !frac.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let ip = int.parse::<i64>().unwrap_or(0);
    let mut fp = 0i64;
    for c in frac.bytes() {
        fp = fp.checked_mul(10)?.checked_add((c - b'0') as i64)?;
    }
    for _ in frac.len()..6 {
        fp *= 10;
    }
    let v = ip.checked_mul(1_000_000)?.checked_add(fp).unwrap_or(0);
    Some(if neg { -v } else { v })
}

fn ws4(l: &str) -> Option<(&str, &str, &str, &str)> {
    let mut it = l.split_whitespace();
    Some((it.next()?, it.next()?, it.next()?, it.next()?))
}

/// Parse a Gaussian cube file.
pub fn parse(d: &[u8]) -> Option<Cube> {
    let text = std::str::from_utf8(d).ok()?;
    let mut lines = text.split('\n');
    let comment1 = lines.next()?.trim_end_matches('\r').to_string();
    let comment2 = lines.next()?.trim_end_matches('\r').to_string();
    let l3 = lines.next()?.trim_end_matches('\r').to_string();
    let (n, x0, y0, z0) = ws4(&l3)?;
    let natoms: i64 = n.parse().ok()?;
    let origin = [num(x0)?, num(y0)?, num(z0)?];
    let mut axes = [(0u32, [0i64; 3]); 3];
    for a in axes.iter_mut() {
        let l = lines.next()?.trim_end_matches('\r').to_string();
        let (nv, x, y, z) = ws4(&l)?;
        *a = (nv.parse().ok()?, [num(x)?, num(y)?, num(z)?]);
    }
    let count = natoms.checked_abs()? as usize;
    if count == 0 {
        return None;
    }
    let mut atoms = Vec::new();
    for _ in 0..count {
        let l = lines.next()?.trim_end_matches('\r').to_string();
        let mut it = l.split_whitespace();
        atoms.push(Atom {
            z: it.next()?.parse().ok()?,
            charge: num(it.next()?)?,
            x: num(it.next()?)?,
            y: num(it.next()?)?,
            zpos: num(it.next()?)?,
        });
    }
    // negative natoms: a DSET_IDS line follows the atom block — skip it.
    if natoms < 0 {
        lines.next();
    }
    let mut voxels = 0usize;
    for l in lines {
        for tok in l.split_whitespace() {
            num(tok)?;
            voxels += 1;
        }
    }
    if voxels == 0 {
        return None;
    }
    Some(Cube {
        comment1,
        comment2,
        natoms,
        origin,
        axes,
        atoms,
        voxels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_cube() {
        let c = parse(b"c1\nc2\n   1    0.0    0.0    0.0\n   2    1.0    0.0    0.0\n   2    0.0    1.0    0.0\n   1    0.0    0.0    1.0\n    8    0.0    0.0    0.0    0.0\n 0.1 0.2 0.3 0.4 0.5 0.6 0.7 0.8\n").unwrap();
        assert_eq!(c.natoms, 1);
        assert_eq!(c.axes[0].0, 2);
        assert_eq!(c.axes[0].1[0], 1_000_000);
        assert_eq!(c.atoms[0].z, 8);
        assert_eq!(c.voxels, 8);
    }

    #[test]
    fn exponent_and_negative_natoms() {
        assert_eq!(num("1e-1"), Some(100_000));
        assert_eq!(num("2.5E0"), Some(2_500_000));
        let c = parse(b"a\nb\n  -1 0 0 0\n 1 1 0 0\n 1 0 1 0\n 1 0 0 1\n 8 0 0 0 0\n 1\n 0.5\n")
            .unwrap();
        assert_eq!(c.voxels, 1);
    }

    #[test]
    fn rejects_junk() {
        assert!(parse(b"x\ny\nz\n").is_none());
        assert!(parse(b"a\nb\n 0 0 0 0\n 1 1 0 0\n 1 0 1 0\n 1 0 0 1\n").is_none());
    }
}
