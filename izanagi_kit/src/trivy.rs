//! Trivy JSON scan report parser (`trivy -f json`).
//!
//! Detects `{"SchemaVersion":N,"Results":[{…,"Vulnerabilities":[…]}]}`
//! documents and counts result targets, vulnerabilities, severities,
//! misconfigurations, secrets and license findings.
//!
//! ```
//! let b = concat!(
//!     "{\"SchemaVersion\":2,\"ArtifactName\":\"img\",",
//!     "\"Results\":[{\"Target\":\"x\",\"Class\":\"os-pkgs\",\"Type\":\"debian\",",
//!     "\"Vulnerabilities\":[{\"VulnerabilityID\":\"CVE-1\",\"Severity\":\"HIGH\"}]}]}"
//! ).as_bytes();
//! assert!(izanagi_kit::trivy::detect(b));
//! let c = izanagi_kit::trivy::Trivy::parse(b).unwrap();
//! assert_eq!(c.results, 1);
//! assert_eq!(c.vulnerabilities, 1);
//! ```

/// Parsed Trivy JSON report summary.
#[derive(Debug, Clone)]
pub struct Trivy {
    /// `"Results"` entries (`"Target"` objects).
    pub results: usize,
    /// `"VulnerabilityID"` entries.
    pub vulnerabilities: usize,
    /// `"Severity":"CRITICAL"`.
    pub critical: usize,
    /// `"Severity":"HIGH"`.
    pub high: usize,
    /// `"Severity":"MEDIUM"`.
    pub medium: usize,
    /// `"Severity":"LOW"`.
    pub low: usize,
    /// `"Misconfigurations"` arrays.
    pub misconfigurations: usize,
    /// `"Secrets"` arrays.
    pub secrets: usize,
    /// `"Licenses"` arrays.
    pub licenses: usize,
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

/// Whether the buffer looks like a Trivy JSON report.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    if !t.trim_start().starts_with('{') {
        return false;
    }
    t.contains("\"Results\"")
        && (t.contains("\"Vulnerabilities\"")
            || t.contains("\"VulnerabilityID\"")
            || t.contains("\"SchemaVersion\""))
}

impl Trivy {
    /// Parses a Trivy JSON report summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        Some(Self {
            results: count_key(t, "\"Target\""),
            vulnerabilities: count_key(t, "\"VulnerabilityID\""),
            critical: count_key(t, "\"CRITICAL\""),
            high: count_key(t, "\"HIGH\""),
            medium: count_key(t, "\"MEDIUM\""),
            low: count_key(t, "\"LOW\""),
            misconfigurations: count_key(t, "\"Misconfigurations\""),
            secrets: count_key(t, "\"Secrets\""),
            licenses: count_key(t, "\"Licenses\""),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = concat!(
            "{\"SchemaVersion\":2,\"ArtifactName\":\"img\",\"ArtifactType\":\"fs\",",
            "\"Results\":[",
            "{\"Target\":\"a\",\"Class\":\"os-pkgs\",\"Type\":\"debian\",",
            "\"Vulnerabilities\":[",
            "{\"VulnerabilityID\":\"CVE-1\",\"Severity\":\"CRITICAL\"},",
            "{\"VulnerabilityID\":\"CVE-2\",\"Severity\":\"HIGH\"},",
            "{\"VulnerabilityID\":\"CVE-3\",\"Severity\":\"MEDIUM\"}],",
            "\"Misconfigurations\":[{\"ID\":\"AVD-1\",\"Severity\":\"LOW\"}]},",
            "{\"Target\":\"b\",\"Class\":\"secret\",\"Type\":\"fs\",",
            "\"Secrets\":[{\"RuleID\":\"x\",\"Severity\":\"HIGH\"}],",
            "\"Licenses\":[{\"Category\":\"restricted\"}]}]}"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Trivy::parse(b).unwrap();
        assert_eq!(c.results, 2);
        assert_eq!(c.vulnerabilities, 3);
        assert_eq!(c.critical, 1);
        assert_eq!(c.high, 2);
        assert_eq!(c.medium, 1);
        assert_eq!(c.low, 1);
        assert_eq!(c.misconfigurations, 1);
        assert_eq!(c.secrets, 1);
        assert_eq!(c.licenses, 1);
    }

    #[test]
    fn rejects_other_json() {
        assert!(!detect(b"{\"Results\": []}"));
        assert!(!detect(b"[1]"));
        assert!(Trivy::parse(b"{}").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
