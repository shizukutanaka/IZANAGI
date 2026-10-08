//! SonarQube/SonarCloud issues JSON parser (`api/issues/search`,
//! scanner report).
//!
//! Detects `{"total":N,"issues":[{…,"rule":…,"severity":…}]}` documents
//! and counts issues, five severity classes, three type classes,
//! rule keys and components.
//!
//! ```
//! let b = concat!(
//!     "{\"total\":1,\"issues\":[{\"key\":\"k\",\"rule\":\"squid:S1\",",
//!     "\"severity\":\"MAJOR\",\"component\":\"p:f.java\",",
//!     "\"type\":\"BUG\"}]}"
//! ).as_bytes();
//! assert!(izanagi_kit::sonar::detect(b));
//! let c = izanagi_kit::sonar::Sonar::parse(b).unwrap();
//! assert_eq!(c.issues, 1);
//! assert_eq!(c.major, 1);
//! ```

use crate::textutil::strip_bom;
/// Parsed SonarQube issues document summary.
#[derive(Debug, Clone)]
pub struct Sonar {
    /// Issue entries (`"rule"` keys within `issues`).
    pub issues: usize,
    /// `"severity":"BLOCKER"`.
    pub blocker: usize,
    /// `"severity":"CRITICAL"`.
    pub critical: usize,
    /// `"severity":"MAJOR"`.
    pub major: usize,
    /// `"severity":"MINOR"`/`"INFO"`.
    pub minor: usize,
    /// `"type":"BUG"` issues.
    pub bugs: usize,
    /// `"type":"VULNERABILITY"` issues.
    pub vulnerabilities: usize,
    /// `"type":"CODE_SMELL"` issues.
    pub code_smells: usize,
    /// `"component"` values.
    pub components: usize,
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

/// Whether the buffer looks like a SonarQube issues document.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    if !t.trim_start().starts_with('{') {
        return false;
    }
    t.contains("\"issues\"")
        && t.contains("\"severity\"")
        && (t.contains("\"component\"") || t.contains("\"rule\"") || t.contains("\"squid\""))
}

impl Sonar {
    /// Parses a SonarQube issues document summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        Some(Self {
            issues: count_key(t, "\"rule\""),
            blocker: count_key(t, "\"BLOCKER\""),
            critical: count_key(t, "\"CRITICAL\""),
            major: count_key(t, "\"MAJOR\""),
            minor: count_key(t, "\"MINOR\"") + count_key(t, "\"INFO\""),
            bugs: count_key(t, "\"BUG\""),
            vulnerabilities: count_key(t, "\"VULNERABILITY\""),
            code_smells: count_key(t, "\"CODE_SMELL\""),
            components: count_key(t, "\"component\""),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = concat!(
            "{\"total\":4,\"issues\":[",
            "{\"key\":\"k1\",\"rule\":\"squid:S1\",\"severity\":\"BLOCKER\",",
            "\"component\":\"p:a.java\",\"type\":\"BUG\"},",
            "{\"key\":\"k2\",\"rule\":\"squid:S2\",\"severity\":\"CRITICAL\",",
            "\"component\":\"p:b.java\",\"type\":\"VULNERABILITY\"},",
            "{\"key\":\"k3\",\"rule\":\"squid:S3\",\"severity\":\"MAJOR\",",
            "\"component\":\"p:a.java\",\"type\":\"CODE_SMELL\"},",
            "{\"key\":\"k4\",\"rule\":\"squid:S4\",\"severity\":\"MINOR\",",
            "\"component\":\"p:c.java\",\"type\":\"CODE_SMELL\"}]}"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Sonar::parse(b).unwrap();
        assert_eq!(c.issues, 4);
        assert_eq!(c.blocker, 1);
        assert_eq!(c.critical, 1);
        assert_eq!(c.major, 1);
        assert_eq!(c.minor, 1);
        assert_eq!(c.bugs, 1);
        assert_eq!(c.vulnerabilities, 1);
        assert_eq!(c.code_smells, 2);
        assert_eq!(c.components, 4);
    }

    #[test]
    fn rejects_other_json() {
        assert!(!detect(b"{\"issues\": []}"));
        assert!(!detect(b"{\"severity\": \"MAJOR\"}"));
        assert!(Sonar::parse(b"{}").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
