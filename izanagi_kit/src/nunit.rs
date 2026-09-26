//! NUnit XML results — v2 `<test-results>` and v3 `<test-run>`.
//!
//! v2: `<test-results name total errors failures not-run>` with
//! `<test-case name executed success result>` leaf nodes.
//! v3: `<test-run total passed failed>` with `<test-case result>`
//! leaves (`Passed`/`Failed`/`Skipped`).
//!
//! ```
//! use izanagi_kit::nunit::parse;
//!
//! let n = parse(br#"<test-results name="r" total="2" errors="0" failures="1" not-run="0"><results><test-suite name="s"><results><test-case name="a" executed="True" success="True" result="Success"/><test-case name="b" executed="True" success="False" result="Failure"/></results></test-suite></results></test-results>"#).unwrap();
//! assert_eq!(n.total, Some(2));
//! assert_eq!(n.cases[1].result, "Failure");
//! ```

/// A `<test-case>` leaf.
#[derive(Clone, Debug)]
pub struct TestCase {
    /// `name` attribute.
    pub name: String,
    /// `executed` (v2, `"True"`/`"False"`).
    pub executed: Option<String>,
    /// `success` (v2) — verbatim.
    pub success: Option<String>,
    /// `result` attribute (`Success`/`Failure`/`Skipped`, v3 `Passed`…).
    pub result: String,
    /// `time` attribute, verbatim.
    pub time: Option<String>,
}

/// A parsed NUnit result document (v2 `<test-results>` or v3 `<test-run>`).
#[derive(Clone, Debug)]
pub struct Nunit {
    /// `name` attribute.
    pub name: Option<String>,
    /// `total` count.
    pub total: Option<u64>,
    /// `errors`/`failures` counts (v2).
    pub errors: Option<u64>,
    /// `failures` count (v2) / `failed` (v3).
    pub failures: Option<u64>,
    /// Document version (`2` for test-results, `3` for test-run).
    pub version: u8,
    /// Leaf `<test-case>` elements in document order.
    pub cases: Vec<TestCase>,
}

fn attr(tag: &str, key: &str) -> Option<String> {
    let pat = format!("{key}=\"");
    let i = tag.find(&pat)?;
    let start = i + pat.len();
    let end = tag[start..].find('"')? + start;
    Some(tag[start..end].to_string())
}

fn u64attr(tag: &str, key: &str) -> Option<u64> {
    attr(tag, key)?.parse().ok()
}

/// Parse an NUnit result document. `None` without a v2/v3 root or on a
/// `<test-case` missing `name`/`result`.
pub fn parse(d: &[u8]) -> Option<Nunit> {
    let text = std::str::from_utf8(d).ok()?;
    let (root, version) = match text.find("<test-results") {
        Some(p) => (p, 2),
        None => (text.find("<test-run")?, 3),
    };
    let e = text[root..].find('>')? + root;
    let tag = &text[root..e];
    let name = attr(tag, "name");
    let total = u64attr(tag, "total");
    let (errors, failures) = if version == 2 {
        (u64attr(tag, "errors"), u64attr(tag, "failures"))
    } else {
        (u64attr(tag, "errors"), u64attr(tag, "failed"))
    };
    let mut cases = Vec::new();
    let mut i = e;
    while let Some(rel) = text[i..].find("<test-case") {
        let s = i + rel;
        let te = text[s..].find('>')? + s;
        let t = &text[s..te];
        cases.push(TestCase {
            name: attr(t, "name")?,
            executed: attr(t, "executed"),
            success: attr(t, "success"),
            result: attr(t, "result").unwrap_or_default(),
            time: attr(t, "time"),
        });
        i = te + 1;
    }
    Some(Nunit {
        name,
        total,
        errors,
        failures,
        version,
        cases,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let n = parse(br#"<test-results name="r" total="2" errors="0" failures="1"><test-case name="a" executed="True" success="True" result="Success"/><test-case name="b" executed="True" success="False" result="Failure" time="0.2"/></test-results>"#).unwrap();
        assert_eq!(n.version, 2);
        assert_eq!(n.failures, Some(1));
        assert_eq!(n.cases.len(), 2);
        let v3 = parse(br#"<test-run total="1" passed="1" failed="0"><test-case name="x" result="Passed"/></test-run>"#).unwrap();
        assert_eq!(v3.version, 3);
        assert_eq!(v3.cases[0].result, "Passed");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<foo/>").is_none());
        assert!(parse(b"<test-results><test-case result=\"Success\"/></test-results>").is_none());
    }
}
