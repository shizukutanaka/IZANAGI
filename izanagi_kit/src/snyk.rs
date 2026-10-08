//! Snyk JSON test report parser (`snyk test --json`).
//!
//! Detects `{"vulnerabilities":[{…,"id":"SNYK-…",…}],…}` documents and
//! counts vulnerabilities, severities, upgrade paths, patches, ignored
//! entries and the dependency count field.
//!
//! ```
//! let b = concat!(
//!     "{\"ok\":false,\"dependencyCount\":5,\"packageManager\":\"npm\",",
//!     "\"vulnerabilities\":[{\"id\":\"SNYK-A-1\",\"title\":\"t\",",
//!     "\"severity\":\"high\",\"packageName\":\"p\",\"upgradePath\":[true]}]}"
//! ).as_bytes();
//! assert!(izanagi_kit::snyk::detect(b));
//! let c = izanagi_kit::snyk::Snyk::parse(b).unwrap();
//! assert_eq!(c.vulnerabilities, 1);
//! assert_eq!(c.high, 1);
//! ```

/// Parsed Snyk JSON report summary.
#[derive(Debug, Clone)]
pub struct Snyk {
    /// `"id"` vulnerability entries.
    pub vulnerabilities: usize,
    /// `"severity":"critical"`.
    pub critical: usize,
    /// `"severity":"high"`.
    pub high: usize,
    /// `"severity":"medium"`.
    pub medium: usize,
    /// `"severity":"low"`.
    pub low: usize,
    /// `"upgradePath"` arrays.
    pub upgrade_paths: usize,
    /// `"patches"` arrays.
    pub patches: usize,
    /// `"isIgnored"` true entries / `"filtered"` ignore blocks.
    pub ignored: usize,
    /// `"dependencyCount"` value.
    pub dependency_count: u64,
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

fn int_after(t: &str, key: &str) -> u64 {
    let Some(i) = t.find(key) else {
        return 0;
    };
    let rest = t[i + key.len()..].trim_start();
    let Some(rest) = rest.strip_prefix(':').map(str::trim_start) else {
        return 0;
    };
    let digits: usize = rest.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return 0;
    }
    rest[..digits].parse().unwrap_or(0)
}
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// Whether the buffer looks like a Snyk JSON report.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    if !t.trim_start().starts_with('{') {
        return false;
    }
    t.contains("\"vulnerabilities\"")
        && (t.contains("\"packageManager\"")
            || t.contains("\"dependencyCount\"")
            || t.contains("\"ok\""))
}

impl Snyk {
    /// Parses a Snyk JSON report summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        Some(Self {
            vulnerabilities: count_key(t, "\"SNYK-") + count_key(t, "\"CVE-"),
            critical: count_key(t, "\"critical\""),
            high: count_key(t, "\"high\""),
            medium: count_key(t, "\"medium\""),
            low: count_key(t, "\"low\""),
            upgrade_paths: count_key(t, "\"upgradePath\""),
            patches: count_key(t, "\"patches\""),
            ignored: count_key(t, "\"isIgnored\":true") + count_key(t, "\"filtered\""),
            dependency_count: int_after(t, "\"dependencyCount\""),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = concat!(
            "{\"ok\":false,\"dependencyCount\":12,\"packageManager\":\"npm\",",
            "\"vulnerabilities\":[",
            "{\"id\":\"SNYK-A-1\",\"title\":\"t\",\"severity\":\"critical\",",
            "\"packageName\":\"p\",\"upgradePath\":[true, false],",
            "\"isIgnored\":false},",
            "{\"id\":\"SNYK-A-2\",\"severity\":\"high\",\"patches\":[],",
            "\"isIgnored\":true,\"filtered\":{\"ignore\":[]}},",
            "{\"id\":\"CVE-2024-1\",\"severity\":\"medium\"},",
            "{\"id\":\"SNYK-A-4\",\"severity\":\"low\"}]}"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Snyk::parse(b).unwrap();
        assert_eq!(c.vulnerabilities, 4);
        assert_eq!(c.critical, 1);
        assert_eq!(c.high, 1);
        assert_eq!(c.medium, 1);
        assert_eq!(c.low, 1);
        assert_eq!(c.upgrade_paths, 1);
        assert_eq!(c.patches, 1);
        assert_eq!(c.ignored, 2);
        assert_eq!(c.dependency_count, 12);
    }

    #[test]
    fn rejects_other_json() {
        assert!(!detect(b"{\"vulnerabilities\": []}"));
        assert!(!detect(b"[1]"));
        assert!(Snyk::parse(b"{}").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
