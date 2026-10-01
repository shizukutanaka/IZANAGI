//! EAN-13 — European Article Number: 12 digits + mod-10 check with
//! weights 1,3 (odd positions 1,3,5,… weighted 1 in ISO numbering).
//!
//! ```
//! let e = izanagi_kit::ean::parse(b"5901234123457").unwrap();
//! assert_eq!(e.digits, 13);
//! assert_eq!(e.prefix, 590);
//! assert!(izanagi_kit::ean::detect(b"5901234123457"));
//! ```

/// A validated EAN-13.
#[derive(Debug, Clone)]
pub struct Ean {
    /// Always 13.
    pub digits: usize,
    /// First 3 digits (GS1 prefix, e.g. 490–499 for Japan).
    pub prefix: u32,
    /// Computed check digit value.
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
    (d.len() == 13).then_some(d)
}

/// Detects an EAN-13-shaped string (13 digits).
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    clean(b).is_some()
}

/// Parses and validates an EAN-13; `None` on bad checksum.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Ean> {
    let d = clean(b)?;
    let sum: u32 = d[..12]
        .iter()
        .enumerate()
        .map(|(i, v)| v * (if i % 2 == 0 { 1 } else { 3 }))
        .sum();
    let check = (10 - sum % 10) % 10;
    if check != d[12] {
        return None;
    }
    Some(Ean {
        digits: 13,
        prefix: d[0] * 100 + d[1] * 10 + d[2],
        check,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let e = parse(b"5901234123457").unwrap();
        assert_eq!(e.prefix, 590);
        assert_eq!(e.check, 7);
    }

    #[test]
    fn parses_japan() {
        let e = parse(b"4901234567894").unwrap();
        assert_eq!(e.prefix, 490);
    }

    #[test]
    fn detect_works() {
        assert!(detect(b"5901234123457"));
        assert!(detect(b"9780306406157"));
        assert!(!detect(b"12345"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"5901234123458").is_none());
        assert!(parse(b"59012341234").is_none());
    }
}
