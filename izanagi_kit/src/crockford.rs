//! Crockford Base32 — `0–9 A–H J K M N P–T V–Z` digits, optional
//! trailing check symbol `* ~ $ = U` (values 32–36); hyphens are
//! allowed separators and `I L O` alias to `1 1 0`.
//!
//! ```
//! let d = b"51J6RQ-4X";
//! let c = izanagi_kit::crockford::parse(d).unwrap();
//! assert_eq!(c.digits, 8);
//! assert_eq!(c.check_symbol, None);
//! assert!(izanagi_kit::crockford::detect(d));
//! ```

/// Census of a Crockford Base32 string.
#[derive(Debug, Clone)]
pub struct Crockford {
    /// Digit count (hyphens excluded, check symbol excluded).
    pub digits: usize,
    /// Hyphen separators.
    pub hyphens: usize,
    /// `I`/`L`/`O` aliases found (map to `1`/`1`/`0`).
    pub aliases: usize,
    /// Optional check symbol `* ~ $ = U` (value 32–36).
    pub check_symbol: Option<u32>,
    /// The check value verified against digits (mod 37).
    pub check_ok: Option<bool>,
    /// Accumulated value bits needed (`digits * 5`).
    pub value_bits: usize,
    /// Distinct digit chars.
    pub distinct: usize,
}

fn val(c: u8) -> Option<u32> {
    match c {
        b'0'..=b'9' => Some((c - b'0') as u32),
        b'A'..=b'H' | b'a'..=b'h' => Some((c.to_ascii_uppercase() - b'A' + 10) as u32),
        b'J'..=b'K' | b'j'..=b'k' => Some((c.to_ascii_uppercase() - b'A' + 9) as u32),
        b'M'..=b'N' | b'm'..=b'n' => Some((c.to_ascii_uppercase() - b'A' + 8) as u32),
        b'P'..=b'T' | b'p'..=b't' => Some((c.to_ascii_uppercase() - b'A' + 7) as u32),
        b'V'..=b'Z' | b'v'..=b'z' => Some((c.to_ascii_uppercase() - b'A' + 6) as u32),
        b'I' | b'i' | b'L' | b'l' => Some(1),
        b'O' | b'o' => Some(0),
        _ => None,
    }
}

fn check_val(c: u8) -> Option<u32> {
    match c {
        b'*' => Some(32),
        b'~' => Some(33),
        b'$' => Some(34),
        b'=' => Some(35),
        b'U' | b'u' => Some(36),
        _ => None,
    }
}

/// Detects Crockford Base32: ≥1 digit, hyphens internal, optional check.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    if b.is_empty() {
        return false;
    }
    let body = if b.len() > 1 && check_val(*b.last().unwrap_or(&0)).is_some() {
        &b[..b.len() - 1]
    } else {
        b
    };
    let digits = body.iter().filter(|c| val(**c).is_some()).count();
    let hyphens = body.iter().filter(|c| **c == b'-').count();
    digits >= 1 && digits + hyphens == body.len() && hyphens <= digits
}

/// Parses a Crockford string; `None` on invalid chars.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Crockford> {
    if !detect(b) {
        return None;
    }
    let (body, check) = match check_val(*b.last().unwrap_or(&0)) {
        Some(v) if b.len() > 1 => (&b[..b.len() - 1], Some(v)),
        _ => (b, None),
    };
    let mut acc = 0u64;
    let mut digits = 0usize;
    let mut aliases = 0usize;
    let mut hyphens = 0usize;
    let mut seen = 0u64;
    let mut distinct = 0usize;
    for &c in body {
        if c == b'-' {
            hyphens += 1;
            continue;
        }
        let v = val(c)?;
        if matches!(c, b'I' | b'i' | b'L' | b'l' | b'O' | b'o') {
            aliases += 1;
        }
        if seen & (1 << v) == 0 {
            seen |= 1 << v;
            distinct += 1;
        }
        acc = (acc * 32 + v as u64) % 37;
        digits += 1;
    }
    let check_ok = check.map(|cv| acc == cv as u64);
    Some(Crockford {
        digits,
        hyphens,
        aliases,
        check_symbol: check,
        check_ok,
        value_bits: digits * 5,
        distinct,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let c = parse(b"51J6RQ-4X").unwrap();
        assert_eq!(c.digits, 8);
        assert_eq!(c.hyphens, 1);
        assert_eq!(c.check_symbol, None);
    }

    #[test]
    fn checkdigit() {
        // "14": 1*32+4 = 36 ≡ 36 mod 37 → 'U' check symbol verifies
        let c = parse(b"14U").unwrap();
        assert_eq!(c.check_symbol, Some(36));
        assert_eq!(c.check_ok, Some(true));
        let bad = parse(b"15U").unwrap();
        assert_eq!(bad.check_ok, Some(false));
    }

    #[test]
    fn aliases() {
        let c = parse(b"1I1L1O").unwrap();
        assert_eq!(c.aliases, 3);
        assert_eq!(c.digits, 6);
    }

    #[test]
    fn detect_works() {
        assert!(detect(b"51J6RQ-4X"));
        assert!(detect(b"0"));
        assert!(!detect(b"!!!"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
    }
}
