//! Luhn algorithm — mod-10 check over a digit string: from the right,
//! double every second digit and subtract 9 if > 9; total must be
//! 0 mod 10. Used by PAN (credit card) numbers and many ID schemes.
//!
//! ```
//! let l = izanagi_kit::luhn::parse(b"79927398713").unwrap();
//! assert_eq!(l.digits, 11);
//! assert_eq!(l.check, 3);
//! assert!(izanagi_kit::luhn::detect(b"79927398713"));
//! ```

/// A Luhn-validated digit string.
#[derive(Debug, Clone)]
pub struct Luhn {
    /// Digit count.
    pub digits: usize,
    /// The check digit (last digit).
    pub check: u32,
    /// Separator count (spaces/hyphens).
    pub separators: usize,
    /// Computed Luhn sum.
    pub sum: u32,
}

fn clean(b: &[u8]) -> Option<(Vec<u32>, usize)> {
    let mut d = Vec::new();
    let mut seps = 0;
    for &c in b {
        match c {
            b'0'..=b'9' => d.push((c - b'0') as u32),
            b'-' | b' ' => seps += 1,
            _ => return None,
        }
    }
    (d.len() >= 2).then_some((d, seps))
}

fn luhn_sum(d: &[u32]) -> u32 {
    let mut sum = 0;
    for (i, v) in d.iter().rev().enumerate() {
        sum += if i % 2 == 1 {
            let d2 = v * 2;
            if d2 > 9 {
                d2 - 9
            } else {
                d2
            }
        } else {
            *v
        };
    }
    sum
}

/// Detects a digit string of length ≥ 2 (Luhn-able).
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    clean(b).is_some()
}

/// Validates a Luhn number; `None` on non-digit content or bad sum.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Luhn> {
    let (d, seps) = clean(b)?;
    let sum = luhn_sum(&d);
    if sum % 10 != 0 {
        return None;
    }
    Some(Luhn {
        digits: d.len(),
        check: *d.last()?,
        separators: seps,
        sum,
    })
}

/// Computes the Luhn check digit for a digit string without check.
#[must_use]
pub fn check_digit(b: &[u8]) -> Option<u32> {
    let mut d = Vec::new();
    for &c in b {
        match c {
            b'0'..=b'9' => d.push((c - b'0') as u32),
            b'-' | b' ' => {}
            _ => return None,
        }
    }
    if d.is_empty() {
        return None;
    }
    d.push(0);
    let sum = luhn_sum(&d);
    Some((10 - sum % 10) % 10)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let l = parse(b"79927398713").unwrap();
        assert_eq!(l.digits, 11);
        assert_eq!(l.check, 3);
    }

    #[test]
    fn parses_spaced() {
        let l = parse(b"7992 7398 713").unwrap();
        assert_eq!(l.separators, 2);
    }

    #[test]
    fn computes() {
        assert_eq!(check_digit(b"7992739871"), Some(3));
        assert_eq!(check_digit(b"0"), Some(0));
    }

    #[test]
    fn detect_works() {
        assert!(detect(b"42"));
        assert!(detect(b"7992-7398"));
        assert!(!detect(b"1"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"79927398714").is_none());
        assert!(parse(b"abc").is_none());
    }
}
