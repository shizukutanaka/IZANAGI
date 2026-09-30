//! CUID v2 — lowercase alphanumeric collision-resistant ID
//! (`a`–`z` initial letter + 2–23 base-36 chars, e.g. `tz4a98xxat96iws9zmbrgj3a`).
//!
//! ```
//! let d = b"tz4a98xxat96iws9zmbrgj3a";
//! let c = izanagi_kit::cuid::parse(d).unwrap();
//! assert_eq!(c.len, 24);
//! assert!(c.v2);
//! assert!(izanagi_kit::cuid::detect(d));
//! ```

/// Census of a candidate CUID.
#[derive(Debug, Clone)]
pub struct Cuid {
    /// Character count.
    pub len: usize,
    /// v2 form (starts with a letter, all lowercase base36).
    pub v2: bool,
    /// Legacy `c…`/`ck`/`cl`/`cm`/`cn` prefix.
    pub legacy_prefix: bool,
    /// Base-36 digits `0–9` seen.
    pub digits: usize,
    /// Base-36 letters `a–z` seen.
    pub letters: usize,
    /// Distinct characters.
    pub distinct: usize,
    /// Entropy proxy: distinct/len ratio × 100.
    pub density: usize,
}

fn ok(c: u8) -> bool {
    c.is_ascii_lowercase() || c.is_ascii_digit()
}

/// Detects a CUID: 12–64 lowercase base36 chars starting with a letter.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    (12..=64).contains(&b.len())
        && b.iter().all(|c| ok(*c))
        && b[0].is_ascii_lowercase()
        && b.iter().any(|c| c.is_ascii_digit())
        && b.iter().any(|c| c.is_ascii_lowercase())
}

/// Parses a candidate CUID; `None` on invalid shape.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Cuid> {
    if !detect(b) {
        return None;
    }
    let mut mask = 0u64;
    let mut distinct = 0usize;
    for &c in b {
        let i = if c.is_ascii_digit() {
            (c - b'0') as usize
        } else {
            (c - b'a' + 10) as usize
        };
        if mask & (1 << i) == 0 {
            mask |= 1 << i;
            distinct += 1;
        }
    }
    Some(Cuid {
        len: b.len(),
        v2: b[0].is_ascii_lowercase(),
        legacy_prefix: b[0] == b'c' && b.get(1).is_some_and(|c| c.is_ascii_lowercase()),
        digits: b.iter().filter(|c| c.is_ascii_digit()).count(),
        letters: b.iter().filter(|c| c.is_ascii_lowercase()).count(),
        distinct,
        density: distinct * 100 / b.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let c = parse(b"tz4a98xxat96iws9zmbrgj3a").unwrap();
        assert_eq!(c.len, 24);
        assert!(c.v2);
        assert!(c.digits > 0 && c.letters > 0);
        assert!(c.density > 40);
    }

    #[test]
    fn legacy() {
        let c = parse(b"ckold9d5j0000qz7l0afs5sq1").unwrap();
        assert!(c.legacy_prefix);
    }

    #[test]
    fn detect_works() {
        assert!(detect(b"tz4a98xxat96iws9zmbrgj3a"));
        assert!(!detect(b"UPPER98xxat96iws9zmbrgj3a"));
        assert!(!detect(b"short"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"0z4a98xxat96iws9zmbrgj3a").is_none());
        assert!(parse(b"aaaaaaaaaaaaaaaaaaaaa").is_none());
    }
}
