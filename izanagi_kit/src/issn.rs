//! ISSN — International Standard Serial Number: 7 digits + check
//! where `sum(d_i * (8 - i))` for `i=0..7` is 0 mod 11; check `X` = 10.
//! Written `NNNN-NNNC`.
//!
//! ```
//! let s = izanagi_kit::issn::parse(b"0317-8471").unwrap();
//! assert_eq!(s.digits, 8);
//! assert_eq!(s.separator, 1);
//! assert!(izanagi_kit::issn::detect(b"0317-8471"));
//! ```

/// A validated ISSN.
#[derive(Debug, Clone)]
pub struct Issn {
    /// Always 8.
    pub digits: usize,
    /// `-` separator present (1 or 0).
    pub separator: usize,
    /// Check digit is `X`.
    pub x_check: bool,
    /// First 7 digits packed as a number.
    pub base: u32,
}

fn clean(b: &[u8]) -> Option<(Vec<u32>, usize, bool)> {
    let mut d = Vec::new();
    let mut seps = 0;
    let mut x = false;
    for &c in b {
        match c {
            b'0'..=b'9' => d.push((c - b'0') as u32),
            b'X' | b'x' if d.len() == 7 => {
                d.push(10);
                x = true;
            }
            b'-' | b' ' => seps += 1,
            _ => return None,
        }
    }
    (d.len() == 8).then_some((d, seps, x))
}

/// Detects an ISSN-shaped string.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    clean(b).is_some()
}

/// Parses and validates an ISSN; `None` on bad checksum.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Issn> {
    let (d, seps, x) = clean(b)?;
    let sum: u32 = d.iter().enumerate().map(|(i, v)| v * (8 - i as u32)).sum();
    if sum % 11 != 0 {
        return None;
    }
    let base = d[..7].iter().fold(0u32, |a, v| a * 10 + v);
    Some(Issn {
        digits: 8,
        separator: seps,
        x_check: x,
        base,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let s = parse(b"0317-8471").unwrap();
        assert_eq!(s.digits, 8);
        assert_eq!(s.separator, 1);
        assert_eq!(s.base, 317847);
        assert!(!s.x_check);
    }

    #[test]
    fn parses_x() {
        let s = parse(b"0146-485X").unwrap();
        assert!(s.x_check);
    }

    #[test]
    fn detect_works() {
        assert!(detect(b"0317-8471"));
        assert!(detect(b"03178471"));
        assert!(!detect(b"0317-84"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"0317-8472").is_none());
        assert!(parse(b"0317847X").is_none());
    }
}
