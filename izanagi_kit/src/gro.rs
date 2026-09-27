//! Gromacs `.gro` — title line, atom count, then fixed-width atom
//! records `resid(5) resname(5) atomname(5) atomnr(5) x(8.3) y z
//! [vx vy vz]` and a final box-vector line.
//!
//! Coordinates are ×10⁶ micro-units (nm); velocities too when present.
//!
//! ```
//! use izanagi_kit::gro::parse;
//!
//! let g = parse(b"water\n    1\n    1SOL     OW    1   0.230   0.630   0.113\n   1.00000   1.00000   1.00000\n").unwrap();
//! assert_eq!(g.atoms[0].resname, "SOL");
//! assert_eq!(g.atoms[0].x, 230_000);
//! assert_eq!(g.boxv, [1_000_000, 1_000_000, 1_000_000]);
//! ```

use std::string::String;
use std::vec::Vec;

/// One atom record.
#[derive(Clone, Debug)]
pub struct Atom {
    /// Residue number.
    pub resid: u32,
    /// Residue name (5 chars).
    pub resname: String,
    /// Atom name (5 chars).
    pub name: String,
    /// Atom number.
    pub nr: u32,
    /// X ×10⁶.
    pub x: i64,
    /// Y ×10⁶.
    pub y: i64,
    /// Z ×10⁶.
    pub z: i64,
    /// Velocities ×10⁶, present when the record carried them.
    pub v: Option<[i64; 3]>,
}

/// A parsed `.gro` file.
#[derive(Clone, Debug)]
pub struct Gro {
    /// Title line.
    pub title: String,
    /// Atom records.
    pub atoms: Vec<Atom>,
    /// Box vectors (first three diagonal terms) ×10⁶.
    pub boxv: [i64; 3],
}

fn col(line: &str, a: usize, b: usize) -> Option<String> {
    let b = b.min(line.len());
    if a >= b {
        return None;
    }
    Some(line.get(a..b)?.trim().to_string())
}

fn num(s: &str) -> Option<i64> {
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

/// Parse a `.gro` file. `None` unless every declared record parses.
pub fn parse(d: &[u8]) -> Option<Gro> {
    let text = std::str::from_utf8(d).ok()?;
    let mut lines = text.split('\n');
    let title = lines.next()?.trim_end_matches('\r').to_string();
    let n: usize = lines.next()?.trim().parse().ok()?;
    if n == 0 || n > 9_999_999 {
        return None;
    }
    let mut atoms = Vec::new();
    for _ in 0..n {
        let l = lines.next()?.trim_end_matches('\r').to_string();
        let resid: u32 = col(&l, 0, 5)?.parse().ok()?;
        let resname = col(&l, 5, 10)?;
        let name = col(&l, 10, 15)?;
        let nr: u32 = col(&l, 15, 20)?.parse().ok()?;
        let x = num(&col(&l, 20, 28)?)?;
        let y = num(&col(&l, 28, 36)?)?;
        let z = num(&col(&l, 36, 44)?)?;
        let v = if l.len() >= 68 {
            Some([
                num(&col(&l, 44, 52)?)?,
                num(&col(&l, 52, 60)?)?,
                num(&col(&l, 60, 68)?)?,
            ])
        } else {
            None
        };
        atoms.push(Atom {
            resid,
            resname,
            name,
            nr,
            x,
            y,
            z,
            v,
        });
    }
    let bx = lines.next()?.trim_end_matches('\r').to_string();
    let mut it = bx.split_whitespace();
    let boxv = [num(it.next()?)?, num(it.next()?)?, num(it.next()?)?];
    Some(Gro { title, atoms, boxv })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_gro() {
        let g = parse(b"water\n    2\n    1SOL     OW    1   0.230   0.630   0.113\n    1SOL    HW1    2   0.137   0.626   0.150\n   1.00000   1.00000   1.00000\n").unwrap();
        assert_eq!(g.title, "water");
        assert_eq!(g.atoms.len(), 2);
        assert_eq!(g.atoms[1].name, "HW1");
        assert_eq!(g.atoms[0].x, 230_000);
        assert_eq!(g.atoms[0].z, 113_000);
        assert!(g.atoms[0].v.is_none());
        assert_eq!(g.boxv[0], 1_000_000);
    }

    #[test]
    fn velocities() {
        let l = b"t\n    1\n    1SOL     OW    1   0.000   0.000   0.000  0.0100  0.0200 -0.0300\n   2.00000   2.00000   2.00000\n";
        let g = parse(l).unwrap();
        assert_eq!(g.atoms[0].v, Some([10_000, 20_000, -30_000]));
    }

    #[test]
    fn rejects_bad() {
        assert!(parse(b"t\nabc\n").is_none());
        assert!(parse(b"t\n    1\nshort\n 1 1 1\n").is_none());
        assert!(parse(b"t\n    1\n    1SOL     OW    1   0.230   0.630   0.113\n").is_none());
        // no box line
    }
}
