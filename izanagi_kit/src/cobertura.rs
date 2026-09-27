//! Cobertura XML coverage reports.
//!
//! `<coverage line-rate branch-rate ...>` wraps `<packages>/<classes>`;
//! each `<class name filename line-rate>` holds `<method>`s and
//! `<line number hits>`. Rates stay verbatim strings (float-free).
//!
//! ```
//! use izanagi_kit::cobertura::parse;
//!
//! let c = parse(br#"<coverage line-rate="0.80" branch-rate="0.5" version="1"><packages><package name="p"><classes><class name="A" filename="a.java" line-rate="0.9"><lines><line number="3" hits="2"/></lines></class></classes></package></packages></coverage>"#).unwrap();
//! assert_eq!(c.line_rate.as_deref(), Some("0.80"));
//! assert_eq!(c.classes[0].lines[0], (3, 2));
//! ```

/// A `<class>` element.
#[derive(Clone, Debug)]
pub struct Class {
    /// `name` attribute.
    pub name: String,
    /// `filename` attribute.
    pub filename: Option<String>,
    /// `line-rate` attribute, verbatim.
    pub line_rate: Option<String>,
    /// `<line number hits>` pairs.
    pub lines: Vec<(u64, u64)>,
    /// `<method name signature>` names.
    pub methods: Vec<String>,
}

/// A parsed Cobertura report.
#[derive(Clone, Debug)]
pub struct Cobertura {
    /// `line-rate` attribute, verbatim.
    pub line_rate: Option<String>,
    /// `branch-rate` attribute, verbatim.
    pub branch_rate: Option<String>,
    /// `version` attribute.
    pub version: Option<String>,
    /// Classes in document order.
    pub classes: Vec<Class>,
}

fn attr(tag: &str, key: &str) -> Option<String> {
    let pat = format!("{key}=\"");
    let i = tag.find(&pat)?;
    let start = i + pat.len();
    let end = tag[start..].find('"')? + start;
    Some(tag[start..end].to_string())
}

/// Parse a Cobertura report. `None` without a `<coverage` element or
/// when a `<class`/`<line` lacks its required attribute.
pub fn parse(d: &[u8]) -> Option<Cobertura> {
    let text = std::str::from_utf8(d).ok()?;
    let co = text.find("<coverage")?;
    let ce = text[co..].find('>')? + co;
    let root = &text[co..ce];
    let mut classes = Vec::new();
    let mut i = ce;
    while let Some(rel) = text[i..].find("<class") {
        let s = i + rel;
        // `<classes`/`class-name` must not match `<class`
        if !text[s..].starts_with("<class ")
            && !text[s..].starts_with("<class\t")
            && !text[s..].starts_with("<class>")
            && !text[s..].starts_with("<class/")
        {
            i = s + 6;
            continue;
        }
        let e = text[s..].find('>')? + s;
        let tag = &text[s..e];
        let name = attr(tag, "name")?;
        let mut lines = Vec::new();
        let mut methods = Vec::new();
        if !tag.ends_with('/') {
            let send = text[s..].find("</class>").map(|p| s + p).unwrap_or(e);
            let body = &text[e + 1..send.min(text.len())];
            let mut j = 0;
            while let Some(mr) = body[j..].find("<method") {
                let ms = j + mr;
                let me = body[ms..].find('>')? + ms;
                if let Some(m) = attr(&body[ms..me], "name") {
                    methods.push(m);
                }
                j = me + 1;
            }
            let mut j = 0;
            while let Some(lr) = body[j..].find("<line") {
                let ls = j + lr;
                let le = body[ls..].find('>')? + ls;
                let lt = &body[ls..le];
                if let (Some(n), Some(h)) = (attr(lt, "number"), attr(lt, "hits")) {
                    lines.push((n.parse().ok()?, h.parse().ok()?));
                }
                j = le + 1;
            }
            i = send;
        } else {
            i = e + 1;
        }
        classes.push(Class {
            name,
            filename: attr(tag, "filename"),
            line_rate: attr(tag, "line-rate"),
            lines,
            methods,
        });
    }
    Some(Cobertura {
        line_rate: attr(root, "line-rate"),
        branch_rate: attr(root, "branch-rate"),
        version: attr(root, "version"),
        classes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let c = parse(br#"<coverage line-rate="0.5"><packages><package name="p"><classes><class name="A" filename="a"><methods><method name="m"/></methods><lines><line number="1" hits="4"/><line number="2" hits="0"/></lines></class></classes></package></packages></coverage>"#).unwrap();
        assert_eq!(c.classes[0].name, "A");
        assert_eq!(c.classes[0].methods, vec!["m"]);
        assert_eq!(c.classes[0].lines, vec![(1, 4), (2, 0)]);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        // bad <line number> is malformed; missing attrs are skipped
        assert!(parse(b"<coverage><class name=\"a\"><lines><line number=\"x\" hits=\"1\"/></lines></class></coverage>").is_none());
        let c = parse(b"<coverage/>").unwrap();
        assert!(c.classes.is_empty());
    }
}
