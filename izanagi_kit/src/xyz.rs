//! XYZ molecule files — `count`, comment, `El x y z` rows.
//!
//! Line 1 is the atom count, line 2 a free comment, then one atom per
//! line: element symbol and three coordinates. Coordinates are stored
//! as integer micro-units (×10⁶) so the parser stays float-free.
//!
//! ```
//! use izanagi_kit::xyz::parse;
//!
//! let x = parse(b"3\nwater\nO 0.0 0.0 0.0\nH 0.757 0.586 0.0\nH -0.757 0.586 0.0\n").unwrap();
//! assert_eq!(x.atoms.len(), 3);
//! assert_eq!(x.atoms[0].element, "O");
//! assert_eq!(x.atoms[1].x, 757_000);
//! ```

/// One atom row.
#[derive(Clone, Debug)]
pub struct Atom {
    /// Element symbol.
    pub element: String,
    /// X coordinate ×10⁶.
    pub x: i64,
    /// Y coordinate ×10⁶.
    pub y: i64,
    /// Z coordinate ×10⁶.
    pub z: i64,
}

/// A parsed `.xyz` file.
#[derive(Clone, Debug)]
pub struct Xyz {
    /// The comment line (line 2).
    pub comment: String,
    /// Atom rows; `len` equals the declared count.
    pub atoms: Vec<Atom>,
}

/// `[+-]?d*[.d*]` → micro-units (×10⁶). No exponent notation.
fn micro(s: &str) -> Option<i64> {
    let (neg, s) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    let (int, frac) = match s.split_once('.') {
        Some((a, b)) => (a, b),
        None => (s, ""),
    };
    if int.is_empty() && frac.is_empty() {
        return None;
    }
    if !int.bytes().all(|b| b.is_ascii_digit()) || !frac.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let iv: i64 = if int.is_empty() { 0 } else { int.parse().ok()? };
    let mut fv: i64 = 0;
    let mut scale: i64 = 100_000;
    for &b in frac.as_bytes().iter().take(6) {
        fv += (b - b'0') as i64 * scale;
        scale /= 10;
    }
    if frac.len() > 6 {
        return None;
    }
    let v = iv.checked_mul(1_000_000)?.checked_add(fv)?;
    Some(if neg { -v } else { v })
}

/// Parse an XYZ file. `None` on wrong count, short file, or bad number.
pub fn parse(d: &[u8]) -> Option<Xyz> {
    let text = std::str::from_utf8(d).ok()?;
    let mut lines = text.lines();
    let count: usize = lines.next()?.trim().parse().ok()?;
    let comment = lines.next().unwrap_or("").trim().to_string();
    let mut atoms = Vec::with_capacity(count);
    for line in lines {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        let mut it = t.split_whitespace();
        let element = it.next()?.to_string();
        let x = micro(it.next()?)?;
        let y = micro(it.next()?)?;
        let z = micro(it.next()?)?;
        atoms.push(Atom { element, x, y, z });
    }
    if atoms.len() != count {
        return None;
    }
    Some(Xyz { comment, atoms })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let x = parse(b"2\nc\nH 0 0 0\nO 1.25 -0.5 2\n").unwrap();
        assert_eq!(x.comment, "c");
        assert_eq!(x.atoms[1].x, 1_250_000);
        assert_eq!(x.atoms[1].y, -500_000);
        assert_eq!(x.atoms[0].x, 0);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"2\nc\nH 0 0 0\n").is_none());
        assert!(parse(b"1\nc\nH 0 0 z\n").is_none());

        assert_eq!(micro("1e3"), None);
        assert_eq!(micro(""), None);
        assert_eq!(micro("-"), None);
        assert_eq!(micro("-.5"), Some(-500_000));
    }
}
