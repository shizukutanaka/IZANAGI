//! ISMN — International Standard Music Number: `979-0` + 9 digits
//! with mod-10 check (weights 1,3), historically `M` + 9 chars with
//! letter values (A=10,…,Z=35).
//!
//! ```
//! let s = izanagi_kit::ismn::parse(b"979-0-060-11561-5").unwrap();
//! assert!(s.modern);
//! assert_eq!(s.digits, 13);
//! assert!(izanagi_kit::ismn::detect(b"979-0-060-11561-5"));
//! ```

/// A validated ISMN.
#[derive(Debug, Clone)]
pub struct Ismn {
    /// True for `979-0` form, false for legacy `M-` form.
    pub modern: bool,
    /// Digit count (13 modern / 9+check legacy = 10 chars).
    pub digits: usize,
    /// Separator count.
    pub separators: usize,
    /// Base number (first digits before check).
    pub base: u64,
}

fn wsum(d: &[u32]) -> u32 {
    d.iter()
        .enumerate()
        .map(|(i, v)| v * (if i % 2 == 0 { 1 } else { 3 }))
        .sum()
}

fn check10w(d: &[u32]) -> bool {
    wsum(d) % 10 == 0
}

/// Detects an ISMN-shaped string (`979-0…` or `M-…`).
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 4 && (b.starts_with(b"M") || b.starts_with(b"979"))
}

/// Parses and validates an ISMN; `None` on bad form or checksum.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Ismn> {
    let t = std::str::from_utf8(b).ok()?;
    if t.starts_with('M') || t.starts_with('m') {
        // Legacy: M + 9 chars (digits or letters valued 10..=35) — mod 10 w/ 1,3.
        let mut d = Vec::new();
        let mut seps = 0;
        for c in t[1..].chars() {
            match c {
                '0'..='9' => d.push(c as u32 - '0' as u32),
                'A'..='Z' => d.push(c as u32 - 'A' as u32 + 10),
                '-' | ' ' => seps += 1,
                _ => return None,
            }
        }
        // M contributes value 3 at weight 3 → +9 to the weighted sum.
        if d.len() != 9 || (wsum(&d) + 9) % 10 != 0 {
            return None;
        }
        let base = d[..8].iter().fold(0u64, |a, v| a * 36 + u64::from(*v));
        return Some(Ismn {
            modern: false,
            digits: 9,
            separators: seps,
            base,
        });
    }
    // Modern: 979-0 + 9 digits, total 13 digits.
    let mut d = Vec::new();
    let mut seps = 0;
    for c in t.chars() {
        match c {
            '0'..='9' => d.push(c as u32 - '0' as u32),
            '-' | ' ' => seps += 1,
            _ => return None,
        }
    }
    if d.len() != 13 || d[0] != 9 || d[1] != 7 || d[2] != 9 || d[3] != 0 || !check10w(&d) {
        return None;
    }
    let base = d[..12].iter().fold(0u64, |a, v| a * 10 + u64::from(*v));
    Some(Ismn {
        modern: true,
        digits: 13,
        separators: seps,
        base,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_modern() {
        let s = parse(b"979-0-060-11561-5").unwrap();
        assert!(s.modern);
        assert_eq!(s.digits, 13);
        assert_eq!(s.separators, 4);
    }

    #[test]
    fn parses_legacy() {
        let s = parse(b"M-2306-7118-7").unwrap();
        assert!(!s.modern);
        assert_eq!(s.digits, 9);
    }

    #[test]
    fn detect_works() {
        assert!(detect(b"979-0-060-11561-5"));
        assert!(detect(b"M-2306-7118-7"));
        assert!(!detect(b"978-0-306"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"979-0-060-11561-6").is_none());
        assert!(parse(b"M-2306-7118-8").is_none());
    }
}
