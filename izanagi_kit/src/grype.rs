//! Anchore Grype JSON scan report parser (`grype -o json`).
//!
//! Detects `{"matches":[{…,"vulnerability":{…},"artifact":{…}}]}`
//! documents and counts matches, severities, distinct artifact types,
//! fixed vulnerabilities and ignored matches.
//!
//! ```
//! let b = concat!(
//!     "{\"matches\":[{\"vulnerability\":{\"id\":\"CVE-1\",",
//!     "\"severity\":\"High\",\"fix\":{\"versions\":[\"2\"]}},",
//!     "\"artifact\":{\"name\":\"p\",\"version\":\"1\",\"type\":\"deb\"}}]}"
//! ).as_bytes();
//! assert!(izanagi_kit::grype::detect(b));
//! let c = izanagi_kit::grype::Grype::parse(b).unwrap();
//! assert_eq!(c.matches, 1);
//! assert_eq!(c.fixed, 1);
//! ```

/// Parsed Grype JSON report summary.
#[derive(Debug, Clone)]
pub struct Grype {
    /// `"matches"` entries (`"vulnerability"` objects).
    pub matches: usize,
    /// `"severity":"Critical"`.
    pub critical: usize,
    /// `"severity":"High"`.
    pub high: usize,
    /// `"severity":"Medium"`.
    pub medium: usize,
    /// `"severity":"Low"`/`"Negligible"`.
    pub low: usize,
    /// Distinct `"type"` artifact types (`deb`, `npm`, `python`, …).
    pub artifact_types: usize,
    /// Matches with a `"fix"` containing versions or `state":"fixed"`.
    pub fixed: usize,
    /// `"ignoredMatches"` entries.
    pub ignored: usize,
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
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// Whether the buffer looks like a Grype JSON report.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    if !t.trim_start().starts_with('{') {
        return false;
    }
    t.contains("\"matches\"")
        && t.contains("\"vulnerability\"")
        && (t.contains("\"artifact\"") || t.contains("\"severity\""))
}

impl Grype {
    /// Parses a Grype JSON report summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut types: Vec<String> = Vec::new();
        let mut off = 0;
        while let Some(i) = t[off..].find("\"type\"") {
            let seg = &t[off + i + 6..];
            if let Some(c1) = seg.find('"') {
                if let Some(c2) = seg[c1 + 1..].find('"') {
                    let v = &seg[c1 + 1..c1 + 1 + c2];
                    if !v.is_empty() && !types.iter().any(|x| x == v) {
                        types.push(v.to_string());
                    }
                }
            }
            off += i + 6;
        }
        Some(Self {
            matches: count_key(t, "\"vulnerability\""),
            critical: count_key(t, "\"Critical\""),
            high: count_key(t, "\"High\""),
            medium: count_key(t, "\"Medium\""),
            low: count_key(t, "\"Low\"") + count_key(t, "\"Negligible\""),
            artifact_types: types.len(),
            fixed: count_key(t, "\"fixed\"") + count_key(t, "\"versions\""),
            ignored: count_key(t, "\"ignoredMatches\""),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = concat!(
            "{\"matches\":[",
            "{\"vulnerability\":{\"id\":\"CVE-1\",\"severity\":\"Critical\",",
            "\"fix\":{\"versions\":[\"2\"],\"state\":\"fixed\"}},",
            "\"artifact\":{\"name\":\"a\",\"version\":\"1\",\"type\":\"deb\"}},",
            "{\"vulnerability\":{\"id\":\"CVE-2\",\"severity\":\"High\"},",
            "\"artifact\":{\"name\":\"b\",\"version\":\"2\",\"type\":\"deb\"}},",
            "{\"vulnerability\":{\"id\":\"GHSA-3\",\"severity\":\"Medium\"},",
            "\"artifact\":{\"name\":\"c\",\"version\":\"3\",\"type\":\"npm\"}}],",
            "\"ignoredMatches\":[{\"vulnerability\":{\"id\":\"CVE-9\"}}]}"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Grype::parse(b).unwrap();
        assert_eq!(c.matches, 4);
        assert_eq!(c.critical, 1);
        assert_eq!(c.high, 1);
        assert_eq!(c.medium, 1);
        assert_eq!(c.artifact_types, 2);
        assert_eq!(c.fixed, 2);
        assert_eq!(c.ignored, 1);
    }

    #[test]
    fn rejects_other_json() {
        assert!(!detect(b"{\"matches\": []}"));
        assert!(!detect(b"{\"vulnerability\": {}}"));
        assert!(Grype::parse(b"{}").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
