//! W3C DID — `did:<method>:<method-specific-id>` with an optional
//! `;` parameters and `#fragment`/`?query`/`/path` suffixes; the MSID
//! charset is `A–Z a–z 0–9 . _ -` and percent-encoded bytes.
//!
//! ```
//! let d = b"did:example:123456789abcdefghi";
//! let t = izanagi_kit::did::parse(d).unwrap();
//! assert_eq!(t.method_len, 7);
//! assert_eq!(t.msid_len, 18);
//! assert!(izanagi_kit::did::detect(d));
//! ```

/// Census of a DID string.
#[derive(Debug, Clone)]
pub struct Did {
    /// `did` scheme present.
    pub scheme: bool,
    /// Method name length.
    pub method_len: usize,
    /// Method-specific-id length (before `;`/`?`/`/`/`#`).
    pub msid_len: usize,
    /// `:` sub-segments inside the MSID.
    pub subids: usize,
    /// `%xx` percent-encoded bytes.
    pub percent_encoded: usize,
    /// `;` parameters.
    pub params: usize,
    /// `?` query present.
    pub query: bool,
    /// `#` fragment present.
    pub fragment: bool,
    /// `/` path segments after MSID.
    pub paths: usize,
    /// `+` service aliases (did:...;service=).
    pub service_hints: usize,
    /// Known method (`web`/`key`/`ion`/`ethr`/`plc`/`peer`/`jwk`/`dns`).
    pub known_method: bool,
}

const KNOWN: &[&str] = &[
    "web", "key", "ion", "ethr", "plc", "peer", "jwk", "dns", "keri", "cheqd",
];

/// Detects a DID: `did:` + method + `:` + non-empty MSID.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let Some(rest) = t.strip_prefix("did:") else {
        return false;
    };
    let Some(c) = rest.find(':') else {
        return false;
    };
    c >= 1
        && rest[..c]
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && rest[c + 1..]
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric())
}

/// Parses a DID; `None` on bad scheme/method/MSID.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Did> {
    let t = std::str::from_utf8(b).ok()?;
    if !detect(b) {
        return None;
    }
    let rest = &t[4..];
    let c = rest.find(':')?;
    let method = &rest[..c];
    let tail = &rest[c + 1..];
    let end = tail.find([';', '?', '/', '#']).unwrap_or(tail.len());
    let msid = &tail[..end];
    let after = &tail[end..];
    Some(Did {
        scheme: true,
        method_len: method.len(),
        msid_len: msid.len(),
        subids: msid.matches(':').count(),
        percent_encoded: t.matches('%').count(),
        params: after.matches(';').count(),
        query: after.contains('?'),
        fragment: after.contains('#'),
        paths: after.matches('/').count(),
        service_hints: after.matches("service").count(),
        known_method: KNOWN.contains(&method),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let t = parse(b"did:example:123456789abcdefghi").unwrap();
        assert_eq!(t.method_len, 7);
        assert_eq!(t.msid_len, 18);
        assert!(!t.known_method);
    }

    #[test]
    fn full() {
        let t = parse(b"did:web:example.com:user:alice/path?query#frag").unwrap();
        assert!(t.known_method);
        assert_eq!(t.subids, 2);
        assert!(t.query && t.fragment);
        assert_eq!(t.paths, 1);
    }

    #[test]
    fn percent() {
        let t = parse(b"did:key:z6MkhaXgBZDvotDkL5257faiz%20x").unwrap();
        assert_eq!(t.percent_encoded, 1);
        assert_eq!(t.method_len, 3);
        assert!(t.known_method);
    }

    #[test]
    fn detect_works() {
        assert!(detect(b"did:example:abc"));
        assert!(!detect(b"did:EXAMPLE:abc"));
        assert!(!detect(b"did:example"));
        assert!(!detect(b"did:example:"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"not:a:did").is_none());
    }
}
