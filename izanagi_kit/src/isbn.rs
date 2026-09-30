//! ISBN — International Standard Book Number. ISBN-10: 9 digits +
//! check where `sum(d_i * i)` for `i=1..=10` is 0 mod 11 (check may be
//! `X`=10). ISBN-13: `978`/`979` + 12 digits with EAN mod-10 check
//! (weights 1,3 alternating).
//!
//! ```
//! let b = izanagi_kit::isbn::parse(b"978-0-306-40615-7").unwrap();
//! assert!(b.isbn13);
//! assert_eq!(b.digits, 13);
//! assert_eq!(b.prefix, Some(978));
//! assert!(izanagi_kit::isbn::detect(b"0-306-40615-7"));
//! ```

/// A validated ISBN.
#[derive(Debug, Clone)]
pub struct Isbn {
    /// True for ISBN-13 (`978`/`979` EAN), false for ISBN-10.
    pub isbn13: bool,
    /// Digit count (10 or 13).
    pub digits: usize,
    /// GS1 prefix for ISBN-13 (978 or 979).
    pub prefix: Option<u32>,
    /// Number of `-` separators (after cleaning).
    pub separators: usize,
    /// Whether the string contained `X`/`x` as check digit.
    pub x_check: bool,
}

fn digits(b: &[u8]) -> (Vec<u32>, usize, bool) {
    let mut d = Vec::new();
    let mut seps = 0;
    let mut x = false;
    for &c in b {
        match c {
            b'0'..=b'9' => d.push((c - b'0') as u32),
            b'X' | b'x' => {
                d.push(10);
                x = true;
            }
            b'-' | b' ' => seps += 1,
            _ => return (Vec::new(), seps, x),
        }
    }
    (d, seps, x)
}

fn check10(d: &[u32]) -> bool {
    d.len() == 10
        && d.iter()
            .enumerate()
            .map(|(i, v)| v * (i as u32 + 1))
            .sum::<u32>()
            % 11
            == 0
}

fn check13(d: &[u32]) -> bool {
    d.len() == 13
        && d.iter()
            .enumerate()
            .map(|(i, v)| v * (if i % 2 == 0 { 1 } else { 3 }))
            .sum::<u32>()
            % 10
            == 0
}

/// Detects an ISBN-shaped string (10 or 13 digits, possibly separated).
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let (d, _, _) = digits(b);
    d.len() == 10 || (d.len() == 13 && (d[0] == 9 && d[1] == 7 && (d[2] == 8 || d[2] == 9)))
}

/// Parses and validates an ISBN; `None` on bad length or checksum.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Isbn> {
    let (d, seps, x) = digits(b);
    match d.len() {
        10 if check10(&d) => Some(Isbn {
            isbn13: false,
            digits: 10,
            prefix: None,
            separators: seps,
            x_check: x,
        }),
        13 if check13(&d) && d[0] == 9 && d[1] == 7 && (d[2] == 8 || d[2] == 9) => Some(Isbn {
            isbn13: true,
            digits: 13,
            prefix: Some(978 + (d[2] - 8)), // 978 or 979
            separators: seps,
            x_check: x,
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_13() {
        let i = parse(b"978-0-306-40615-7").unwrap();
        assert!(i.isbn13);
        assert_eq!(i.prefix, Some(978));
        assert_eq!(i.separators, 4);
        assert!(!i.x_check);
    }

    #[test]
    fn parses_10() {
        let i = parse(b"0-306-40615-2").unwrap();
        assert!(!i.isbn13);
        assert_eq!(i.digits, 10);
        assert_eq!(i.prefix, None);
    }

    #[test]
    fn parses_x() {
        let i = parse(b"0-8044-2957-X").unwrap();
        assert!(i.x_check);
    }

    #[test]
    fn detect_works() {
        assert!(detect(b"9780306406157"));
        assert!(detect(b"0306406152"));
        assert!(!detect(b"123"));
        assert!(!detect(b"hello"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"978-0-306-40615-8").is_none()); // bad check
        assert!(parse(b"0-306-40615-3").is_none()); // bad 10 check
        assert!(parse(b"1234567890123").is_none()); // no 978/979
    }
}
