//! Checkov JSON scan report parser (`checkov -o json`).
//!
//! Detects `{"check_type":…,"results":{"passed_checks":…,"failed_checks":
//! …}}` documents and counts passed/failed/skipped checks, parsing
//! errors, severities and distinct `check_type` values.
//!
//! ```
//! let b = concat!(
//!     "{\"check_type\":\"terraform\",\"results\":{",
//!     "\"passed_checks\":[{\"check_id\":\"CKV_AWS_1\"}],",
//!     "\"failed_checks\":[{\"check_id\":\"CKV_AWS_2\",\"severity\":\"HIGH\"}],",
//!     "\"skipped_checks\":[],\"parsing_errors\":[]}}"
//! ).as_bytes();
//! assert!(izanagi_kit::checkov::detect(b));
//! let c = izanagi_kit::checkov::Checkov::parse(b).unwrap();
//! assert_eq!(c.passed, 1);
//! assert_eq!(c.failed, 1);
//! ```

/// Parsed Checkov JSON report summary.
#[derive(Debug, Clone)]
pub struct Checkov {
    /// `"passed_checks"` entries.
    pub passed: usize,
    /// `"failed_checks"` entries.
    pub failed: usize,
    /// `"skipped_checks"` entries.
    pub skipped: usize,
    /// `"parsing_errors"` entries.
    pub parsing_errors: usize,
    /// Distinct `"check_type"` values (`terraform`, `kubernetes`, …).
    pub check_types: usize,
    /// `"severity":"HIGH"`/`"CRITICAL"` findings.
    pub high: usize,
    /// `"resource"` values.
    pub resources: usize,
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

fn entries_in(t: &str, key: &str) -> usize {
    let Some(i) = t.find(key) else {
        return 0;
    };
    let seg = &t[i + key.len()..];
    let Some(open) = seg.find('[') else {
        return 0;
    };
    let mut depth = 0usize;
    let mut end = seg.len();
    for (j, ch) in seg[open..].char_indices() {
        match ch {
            '[' => depth += 1,
            ']' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    end = open + j;
                    break;
                }
            }
            _ => {}
        }
    }
    count_key(&seg[open..end], "\"check_id\"") + count_key(&seg[open..end], "\"resource_id\"")
}

/// Whether the buffer looks like a Checkov JSON report.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    if !t.trim_start().starts_with('{') && !t.trim_start().starts_with('[') {
        return false;
    }
    (t.contains("\"passed_checks\"") && t.contains("\"failed_checks\""))
        || (t.contains("\"check_type\"") && t.contains("CKV_"))
}

impl Checkov {
    /// Parses a Checkov JSON report summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut types: Vec<String> = Vec::new();
        let mut off = 0;
        while let Some(i) = t[off..].find("\"check_type\"") {
            let seg = &t[off + i + 12..];
            if let Some(c1) = seg.find('"') {
                if let Some(c2) = seg[c1 + 1..].find('"') {
                    let v = &seg[c1 + 1..c1 + 1 + c2];
                    if !v.is_empty() && !types.iter().any(|x| x == v) {
                        types.push(v.to_string());
                    }
                }
            }
            off += i + 12;
        }
        Some(Self {
            passed: entries_in(t, "\"passed_checks\""),
            failed: entries_in(t, "\"failed_checks\""),
            skipped: entries_in(t, "\"skipped_checks\""),
            parsing_errors: entries_in(t, "\"parsing_errors\""),
            check_types: types.len(),
            high: count_key(t, "\"HIGH\"") + count_key(t, "\"CRITICAL\""),
            resources: count_key(t, "\"resource\""),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = concat!(
            "{\"check_type\":\"terraform\",\"results\":{",
            "\"passed_checks\":[{\"check_id\":\"CKV_AWS_1\",\"file\":\"main.tf\"}],",
            "\"failed_checks\":[",
            "{\"check_id\":\"CKV_AWS_2\",\"severity\":\"HIGH\",",
            "\"resource\":\"aws_s3.a\",\"file\":\"main.tf\"},",
            "{\"check_id\":\"CKV_AWS_3\",\"severity\":\"CRITICAL\",",
            "\"resource\":\"aws_iam.b\",\"file\":\"iam.tf\"}],",
            "\"skipped_checks\":[{\"check_id\":\"CKV_AWS_4\"}],",
            "\"parsing_errors\":[\"bad.tf\"]}}"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Checkov::parse(b).unwrap();
        assert_eq!(c.passed, 1);
        assert_eq!(c.failed, 2);
        assert_eq!(c.skipped, 1);
        assert_eq!(c.check_types, 1);
        assert_eq!(c.high, 2);
        assert_eq!(c.resources, 2);
    }

    #[test]
    fn rejects_other_json() {
        assert!(!detect(b"{\"results\": {}}"));
        assert!(Checkov::parse(b"{}").is_none());
    }
}
