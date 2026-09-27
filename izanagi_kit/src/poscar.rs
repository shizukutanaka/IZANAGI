//! VASP `POSCAR`/`CONTCAR` — comment line, lattice scale factor, three
//! lattice vectors, element symbols (VASP 5) and per-element counts,
//! optional `Selective dynamics`, then `Direct`/`Cartesian` positions.
//!
//! All numbers are ×10⁶ micro-units; negative scale factors (volume
//! semantics) are preserved as negative micro values.
//!
//! ```
//! use izanagi_kit::poscar::parse;
//!
//! let p = parse(b"Si\n  1.0\n  5.43 0.0 0.0\n  0.0 5.43 0.0\n  0.0 0.0 5.43\n  Si\n  2\nDirect\n  0.0 0.0 0.0\n  0.25 0.25 0.25\n").unwrap();
//! assert_eq!(p.symbols, vec!["Si".to_string()]);
//! assert_eq!(p.counts, vec![2]);
//! assert!(p.direct);
//! assert_eq!(p.positions.len(), 2);
//! ```

use std::string::String;
use std::vec::Vec;

/// A parsed POSCAR.
#[derive(Clone, Debug)]
pub struct Poscar {
    /// Comment line.
    pub comment: String,
    /// Scale factor ×10⁶ (may be negative = target volume).
    pub scale: i64,
    /// Lattice rows ×10⁶.
    pub lattice: [[i64; 3]; 3],
    /// Element symbols per group (empty on VASP 4 files).
    pub symbols: Vec<String>,
    /// Atom count per group.
    pub counts: Vec<u32>,
    /// `Selective dynamics` line was present.
    pub selective: bool,
    /// Positions are direct (fractional) coordinates, not Cartesian.
    pub direct: bool,
    /// Position rows ×10⁶ (with optional `T/F` flags ignored).
    pub positions: Vec<[i64; 3]>,
}

fn num(s: &str) -> Option<i64> {
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

fn vec3(l: &str) -> Option<[i64; 3]> {
    let mut it = l.split_whitespace();
    Some([num(it.next()?)?, num(it.next()?)?, num(it.next()?)?])
}

fn all_digits(l: &str) -> bool {
    let mut it = l.split_whitespace().peekable();
    if it.peek().is_none() {
        return false;
    }
    it.all(|t| t.bytes().all(|c| c.is_ascii_digit()))
}

/// Parse a `POSCAR`/`CONTCAR` (VASP 4 and 5 layouts).
pub fn parse(d: &[u8]) -> Option<Poscar> {
    let text = std::str::from_utf8(d).ok()?;
    let mut lines = text.split('\n');
    let comment = lines.next()?.trim_end_matches('\r').to_string();
    let scale = num(lines.next()?.trim_end_matches('\r').trim())?;
    let mut lattice = [[0i64; 3]; 3];
    for r in lattice.iter_mut() {
        *r = vec3(lines.next()?.trim_end_matches('\r'))?;
    }
    let l6 = lines.next()?.trim_end_matches('\r').to_string();
    let (symbols, counts): (Vec<String>, Vec<u32>) = if all_digits(&l6) {
        // VASP 4: counts directly on line 6
        (
            Vec::new(),
            l6.split_whitespace()
                .map(|t| t.parse().ok())
                .collect::<Option<Vec<u32>>>()?,
        )
    } else {
        let syms: Vec<String> = l6.split_whitespace().map(|t| t.to_string()).collect();
        if syms.is_empty() {
            return None;
        }
        let l7 = lines.next()?.trim_end_matches('\r').to_string();
        let cs: Vec<u32> = l7
            .split_whitespace()
            .map(|t| t.parse().ok())
            .collect::<Option<Vec<u32>>>()?;
        if cs.len() != syms.len() {
            return None;
        }
        (syms, cs)
    };
    if counts.is_empty() || counts.contains(&0) {
        return None;
    }
    let mut l = lines.next()?.trim_end_matches('\r').to_string();
    let selective = matches!(l.chars().next(), Some('s') | Some('S'));
    if selective {
        l = lines.next()?.trim_end_matches('\r').to_string();
    }
    let direct = match l.chars().next() {
        Some('d') | Some('D') => true,
        Some('c') | Some('C') | Some('k') | Some('K') => false,
        _ => return None,
    };
    let total: usize = counts.iter().map(|c| *c as usize).sum();
    let mut positions = Vec::new();
    for _ in 0..total {
        positions.push(vec3(lines.next()?.trim_end_matches('\r'))?);
    }
    Some(Poscar {
        comment,
        scale,
        lattice,
        symbols,
        counts,
        selective,
        direct,
        positions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vasp5_direct() {
        let p = parse(b"Si\n  1.0\n  5.43 0.0 0.0\n  0.0 5.43 0.0\n  0.0 0.0 5.43\n  Si\n  2\nDirect\n  0.0 0.0 0.0\n  0.25 0.25 0.25\n").unwrap();
        assert_eq!(p.lattice[0][0], 5_430_000);
        assert_eq!(p.positions[1], [250_000, 250_000, 250_000]);
        assert!(p.direct && !p.selective);
    }

    #[test]
    fn vasp4_cartesian_selective() {
        let p = parse(b"x\n  -15.0\n  1 0 0\n  0 1 0\n  0 0 1\n  1\nSelective dynamics\nCartesian\n 0.5 0.5 0.5 T T T\n").unwrap();
        assert!(p.symbols.is_empty());
        assert_eq!(p.scale, -15_000_000);
        assert!(p.selective && !p.direct);
        assert_eq!(p.positions.len(), 1);
    }

    #[test]
    fn rejects_mismatch() {
        assert!(parse(b"x\n1\n1 0 0\n0 1 0\n0 0 1\nSi O\n2\nDirect\n0 0 0\n0 0 0\n").is_none());
        assert!(parse(b"x\n1\n1 0 0\n0 1 0\n0 0 1\nSi\n0\nDirect\n").is_none());
    }
}
