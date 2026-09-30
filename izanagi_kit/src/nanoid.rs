//! Nano ID — URL-safe identifier over the 64-char alphabet
//! `A–Z a–z 0–9 - _` (default 21 chars); detects/configures length and
//! alphabet subsets (numeric, hex, lowercase).
//!
//! ```
//! let d = b"V1StGXR8_Z5jdHi6B-myT";
//! let n = izanagi_kit::nanoid::parse(d).unwrap();
//! assert_eq!(n.len, 21);
//! assert!(n.default_alphabet);
//! assert!(izanagi_kit::nanoid::detect(d));
//! ```

/// Census of a candidate Nano ID string.
#[derive(Debug, Clone)]
pub struct Nanoid {
    /// Character count.
    pub len: usize,
    /// Every char in the default 64-char alphabet.
    pub default_alphabet: bool,
    /// Every char is `0–9`.
    pub numeric_only: bool,
    /// Every char is lowercase hex `0–9a–f`.
    pub hex_only: bool,
    /// Every char is lowercase `a–z` (+digits).
    pub lower_only: bool,
    /// Has `-`/`_` symbols.
    pub has_symbols: bool,
    /// Distinct characters seen.
    pub distinct: usize,
}

fn in_default(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'-' || c == b'_'
}

/// Detects a Nano ID: 8–64 URL-safe chars, at least one letter.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    (8..=64).contains(&b.len())
        && b.iter().all(|c| in_default(*c))
        && b.iter().any(|c| c.is_ascii_alphanumeric())
}

/// Parses a candidate Nano ID; `None` on invalid chars/length.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Nanoid> {
    if !detect(b) {
        return None;
    }
    let mut seen = 0u64;
    let mut distinct = 0usize;
    for &c in b {
        let i = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            _ => 63,
        } as usize;
        if seen & (1 << i) == 0 {
            seen |= 1 << i;
            distinct += 1;
        }
    }
    Some(Nanoid {
        len: b.len(),
        default_alphabet: true,
        numeric_only: b.iter().all(|c| c.is_ascii_digit()),
        hex_only: b
            .iter()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
        lower_only: b
            .iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()),
        has_symbols: b.iter().any(|c| *c == b'-' || *c == b'_'),
        distinct,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let n = parse(b"V1StGXR8_Z5jdHi6B-myT").unwrap();
        assert_eq!(n.len, 21);
        assert!(n.has_symbols);
        assert!(!n.numeric_only);
        assert!(!n.hex_only);
        assert!(!n.lower_only);
        assert!(n.distinct > 12);
    }

    #[test]
    fn subsets() {
        assert!(parse(b"0123456789a").unwrap().hex_only);
        assert!(parse(b"abcdef012345").unwrap().lower_only);
        assert!(parse(b"Ab1Cd2Ef3Gh4").unwrap().default_alphabet);
    }

    #[test]
    fn detect_works() {
        assert!(detect(b"V1StGXR8_Z5jdHi6B-myT"));
        assert!(!detect(b"short"));
        assert!(!detect(b"with space in it"));
        assert!(detect(b"alllowercaseletters"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"bad+chars/here1").is_none());
    }
}
