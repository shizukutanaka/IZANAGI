//! JUnit XML test reports (`<testsuite>`/`<testsuites>`).
//!
//! `<testsuite name tests failures errors skipped>` holds
//! `<testcase classname name>` elements whose child `<failure>` or
//! `<error>`/`skipped` element marks the verdict.
//!
//! ```
//! use izanagi_kit::junit::{parse, Verdict};
//!
//! let x = parse(br#"<testsuite name="s" tests="2" failures="1" errors="0" skipped="0"><testcase classname="C" name="ok"/><testcase classname="C" name="bad"><failure message="boom"/></testcase></testsuite>"#).unwrap();
//! assert_eq!(x.failures, Some(1));
//! assert_eq!(x.cases[1].verdict, Verdict::Failure);
//! ```

/// A `<testcase>` outcome.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// No failure/error/skipped child.
    Passed,
    /// `<failure>` child present.
    Failure,
    /// `<error>` child present.
    Error,
    /// `<skipped>` child present.
    Skipped,
}

/// A `<testcase>` element.
#[derive(Clone, Debug)]
pub struct Case {
    /// `classname` attribute.
    pub classname: Option<String>,
    /// `name` attribute.
    pub name: String,
    /// `time` attribute, verbatim seconds text.
    pub time: Option<String>,
    /// Outcome from child elements.
    pub verdict: Verdict,
    /// `<failure>`/`<error>` `message` attribute when present.
    pub message: Option<String>,
}

/// A parsed `<testsuite>`.
#[derive(Clone, Debug)]
pub struct Junit {
    /// `name` attribute.
    pub name: Option<String>,
    /// `tests` count.
    pub tests: Option<u64>,
    /// `failures` count.
    pub failures: Option<u64>,
    /// `errors` count.
    pub errors: Option<u64>,
    /// `skipped` count.
    pub skipped: Option<u64>,
    /// Cases in document order.
    pub cases: Vec<Case>,
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

/// Parse a JUnit report. `None` without `<testsuite` or `<testsuites`,
/// or when a `<testcase` lacks `name`.
pub fn parse(d: &[u8]) -> Option<Junit> {
    let text = std::str::from_utf8(d).ok()?;
    let mut pos = 0;
    let head = loop {
        let s = text[pos..].find('<')? + pos;
        if text[s..].starts_with("<?") || text[s..].starts_with("<!--") {
            let e = if text[s..].starts_with("<!--") {
                // `<!--` コメントは `-->` まで読み飛ばす(コメント内の `>` で誤終了しない)。
                text[s..].find("-->")? + s + 2
            } else {
                text[s..].find('>')? + s
            };
            pos = e + 1;
            continue;
        }
        break s;
    };
    let mut base = head;
    let mut name = None;
    let mut tests = None;
    let mut failures = None;
    let mut errors = None;
    let mut skipped = None;
    if text[head..].starts_with("<testsuite") && !text[head..].starts_with("<testsuites") {
        let e = text[head..].find('>')? + head;
        let tag = &text[head..e];
        name = attr(tag, "name");
        tests = u64attr(tag, "tests");
        failures = u64attr(tag, "failures");
        errors = u64attr(tag, "errors");
        skipped = u64attr(tag, "skipped");
        base = e;
    } else if text[head..].starts_with("<testsuites") {
        // wrapper: take stats from first suite — `<testsuites` itself
        // also matches the `<testsuite` prefix, so skip it
        let mut scan = head;
        while let Some(rel) = text[scan..].find("<testsuite") {
            let s = scan + rel;
            if text.as_bytes().get(s + 10) == Some(&b's') {
                scan = s + 10;
                continue;
            }
            let e = text[s..].find('>')? + s;
            let tag = &text[s..e];
            name = attr(tag, "name");
            tests = u64attr(tag, "tests");
            failures = u64attr(tag, "failures");
            errors = u64attr(tag, "errors");
            skipped = u64attr(tag, "skipped");
            base = e;
            break;
        }
    } else {
        return None;
    }
    let mut cases = Vec::new();
    let mut i = base;
    while let Some(rel) = text[i..].find("<testcase") {
        let s = i + rel;
        let e = text[s..].find('>')? + s;
        let tag = &text[s..e];
        let name = attr(tag, "name")?;
        let selfclose = tag.ends_with('/');
        let body_end = if selfclose {
            None
        } else {
            text[s..].find("</testcase>").map(|p| s + p)
        };
        let mut verdict = Verdict::Passed;
        let mut message = None;
        if let Some(be) = body_end {
            let body = &text[s..be];
            for (mark, v) in [
                ("<failure", Verdict::Failure),
                ("<error", Verdict::Error),
                ("<skipped", Verdict::Skipped),
            ] {
                if let Some(mi) = body.find(mark) {
                    verdict = v;
                    if let Some(me) = body[mi..].find('>') {
                        if let Some(m) = attr(&body[mi..mi + me], "message") {
                            message = Some(m);
                        }
                    }
                    break;
                }
            }
            i = be;
        } else {
            i = e + 1;
        }
        cases.push(Case {
            classname: attr(tag, "classname"),
            name,
            time: attr(tag, "time"),
            verdict,
            message,
        });
    }
    Some(Junit {
        name,
        tests,
        failures,
        errors,
        skipped,
        cases,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let x = parse(br#"<?xml version="1.0"?><testsuites><testsuite name="s" tests="3" failures="1" errors="0" skipped="1"><testcase classname="C" name="a" time="0.01"/><testcase name="b"><skipped/></testcase><testcase name="c"><error message="e"/></testcase></testsuite></testsuites>"#).unwrap();
        assert_eq!(x.name.as_deref(), Some("s"));
        assert_eq!(x.tests, Some(3));
        assert_eq!(x.cases.len(), 3);
        assert_eq!(x.cases[0].verdict, Verdict::Passed);
        assert_eq!(x.cases[0].classname.as_deref(), Some("C"));
        assert_eq!(x.cases[1].verdict, Verdict::Skipped);
        assert_eq!(x.cases[2].verdict, Verdict::Error);
        assert_eq!(x.cases[2].message.as_deref(), Some("e"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<foo/>").is_none());
        assert!(parse(b"<testsuite><testcase/></testsuite>").is_none());
    }
}
