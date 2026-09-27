//! MDL Molfile (V2000) — the atom/bond record format used inside `.mol`
//! and `.sdf` files. Fixed-width counts line, atom block, bond block,
//! then `M`-property records terminated by `M  END`.
//!
//! Coordinates are ×10⁶ micro-units; the parser is float-free.
//!
//! ```
//! use izanagi_kit::mol::parse;
//!
//! let m = parse(b"water\n  prog    0101000000\n\n  3  2  0  0  0  0  0  0  0  0999 V2000\n    0.0000    0.0000    0.0000 O   0  0  0  0  0  0  0  0  0  0  0  0\n    0.7570    0.5860    0.0000 H   0  0  0  0  0  0  0  0  0  0  0  0\n   -0.7570    0.5860    0.0000 H   0  0  0  0  0  0  0  0  0  0  0  0\n  1  2  1  0  0  0  0\n  1  3  1  0  0  0  0\nM  END\n").unwrap();
//! assert_eq!(m.atoms.len(), 3);
//! assert_eq!(m.bonds.len(), 2);
//! ```

use std::string::String;
use std::vec::Vec;

/// One atom line.
#[derive(Clone, Debug)]
pub struct Atom {
    /// X coordinate ×10⁶.
    pub x: i64,
    /// Y coordinate ×10⁶.
    pub y: i64,
    /// Z coordinate ×10⁶.
    pub z: i64,
    /// Element symbol.
    pub element: String,
}

/// One bond line (1-based atom indices).
#[derive(Clone, Debug)]
pub struct Bond {
    /// First atom index (1-based).
    pub a1: u32,
    /// Second atom index (1-based).
    pub a2: u32,
    /// Bond type (1 single, 2 double, 3 triple, 4 aromatic, …).
    pub kind: u32,
}

/// A parsed V2000 molfile.
#[derive(Clone, Debug)]
pub struct Mol {
    /// Line 1: molecule name.
    pub name: String,
    /// Line 2: program/timestamp line, verbatim.
    pub program: String,
    /// Line 3: comment line.
    pub comment: String,
    /// Declared `V2000` marker was present.
    pub v2000: bool,
    /// Atom block.
    pub atoms: Vec<Atom>,
    /// Bond block.
    pub bonds: Vec<Bond>,
    /// `M`-property lines between the bond block and `M  END`.
    pub properties: Vec<String>,
}

fn col(line: &str, a: usize, b: usize) -> Option<String> {
    let b = b.min(line.len());
    if a >= b {
        return None;
    }
    Some(line.get(a..b)?.trim().to_string())
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

/// Parse a V2000 `.mol` file. V3000 (`M  V30`) is rejected.
pub fn parse(d: &[u8]) -> Option<Mol> {
    let text = std::str::from_utf8(d).ok()?;
    let mut lines = text.split('\n');
    let name = lines.next()?.trim_end_matches('\r').to_string();
    let program = lines.next()?.trim_end_matches('\r').to_string();
    let comment = lines.next()?.trim_end_matches('\r').to_string();
    let counts = lines.next()?.trim_end_matches('\r').to_string();
    let natoms: usize = col(&counts, 0, 3)?.parse().ok()?;
    let nbonds: usize = col(&counts, 3, 6)?.parse().ok()?;
    if natoms == 0 || natoms > 999 || nbonds > 999 {
        return None;
    }
    let v2000 = counts.contains("V2000");
    if counts.contains("V3000") {
        return None;
    }
    let mut atoms = Vec::new();
    for _ in 0..natoms {
        let l = lines.next()?.trim_end_matches('\r').to_string();
        atoms.push(Atom {
            x: micro(&col(&l, 0, 10)?)?,
            y: micro(&col(&l, 10, 20)?)?,
            z: micro(&col(&l, 20, 30)?)?,
            element: col(&l, 31, 34)?,
        });
    }
    let mut bonds = Vec::new();
    for _ in 0..nbonds {
        let l = lines.next()?.trim_end_matches('\r').to_string();
        bonds.push(Bond {
            a1: col(&l, 0, 3)?.parse().ok()?,
            a2: col(&l, 3, 6)?.parse().ok()?,
            kind: col(&l, 6, 9)?.parse().ok()?,
        });
    }
    let mut properties = Vec::new();
    for l in lines {
        let l = l.trim_end_matches('\r');
        if l == "M  END" {
            return Some(Mol {
                name,
                program,
                comment,
                v2000,
                atoms,
                bonds,
                properties,
            });
        }
        if !l.is_empty() {
            properties.push(l.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const WATER: &[u8] = b"water\n  prog    0101000000\n\n  3  2  0  0  0  0  0  0  0  0999 V2000\n    0.0000    0.0000    0.0000 O   0  0  0  0  0  0  0  0  0  0  0  0\n    0.7570    0.5860    0.0000 H   0  0  0  0  0  0  0  0  0  0  0  0\n   -0.7570    0.5860    0.0000 H   0  0  0  0  0  0  0  0  0  0  0  0\n  1  2  1  0  0  0  0\n  1  3  1  0  0  0  0\nM  CHG  1   1   1\nM  END\n";

    #[test]
    fn parses_water() {
        let m = parse(WATER).unwrap();
        assert_eq!(m.name, "water");
        assert!(m.v2000);
        assert_eq!(m.atoms[0].element, "O");
        assert_eq!(m.atoms[1].x, 757_000);
        assert_eq!(m.atoms[2].x, -757_000);
        assert_eq!(m.bonds[0].a1, 1);
        assert_eq!(m.bonds[1].a2, 3);
        assert_eq!(m.properties, vec!["M  CHG  1   1   1".to_string()]);
    }

    #[test]
    fn rejects_v3000_and_missing_end() {
        assert!(parse(b"x\np\n\n  1  0  0  0  0  0  0  0  0  0999 V3000\n").is_none());
        assert!(parse(b"x\np\n\n  1  0  0  0  0  0  0  0  0  0999 V2000\n    0.0000    0.0000    0.0000 O   0\n").is_none());
        assert!(parse(b"x\np\n\nabc\n").is_none());
    }
}
