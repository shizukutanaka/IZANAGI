//! ORCID iD — `0000-0002-1825-0097`: 15 digits + check where the
//! mod-11 algorithm (ISO 7064) weights each digit `2×`, tracking a
//! running total; check `X` = 10.
//!
//! ```
//! let o = izanagi_kit::orcid::parse(b"0000-0002-1825-0097").unwrap();
//! assert_eq!(o.digits, 16);
//! assert_eq!(o.group_separator, 3);
//! assert!(izanagi_kit::orcid::detect(b"0000-0002-1825-0097"));
//! ```

/// A validated ORCID iD.
#[derive(Debug, Clone)]
pub struct Orcid {
    /// Always 16.
    pub digits: usize,
    /// `-` separator count (canonical is 3).
    pub group_separator: usize,
    /// Check digit is `X`.
    pub x_check: bool,
    /// First 15 digits packed.
    pub base: u64,
}

fn clean(b: &[u8]) -> Option<(Vec<u32>, usize, bool)> {
    let mut d = Vec::new();
    let mut seps = 0;
    let mut x = false;
    for &c in b {
        match c {
            b'0'..=b'9' => d.push((c - b'0') as u32),
            b'X' | b'x' if d.len() == 15 => {
                d.push(10);
                x = true;
            }
            b'-' | b' ' => seps += 1,
            _ => return None,
        }
    }
    (d.len() == 16).then_some((d, seps, x))
}

/// Detects an ORCID-shaped string (15 digits + digit/X).
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    clean(b).is_some()
}

/// Parses and validates an ORCID; `None` on bad checksum.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Orcid> {
    let (d, seps, x) = clean(b)?;
    let mut total = 0u32;
    for v in &d[..15] {
        total = (total + v) * 2;
    }
    let check = (12 - total % 11) % 11;
    let check = if check == 10 { 10 } else { check % 10 };
    if check != d[15] {
        return None;
    }
    let base = d[..15].iter().fold(0u64, |a, v| a * 10 + u64::from(*v));
    Some(Orcid {
        digits: 16,
        group_separator: seps,
        x_check: x,
        base,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let o = parse(b"0000-0002-1825-0097").unwrap();
        assert_eq!(o.digits, 16);
        assert_eq!(o.group_separator, 3);
        assert_eq!(o.base, 21825009);
        assert!(!o.x_check);
    }

    #[test]
    fn parses_x() {
        let o = parse(b"0000-0002-1694-233X").unwrap();
        assert!(o.x_check);
    }

    #[test]
    fn detect_works() {
        assert!(detect(b"0000-0002-1825-0097"));
        assert!(detect(b"0000000218250097"));
        assert!(!detect(b"0000-0002-1825-009"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"0000-0002-1825-0098").is_none());
        assert!(parse(b"0000-0002-1694-2337").is_none());
    }
}
