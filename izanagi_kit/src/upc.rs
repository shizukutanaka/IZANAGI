//! UPC-A — Universal Product Code: 11 digits + mod-10 check where
//! odd positions (1st, 3rd, …) weight 3 and even weight 1.
//!
//! ```
//! let u = izanagi_kit::upc::parse(b"036000291452").unwrap();
//! assert_eq!(u.digits, 12);
//! assert_eq!(u.system, 0);
//! assert!(izanagi_kit::upc::detect(b"036000291452"));
//! ```

/// A validated UPC-A.
#[derive(Debug, Clone)]
pub struct Upc {
    /// Always 12.
    pub digits: usize,
    /// Number system digit (first digit).
    pub system: u32,
    /// Computed check digit.
    pub check: u32,
}

fn clean(b: &[u8]) -> Option<Vec<u32>> {
    let mut d = Vec::new();
    for &c in b {
        match c {
            b'0'..=b'9' => d.push((c - b'0') as u32),
            b'-' | b' ' => {}
            _ => return None,
        }
    }
    (d.len() == 12).then_some(d)
}

/// Detects a UPC-A-shaped string (12 digits).
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    clean(b).is_some()
}

/// Parses and validates a UPC-A; `None` on bad checksum.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Upc> {
    let d = clean(b)?;
    let sum: u32 = d[..11]
        .iter()
        .enumerate()
        .map(|(i, v)| v * (if i % 2 == 0 { 3 } else { 1 }))
        .sum();
    let check = (10 - sum % 10) % 10;
    if check != d[11] {
        return None;
    }
    Some(Upc {
        digits: 12,
        system: d[0],
        check,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let u = parse(b"036000291452").unwrap();
        assert_eq!(u.system, 0);
        assert_eq!(u.check, 2);
    }

    #[test]
    fn parses_other() {
        let u = parse(b"042100005264").unwrap();
        assert_eq!(u.check, 4);
    }

    #[test]
    fn detect_works() {
        assert!(detect(b"036000291452"));
        assert!(!detect(b"03600029145"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"036000291453").is_none());
        assert!(parse(b"12345678901").is_none());
    }
}
