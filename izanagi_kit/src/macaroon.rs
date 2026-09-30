//! Macaroon token — a `location`/`identifier`/`cid` caveat chain with
//! a trailing `signature` (v2 JSON `{"v":2,"l":…,"i":…,"s64":…}` or the
//! legacy packet text).
//!
//! ```
//! let d = b"{\"v\":2,\"l\":\"loc\",\"i\":\"id\",\"c\":[],\"s64\":\"sig\"}";
//! let m = izanagi_kit::macaroon::parse(d).unwrap();
//! assert_eq!(m.v2, true);
//! assert_eq!(m.version, Some(1));
//! assert!(m.signed);
//! assert!(izanagi_kit::macaroon::detect(d));
//! ```

/// Census of a Macaroon token.
#[derive(Debug, Clone)]
pub struct Macaroon {
    /// v2 JSON form (`"v":2`) vs legacy packet text.
    pub v2: bool,
    /// `"v"` version field length.
    pub version: Option<usize>,
    /// `location`/`l` fields.
    pub locations: usize,
    /// `identifier`/`i` fields.
    pub identifiers: usize,
    /// Caveat `cid` fields.
    pub cids: usize,
    /// Verification-key `vid` caveats.
    pub vids: usize,
    /// `signature`/`s`/`s64` fields.
    pub signed: bool,
    /// Total `"{` object starts (rough caveat count proxy).
    pub objects: usize,
    /// `signature` hex string present.
    pub hex_sig: bool,
}

fn count_key(t: &str, key: &str) -> usize {
    let needle = format!("\"{key}\"");
    t.matches(&needle).count()
}

/// Detects a Macaroon: `location`+`identifier`/`i`+signature-ish fields.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("location") || t.contains("\"l\""))
        && (t.contains("identifier") || t.contains("\"i\"") || t.contains("cid"))
        && (t.contains("signature") || t.contains("\"s64\"") || t.contains("\"s\""))
}

/// Parses a Macaroon; `None` on non-UTF-8 or missing markers.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Macaroon> {
    let t = std::str::from_utf8(b).ok()?;
    if !detect(b) {
        return None;
    }
    let v2 = t.contains("\"v\":2") || t.contains("\"v\": 2");
    // legacy packet form uses unquoted `location x` / `identifier y` lines
    let bare = if v2 { 0 } else { 1 };
    Some(Macaroon {
        v2,
        version: t.find("\"v\":").map(|_| 1),
        locations: count_key(t, "location")
            + count_key(t, "l")
            + bare * t.matches("location").count(),
        identifiers: count_key(t, "identifier")
            + count_key(t, "i")
            + bare * t.matches("identifier").count(),
        cids: t.matches("cid").count(),
        vids: t.matches("vid").count(),
        signed: t.contains("signature") || t.contains("\"s64\"") || t.contains("\"s\""),
        objects: t.matches('{').count(),
        hex_sig: t.contains("signature"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"{\"v\":2,\"l\":\"loc\",\"i\":\"id\",\"c\":[{\"cid\":\"a\"},{\"cid\":\"b\",\"vid\":\"v\"}],\"s64\":\"sig\"}";

    #[test]
    fn parses_v2() {
        let m = parse(D).unwrap();
        assert!(m.v2);
        assert_eq!(m.locations, 1);
        assert_eq!(m.identifiers, 1);
        assert_eq!(m.cids, 2);
        assert_eq!(m.vids, 1);
        assert!(m.signed);
        assert!(m.objects >= 3);
    }

    #[test]
    fn parses_legacy() {
        let m = parse(b"location http://x\nidentifier root\ncid a\nsignature 0123").unwrap();
        assert!(!m.v2);
        assert_eq!(m.locations, 1);
        assert!(m.hex_sig);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"{\"a\":1}"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"[]").is_none());
    }
}
