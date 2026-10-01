//! xUnit.net XML test-results census.
//!
//! xUnit.net reports use `<assemblies>` containing `<assembly>` elements with
//! `total`/`passed`/`failed`/`skipped`/`errors` attributes, `<collection>`
//! elements, `<test ... result="...">` cases and `<failure>`/`<reason>`/
//! `<traits>` diagnostics. `parse` counts elements and `result=` values.
//!
//! ```rust
//! let x = br#"<assemblies timestamp="01/01/2025">
//!   <assembly name="T.dll" total="2" passed="1" failed="1" skipped="0" errors="0">
//!     <collection name="c" total="2">
//!       <test name="a" type="T" method="m" result="Pass" />
//!       <test name="b" type="T" method="n" result="Fail">
//!         <failure><message>boom</message></failure>
//!       </test>
//!     </collection>
//!   </assembly>
//! </assemblies>"#;
//! let c = izanagi_kit::xunit::Xunit::parse(x).unwrap();
//! assert_eq!(c.assemblies, 1);
//! assert_eq!(c.tests, 2);
//! ```

/// xUnit.net report census counts.
#[derive(Debug, Clone)]
pub struct Xunit {
    /// `<assembly` elements.
    pub assemblies: usize,
    /// `<collection` elements.
    pub collections: usize,
    /// `<test` elements.
    pub tests: usize,
    /// `result="Pass"` test results.
    pub passed: usize,
    /// `result="Fail"` test results.
    pub failed: usize,
    /// `result="Skip"` test results.
    pub skipped: usize,
    /// `<failure` elements.
    pub failures: usize,
    /// `<reason` elements (skip reasons).
    pub reasons: usize,
    /// `<trait` elements.
    pub traits: usize,
    /// `<output` elements.
    pub outputs: usize,
}

fn tag_count(t: &str, name: &str) -> usize {
    t.matches(&format!("<{name}>")).count()
        + t.matches(&format!("<{name} ")).count()
        + t.matches(&format!("<{name}/")).count()
}

/// Whether the buffer looks like an xUnit.net results document.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("<assemblies") && (t.contains("<assembly") || t.contains("<collection"))
}

impl Xunit {
    /// Parse an xUnit.net results document into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        Some(Self {
            assemblies: tag_count(t, "assembly"),
            collections: tag_count(t, "collection"),
            tests: tag_count(t, "test"),
            passed: t.matches("result=\"Pass\"").count(),
            failed: t.matches("result=\"Fail\"").count(),
            skipped: t.matches("result=\"Skip\"").count(),
            failures: tag_count(t, "failure"),
            reasons: tag_count(t, "reason"),
            traits: tag_count(t, "trait"),
            outputs: tag_count(t, "output"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_report() {
        let b = br#"<assemblies timestamp="t">
          <assembly name="a.dll" total="3" passed="1" failed="1" skipped="1">
            <collection name="c1">
              <test name="t1" result="Pass" />
              <test name="t2" result="Fail"><failure><message>x</message></failure></test>
              <test name="t3" result="Skip"><reason><message>s</message></reason></test>
            </collection>
            <collection name="c2"><test name="t4" result="Pass" /></collection>
          </assembly>
        </assemblies>"#;
        let c = Xunit::parse(b).unwrap();
        assert_eq!(c.assemblies, 1);
        assert_eq!(c.collections, 2);
        assert_eq!(c.tests, 4);
        assert_eq!(c.passed, 2);
        assert_eq!(c.failed, 1);
        assert_eq!(c.skipped, 1);
        assert_eq!(c.failures, 1);
        assert_eq!(c.reasons, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Xunit::parse(b"<xml />").is_none());
    }
}
