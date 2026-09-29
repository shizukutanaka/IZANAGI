//! SpotBugs/FindBugs XML report parser (`<BugCollection>`).
//!
//! Detects `<BugCollection>` documents and counts `<BugInstance>`
//! findings, priorities, `<Class>` stats, `<Error>` nodes and distinct
//! bug `type`/`category` attributes.
//!
//! ```
//! let b = br#"<BugCollection><Project><Jar>a.jar</Jar></Project>
//! <BugInstance type="NP_NULL" priority="1" category="CORRECTNESS"/>
//! <BugInstance type="EI_EXPOSE" priority="2" category="BAD_PRACTICE"/>
//! <FindBugsSummary total_bugs="2"/><Errors errors="0"/></BugCollection>"#;
//! assert!(izanagi_kit::spotbugs::detect(b));
//! let c = izanagi_kit::spotbugs::Spotbugs::parse(b).unwrap();
//! assert_eq!(c.bugs, 2);
//! assert_eq!(c.categories, 2);
//! ```

/// Parsed SpotBugs XML report summary.
#[derive(Debug, Clone)]
pub struct Spotbugs {
    /// `<BugInstance>` findings.
    pub bugs: usize,
    /// `priority="1"` (high) findings.
    pub high_priority: usize,
    /// `priority="2"` (medium) findings.
    pub medium_priority: usize,
    /// `priority="3"`/`"4"` (low/experimental) findings.
    pub low_priority: usize,
    /// Distinct `type` attribute values.
    pub types: usize,
    /// Distinct `category` attribute values.
    pub categories: usize,
    /// `<Class>` entries.
    pub classes: usize,
    /// `<Error>` nodes.
    pub errors: usize,
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

fn attr_values(t: &str, attr: &str) -> usize {
    let mut vs: Vec<String> = Vec::new();
    let mut off = 0;
    while let Some(i) = t[off..].find(attr) {
        let seg = &t[off + i + attr.len()..];
        if let Some(end) = seg.find('"') {
            let v = &seg[..end];
            if !v.is_empty() && !vs.iter().any(|x| x == v) {
                vs.push(v.to_string());
            }
        }
        off += i + attr.len();
    }
    vs.len()
}

/// Whether the buffer looks like a SpotBugs/FindBugs XML report.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("<BugCollection") || t.contains("<BugInstance")
}

impl Spotbugs {
    /// Parses a SpotBugs XML report summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        Some(Self {
            bugs: count_key(t, "<BugInstance"),
            high_priority: count_key(t, "priority=\"1\""),
            medium_priority: count_key(t, "priority=\"2\""),
            low_priority: count_key(t, "priority=\"3\"") + count_key(t, "priority=\"4\""),
            types: attr_values(t, "type=\""),
            categories: attr_values(t, "category=\""),
            classes: count_key(t, "<Class"),
            errors: count_key(t, "<Error"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = concat!(
            "<BugCollection><Project><Jar>a.jar</Jar></Project>",
            "<BugInstance type=\"NP_NULL\" priority=\"1\" category=\"CORRECTNESS\"/>",
            "<BugInstance type=\"EI_EXPOSE\" priority=\"2\" category=\"BAD_PRACTICE\"/>",
            "<BugInstance type=\"DLS_DEAD\" priority=\"3\" category=\"PERFORMANCE\"/>",
            "<Class class=\"a.B\"><SourceLine/></Class>",
            "<FindBugsSummary total_bugs=\"3\"/><Errors errors=\"0\"/></BugCollection>"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Spotbugs::parse(b).unwrap();
        assert_eq!(c.bugs, 3);
        assert_eq!(c.high_priority, 1);
        assert_eq!(c.medium_priority, 1);
        assert_eq!(c.low_priority, 1);
        assert_eq!(c.types, 3);
        assert_eq!(c.categories, 3);
        assert_eq!(c.classes, 1);
        assert_eq!(c.errors, 1);
    }

    #[test]
    fn rejects_other_xml() {
        assert!(!detect(b"<root/>"));
        assert!(!detect(b"plain"));
        assert!(Spotbugs::parse(b"<x/>").is_none());
    }
}
