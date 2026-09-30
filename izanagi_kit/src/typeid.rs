//! TypeID — `<prefix>_` + 26-char base32 (Crockford, lowercase) suffix
//! encoding a UUIDv7 (e.g. `user_01h455vb4pex5vsknk084sn02q`).
//!
//! ```
//! let d = b"user_01h455vb4pex5vsknk084sn02q";
//! let t = izanagi_kit::typeid::parse(d).unwrap();
//! assert_eq!(t.prefix_len, 4);
//! assert_eq!(t.suffix_len, 26);
//! assert!(t.v7);
//! assert!(izanagi_kit::typeid::detect(d));
//! ```

/// Census of a TypeID string.
#[derive(Debug, Clone)]
pub struct Typeid {
    /// Prefix length in chars.
    pub prefix_len: usize,
    /// Always 26 when valid.
    pub suffix_len: usize,
    /// UUIDv7 signal: first suffix char decodes a `7` in the version nibble.
    pub v7: bool,
    /// Prefix chars are `[a-z]`.
    pub prefix_ok: bool,
    /// Distinct suffix alphabet chars.
    pub distinct: usize,
    /// Variant bits look sane (top bits `10`) when decoded.
    pub variant_ok: bool,
}

fn b32v(c: u8) -> Option<u32> {
    match c {
        b'0'..=b'9' => Some((c - b'0') as u32),
        b'a'..=b'h' => Some((c - b'a' + 10) as u32),
        b'j'..=b'k' => Some((c - b'a' + 9) as u32),
        b'm'..=b'n' => Some((c - b'a' + 8) as u32),
        b'p'..=b't' => Some((c - b'a' + 7) as u32),
        b'v'..=b'z' => Some((c - b'a' + 6) as u32),
        _ => None,
    }
}

fn decode_suffix(s: &[u8]) -> Option<u128> {
    if s.len() != 26 {
        return None;
    }
    let mut v = 0u128;
    for &c in s {
        v = (v << 5) | b32v(c)? as u128;
    }
    Some(v)
}

/// Detects a TypeID: `[a-z]+_` + 26 base32 chars.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Some(u) = b.iter().position(|c| *c == b'_') else {
        return false;
    };
    (1..=63).contains(&u)
        && b[..u].iter().all(|c| c.is_ascii_lowercase())
        && b.len() - u - 1 == 26
        && b[u + 1..].iter().all(|c| b32v(*c).is_some())
}

/// Parses a TypeID; `None` on bad prefix/suffix.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Typeid> {
    if !detect(b) {
        return None;
    }
    let u = b.iter().position(|c| *c == b'_')?;
    let suffix = &b[u + 1..];
    let v = decode_suffix(suffix)?;
    let version = ((v >> 76) & 0xf) as u8; // bits 48-51 of uuid
    let variant = ((v >> 62) & 0x3) as u8; // bits 64-65
    let mut seen = 0u64;
    let mut distinct = 0usize;
    for &c in suffix {
        let i = b32v(c)? as usize;
        if seen & (1 << i) == 0 {
            seen |= 1 << i;
            distinct += 1;
        }
    }
    Some(Typeid {
        prefix_len: u,
        suffix_len: suffix.len(),
        v7: version == 7,
        prefix_ok: true,
        distinct,
        variant_ok: variant == 0b10,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let t = parse(b"user_01h455vb4pex5vsknk084sn02q").unwrap();
        assert_eq!(t.prefix_len, 4);
        assert_eq!(t.suffix_len, 26);
        assert!(t.v7);
        assert!(t.distinct > 10);
    }

    #[test]
    fn prefixes() {
        assert!(parse(b"a_01h455vb4pex5vsknk084sn02q").is_some());
        assert!(parse(b"account_05jv4ttqec2fkr4ehvpgb8c1gd").is_some());
    }

    #[test]
    fn detect_works() {
        assert!(detect(b"user_01h455vb4pex5vsknk084sn02q"));
        assert!(!detect(b"User_01h455vb4pex5vsknk084sn02q"));
        assert!(!detect(b"user_01h455vb4pex5vsknk084sn02"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"user_xyz").is_none());
        assert!(parse(b"_01h455vb4pex5vsknk084sn02q").is_none());
    }
}
