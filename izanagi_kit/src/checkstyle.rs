//! Checkstyle XML reports — `<checkstyle><file><error/>` findings.
//!
//! Each `<file name>` holds `<error line column severity message
//! source/>` self-closing findings.
//!
//! ```
//! use izanagi_kit::checkstyle::parse;
//!
//! let c = parse(br#"<checkstyle version="8.0"><file name="A.java"><error line="3" column="9" severity="warning" message="missing" source="Rule"/></file></checkstyle>"#).unwrap();
//! assert_eq!(c.files[0].errors[0].severity, "warning");
//! ```

/// One `<error>` finding.
#[derive(Clone, Debug)]
pub struct Finding {
    /// `line` (1-based).
    pub line: u64,
    /// `column` (0 when absent).
    pub column: u64,
    /// `severity` (`info`/`warning`/`error`).
    pub severity: String,
    /// `message` text.
    pub message: String,
    /// `source` rule id.
    pub source: Option<String>,
}

/// A `<file>` element with its findings.
#[derive(Clone, Debug)]
pub struct File {
    /// `name` attribute (path).
    pub name: String,
    /// `<error>` findings in order.
    pub errors: Vec<Finding>,
}

/// A parsed checkstyle report.
#[derive(Clone, Debug)]
pub struct Checkstyle {
    /// `version` attribute.
    pub version: Option<String>,
    /// Files in document order.
    pub files: Vec<File>,
}

fn attr(tag: &str, key: &str) -> Option<String> {
    let pat = format!("{key}=\"");
    let i = tag.find(&pat)?;
    let start = i + pat.len();
    let end = tag[start..].find('"')? + start;
    Some(tag[start..end].to_string())
}

/// Parse a checkstyle report. `None` without `<checkstyle`, when a
/// `<file` lacks `name`, or an `<error` lacks `line`/`severity`/`message`.
pub fn parse(d: &[u8]) -> Option<Checkstyle> {
    let text = std::str::from_utf8(d).ok()?;
    let cs = text.find("<checkstyle")?;
    let ce = text[cs..].find('>')? + cs;
    let version = attr(&text[cs..ce], "version");
    let mut files = Vec::new();
    let mut i = ce;
    while let Some(rel) = text[i..].find("<file") {
        let s = i + rel;
        let e = text[s..].find('>')? + s;
        let tag = &text[s..e];
        let name = attr(tag, "name")?;
        let mut errors = Vec::new();
        if !tag.ends_with('/') {
            let fend = text[s..].find("</file>").map(|p| s + p).unwrap_or(e);
            let body = &text[e + 1..fend.min(text.len())];
            let mut j = 0;
            while let Some(rr) = body[j..].find("<error") {
                let es = j + rr;
                let ee = body[es..].find('>')? + es;
                let et = &body[es..ee];
                errors.push(Finding {
                    line: attr(et, "line")?.parse().ok()?,
                    column: attr(et, "column")
                        .map(|c| c.parse().ok().unwrap_or(0))
                        .unwrap_or(0),
                    severity: attr(et, "severity")?,
                    message: attr(et, "message")?,
                    source: attr(et, "source"),
                });
                j = ee + 1;
            }
            i = fend;
        } else {
            i = e + 1;
        }
        files.push(File { name, errors });
    }
    Some(Checkstyle { version, files })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let c = parse(br#"<checkstyle version="8"><file name="a"><error line="1" column="2" severity="info" message="m" source="S"/></file><file name="b"/></checkstyle>"#).unwrap();
        assert_eq!(c.version.as_deref(), Some("8"));
        assert_eq!(c.files.len(), 2);
        assert_eq!(c.files[0].errors[0].line, 1);
        assert_eq!(c.files[0].errors[0].source.as_deref(), Some("S"));
        assert!(c.files[1].errors.is_empty());
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<checkstyle><file/></checkstyle>").is_none());
        assert!(
            parse(b"<checkstyle><file name=\"a\"><error line=\"1\"/></file></checkstyle>")
                .is_none()
        );
    }
}
