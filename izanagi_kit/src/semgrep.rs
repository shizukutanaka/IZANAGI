//! Semgrep JSON output parser (`semgrep --json`).
//!
//! Detects `{"results":[{…,"check_id":…,"extra":{…}}]}` documents and
//! counts findings, ERROR/WARNING/INFO severities, distinct rule ids,
//! parse errors and interfile findings.
//!
//! ```
//! let b = concat!(
//!     "{\"results\":[{\"check_id\":\"rules.a\",\"path\":\"f.py\",",
//!     "\"start\":{\"line\":1},\"end\":{\"line\":1},",
//!     "\"extra\":{\"severity\":\"ERROR\",\"message\":\"m\"}}],",
//!     "\"errors\":[{\"type\":\"x\"}]}"
//! ).as_bytes();
//! assert!(izanagi_kit::semgrep::detect(b));
//! let c = izanagi_kit::semgrep::Semgrep::parse(b).unwrap();
//! assert_eq!(c.results, 1);
//! assert_eq!(c.errors, 1);
//! ```

use crate::textutil::strip_bom;
/// Parsed Semgrep JSON output summary.
#[derive(Debug, Clone)]
pub struct Semgrep {
    /// Result findings (`"check_id"` keys).
    pub results: usize,
    /// `"severity":"ERROR"` findings.
    pub error: usize,
    /// `"severity":"WARNING"` findings.
    pub warning: usize,
    /// `"severity":"INFO"` findings.
    pub info: usize,
    /// Distinct `"path"` values.
    pub paths: usize,
    /// `"errors"` array entries (`"type"` in errors block counted via
    /// top-level `"type"` keys — approximated by `"type"` keys total).
    pub errors: usize,
    /// `"fingerprint"`/`"lines"` provenance fields.
    pub fingerprints: usize,
}

fn count_key(t: &str, key: &str) -> usize {
    let mut n = 0;
    let mut off = 0;
    while let Some(i) = t[off..].find(key) {
        n += 1;
        off += i + key.len();
    }
    n
}

/// Whether the buffer looks like a Semgrep JSON document.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    if !t.trim_start().starts_with('{') {
        return false;
    }
    t.contains("\"check_id\"")
        && (t.contains("\"results\"") || t.contains("\"extra\"") || t.contains("\"path\""))
}

impl Semgrep {
    /// Parses a Semgrep JSON document summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let head = t.find("\"errors\"").map_or(t, |e| &t[..e]);
        let mut paths: Vec<String> = Vec::new();
        let mut off = 0;
        while let Some(i) = head[off..].find("\"path\"") {
            let seg = &head[off + i + 6..];
            if let Some(c1) = seg.find('"') {
                if let Some(c2) = seg[c1 + 1..].find('"') {
                    let v = &seg[c1 + 1..c1 + 1 + c2];
                    if !v.is_empty() && !paths.iter().any(|x| x == v) {
                        paths.push(v.to_string());
                    }
                }
            }
            off += i + 6;
        }
        let errors = t
            .find("\"errors\"")
            .map_or(0usize, |e| count_key(&t[e..], "\"type\""));
        Some(Self {
            results: count_key(t, "\"check_id\""),
            error: count_key(t, "\"ERROR\""),
            warning: count_key(t, "\"WARNING\""),
            info: count_key(t, "\"INFO\""),
            paths: paths.len(),
            errors,
            fingerprints: count_key(t, "\"fingerprint\"") + count_key(t, "\"lines\""),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = concat!(
            "{\"results\":[",
            "{\"check_id\":\"rules.a\",\"path\":\"a.py\",",
            "\"start\":{\"line\":1},\"end\":{\"line\":2},",
            "\"extra\":{\"severity\":\"ERROR\",\"message\":\"m\",",
            "\"fingerprint\":\"f1\",\"lines\":\"x()\"}},",
            "{\"check_id\":\"rules.b\",\"path\":\"b.py\",",
            "\"extra\":{\"severity\":\"WARNING\",\"message\":\"n\"}},",
            "{\"check_id\":\"rules.c\",\"path\":\"a.py\",",
            "\"extra\":{\"severity\":\"INFO\",\"message\":\"o\"}}],",
            "\"errors\":[{\"type\":\"ParseError\",\"path\":\"c.py\"}]}"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Semgrep::parse(b).unwrap();
        assert_eq!(c.results, 3);
        assert_eq!(c.error, 1);
        assert_eq!(c.warning, 1);
        assert_eq!(c.info, 1);
        assert_eq!(c.paths, 2);
        assert_eq!(c.errors, 1);
        assert_eq!(c.fingerprints, 2);
    }

    #[test]
    fn rejects_other_json() {
        assert!(!detect(b"{\"results\": []}"));
        assert!(!detect(b"{\"check_id\": \"x\"}"));
        assert!(Semgrep::parse(b"{}").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
