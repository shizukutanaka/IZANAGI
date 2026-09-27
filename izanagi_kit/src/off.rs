//! Minimal reader for OFF (Object File Format, Geomview): `OFF` magic,
//! counts `nv nf ne`, then `nv` vertex lines `x y z` and `nf` face lines
//! `n i j k...`. `#` comments; coordinates are stored as integer
//! micro-units (×10⁶) — no floats.
//!
//! ```
//! use izanagi_kit::off::parse;
//!
//! let o = parse(b"OFF\n3 1 0\n0 0 0\n1 0 0\n0 1 0\n3 0 1 2\n").unwrap();
//! assert_eq!(o.vertices.len(), 3);
//! assert_eq!(o.faces, vec![vec![0, 1, 2]]);
//! ```

/// Parse a decimal literal into ×10⁶ micro-units (`[+-]?int[.frac]`,
/// fraction ≤ 6 digits, no exponents).
fn micro(s: &str) -> Option<i64> {
    let (neg, s) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    let (int, frac) = s.split_once('.').unwrap_or((s, ""));
    if int.is_empty() || !int.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if frac.len() > 6 || !frac.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let int: i64 = int.parse().ok()?;
    let mut f: i64 = 0;
    let mut scale: i64 = 1_000_000;
    for b in frac.bytes() {
        f = f * 10 + (b - b'0') as i64;
        scale /= 10;
    }
    f *= scale;
    let v = int.checked_mul(1_000_000)?.checked_add(f)?;
    Some(if neg { -v } else { v })
}

/// A parsed OFF mesh.
#[derive(Debug)]
pub struct Off {
    /// `(x, y, z)` vertices as integer micro-units (×10⁶).
    pub vertices: Vec<[i64; 3]>,
    /// Faces as index lists into `vertices` (n may be 3, 4, ...).
    pub faces: Vec<Vec<u64>>,
}

/// Parse an OFF file. `None` on wrong magic, bad counts, or a vertex/face
/// count mismatch.
pub fn parse(data: &[u8]) -> Option<Off> {
    let text = std::str::from_utf8(data).ok()?;
    let mut it = text
        .lines()
        .map(|l| l.split('#').next().unwrap_or("").trim())
        .filter(|l| !l.is_empty());
    let magic = it.next()?;
    if magic != "OFF" && magic != "STOFF" {
        return None;
    }
    let counts = it.next()?;
    let mut c = counts.split_whitespace();
    let nv: usize = c.next()?.parse().ok()?;
    let nf: usize = c.next()?.parse().ok()?;
    let _ne: usize = c.next()?.parse().ok()?;
    if c.next().is_some() {
        return None;
    }
    let mut vertices = Vec::with_capacity(nv);
    for _ in 0..nv {
        let l = it.next()?;
        let mut w = l.split_whitespace();
        let x = micro(w.next()?)?;
        let y = micro(w.next()?)?;
        let z = micro(w.next()?)?;
        // STOFF may carry extra colour words — tolerate them
        vertices.push([x, y, z]);
    }
    let mut faces = Vec::with_capacity(nf);
    for _ in 0..nf {
        let l = it.next()?;
        let mut w = l.split_whitespace();
        let n: usize = w.next()?.parse().ok()?;
        if n < 3 {
            return None;
        }
        let mut idx = Vec::with_capacity(n);
        for _ in 0..n {
            let i: u64 = w.next()?.parse().ok()?;
            if i >= nv as u64 {
                return None;
            }
            idx.push(i);
        }
        faces.push(idx);
    }
    Some(Off { vertices, faces })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TET: &[u8] =
        b"OFF\n# comment\n4 4 6\n0 0 0\n1 0 0\n0 1 0\n0 0 1\n3 0 1 2\n3 0 1 3\n3 0 2 3\n3 1 2 3\n";

    #[test]
    fn parses() {
        let o = parse(TET).unwrap();
        assert_eq!(o.vertices.len(), 4);
        assert_eq!(o.vertices[1], [1_000_000, 0, 0]);
        assert_eq!(o.faces.len(), 4);
        assert_eq!(o.faces[0], vec![0, 1, 2]);
    }

    #[test]
    fn micro_units() {
        assert_eq!(micro("1"), Some(1_000_000));
        assert_eq!(micro("-2.5"), Some(-2_500_000));
        assert_eq!(micro("0.000001"), Some(1));
        assert_eq!(micro("0.0000001"), None); // >6 digits
        assert_eq!(micro("x"), None);
        assert_eq!(micro("1e3"), None); // no exponents
    }

    #[test]
    fn rejects() {
        assert!(parse(b"NOFF\n").is_none());
        assert!(parse(b"OFF\n3 1 0\n0 0 0\n").is_none()); // short vertex list
        assert!(parse(b"OFF\n1 1 0\n0 0 0\n3 0 5 2\n").is_none()); // oob index
        assert!(parse(b"OFF\n1 1 0\n0 0 0\n2 0 1\n").is_none()); // edge < 3
    }
}
