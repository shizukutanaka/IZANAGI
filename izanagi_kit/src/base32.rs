//! RFC 4648 Base32 — `A–Z2–7` alphabet with `=` padding, optional
//! Base32hex `0–9A–V` variant; validates group-of-8 padding rules.
//!
//! ```
//! let d = b"MFRGGZDFMZTWQ2LKNNWG23TPOBYXE43U";
//! let x = izanagi_kit::base32::parse(d).unwrap();
//! assert_eq!(x.len, 32);
//! assert_eq!(x.hex_variant, Some(false));
//! assert!(izanagi_kit::base32::detect(d));
//! ```

/// Census of a Base32 string.
#[derive(Debug, Clone)]
pub struct Base32 {
    /// Total chars including padding.
    pub len: usize,
    /// Chars excluding `=` padding and whitespace.
    pub data_len: usize,
    /// `=` padding chars.
    pub padding: usize,
    /// Standard `A–Z2–7` vs hex `0–9A–V` when detectable (`Some`).
    pub hex_variant: Option<bool>,
    /// Padding length is legal (`0`/`1`/`3`/`4`/`6`).
    pub padding_ok: bool,
    /// Lowercase input (RFC allows decoding lowercase).
    pub lowercase: bool,
    /// Distinct alphabet chars.
    pub distinct: usize,
}

fn std32(c: u8) -> bool {
    c.is_ascii_uppercase() || (b'2'..=b'7').contains(&c)
}

fn hex32(c: u8) -> bool {
    c.is_ascii_digit() || (b'A'..=b'V').contains(&c)
}

/// Detects Base32: ≥8 alphabet chars, `=` only trailing, padding legal.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t: Vec<u8> = b
        .iter()
        .copied()
        .filter(|c| !c.is_ascii_whitespace())
        .collect();
    if t.len() < 8 {
        return false;
    }
    let pad = t.iter().rev().take_while(|c| **c == b'=').count();
    if ![0, 1, 3, 4, 6].contains(&pad) {
        return false;
    }
    let data = &t[..t.len() - pad];
    let length_ok = pad == 0 || t.len() % 8 == 0;
    data.len() >= 8
        && length_ok
        && data
            .iter()
            .all(|c| std32(c.to_ascii_uppercase()) || hex32(c.to_ascii_uppercase()))
}

/// Parses a Base32 string; `None` on bad alphabet/padding.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Base32> {
    if !detect(b) {
        return None;
    }
    let t: Vec<u8> = b
        .iter()
        .copied()
        .filter(|c| !c.is_ascii_whitespace())
        .collect();
    let padding = t.iter().rev().take_while(|c| **c == b'=').count();
    let data = &t[..t.len() - padding];
    let mut seen = 0u64;
    let mut distinct = 0usize;
    let mut all_hex = true;
    for &c in data {
        let u = c.to_ascii_uppercase();
        if !hex32(u) {
            all_hex = false;
        }
        let i = match u {
            b'A'..=b'Z' => (u - b'A') as usize,
            b'0'..=b'9' => (u - b'0' + 26) as usize,
            _ => 63,
        };
        if seen & (1 << i) == 0 {
            seen |= 1 << i;
            distinct += 1;
        }
    }
    Some(Base32 {
        len: t.len(),
        data_len: data.len(),
        padding,
        hex_variant: Some(all_hex),
        padding_ok: true,
        lowercase: data.iter().any(|c| c.is_ascii_lowercase()),
        distinct,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let x = parse(b"MFRGGZDFMZTWQ2LKNNWG23TPOBYXE43U").unwrap();
        assert_eq!(x.len, 32);
        assert_eq!(x.padding, 0);
        assert!(x.padding_ok);
        assert!(!x.lowercase);
        assert_eq!(x.hex_variant, Some(false));
    }

    #[test]
    fn padded() {
        let x = parse(b"MZXW6YTBOI======").unwrap();
        assert_eq!(x.padding, 6);
        assert!(x.padding_ok);
        assert_eq!(x.data_len, 10);
    }

    #[test]
    fn hexform() {
        let x = parse(b"0123456789ABCDEFGH").unwrap();
        assert_eq!(x.hex_variant, Some(true));
    }

    #[test]
    fn detect_works() {
        assert!(detect(b"MFRGGZDFMZTWQ2LK"));
        assert!(detect(b"MZXW6YTBOI======"));
        assert!(!detect(b"MFRGG==="));
        assert!(!detect(b"short"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"MZXW6YTBOI=======").is_none());
    }
}
