//! Minimal reader for EndNote XML export (`*.xml`).
//!
//! Structure: `<xml><records><record>` per reference, each with
//! `<ref-type name="...">`, `<titles><title>text</title></titles>`,
//! `<contributors><authors><author>Name</author>...</authors></contributors>`,
//! `<dates><year>YYYY</year></dates>`. Self-contained tag scanning — no
//! dependency on a full XML parser.
//!
//! ```
//! use izanagi_kit::endnote::parse;
//!
//! let d = parse(
//!     b"<xml><records><record><ref-type name=\"Journal Article\">17</ref-type>\
//!        <contributors><authors><author>Doe, J.</author></authors></contributors>\
//!        <titles><title>Some work</title></titles>\
//!        <dates><year>1998</year></dates></record></records></xml>",
//! )
//! .unwrap();
//! assert_eq!(d.records.len(), 1);
//! assert_eq!(d.records[0].kind.as_deref(), Some("Journal Article"));
//! assert_eq!(d.records[0].title.as_deref(), Some("Some work"));
//! assert_eq!(d.records[0].authors, vec!["Doe, J."]);
//! ```

/// One `<record>`: reference type, title, authors, year.
#[derive(Debug, Default)]
pub struct Record {
    /// `ref-type` `name` attribute (e.g. `"Journal Article"`).
    pub kind: Option<String>,
    /// `<titles><title>` text.
    pub title: Option<String>,
    /// `<contributors><authors><author>` texts.
    pub authors: Vec<String>,
    /// `<dates><year>` as an integer.
    pub year: Option<i64>,
}

/// A parsed EndNote XML file.
#[derive(Debug)]
pub struct Endnote {
    /// `<record>` elements in document order.
    pub records: Vec<Record>,
}

fn text_between<'a>(src: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let a = src.find(open)? + open.len();
    let b = src[a..].find(close)? + a;
    Some(&src[a..b])
}

/// `name="..."` value inside an open tag.
fn attr(tag: &str, key: &str) -> Option<String> {
    let pat = format!("{key}=\"");
    let at = tag.find(&pat)? + pat.len();
    let end = tag[at..].find('"')? + at;
    Some(tag[at..end].to_string())
}

fn open_tag(src: &str, from: usize) -> Option<(usize, usize)> {
    let a = src[from..].find('<')? + from;
    let b = src[a..].find('>')? + a;
    Some((a, b + 1))
}

fn record(src: &str) -> Option<Record> {
    let mut r = Record::default();
    // ref-type
    if let Some(at) = src.find("<ref-type") {
        let (a, b) = open_tag(src, at)?;
        r.kind = attr(&src[a..b], "name");
    }
    // title
    if let Some(titles) = text_between(src, "<titles>", "</titles>") {
        r.title = text_between(titles, "<title>", "</title>")
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty());
    }
    // authors
    if let Some(auths) = text_between(src, "<authors>", "</authors>") {
        let mut rest = auths;
        while let Some(body) = text_between(rest, "<author>", "</author>") {
            r.authors.push(body.trim().to_string());
            let next = rest.find("</author>")? + "</author>".len();
            rest = &rest[next..];
        }
    }
    // year
    if let Some(dates) = text_between(src, "<dates>", "</dates>") {
        r.year = text_between(dates, "<year>", "</year>").and_then(|y| y.trim().parse().ok());
    }
    Some(r)
}

/// Parse a whole EndNote XML export. `None` without `<records>`; each
/// `<record>` body is scanned independently.
pub fn parse(data: &[u8]) -> Option<Endnote> {
    let src = std::str::from_utf8(data).ok()?;
    let body = text_between(src, "<records>", "</records>")
        .or_else(|| text_between(src, "<records >", "</records>"))?;
    let mut records = Vec::new();
    let mut rest = body;
    while let Some(at) = rest.find("<record") {
        // `<record>` vs `<records` — next char must not be 's'
        if rest.as_bytes().get(at + 7) == Some(&b's') {
            rest = &rest[at + 7..];
            continue;
        }
        let inner = text_between(rest, "<record>", "</record>")?;
        records.push(record(inner)?);
        let next = rest.find("</record>")? + "</record>".len();
        rest = &rest[next..];
    }
    Some(Endnote { records })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b"<xml><records>\
<record><ref-type name=\"Journal Article\">17</ref-type>\
<contributors><authors><author>Doe, J.</author><author>Roe, K.</author></authors></contributors>\
<titles><title>Alpha</title></titles><dates><year>2001</year></dates></record>\
<record><titles><title>Beta</title></titles></record>\
</records></xml>";

    #[test]
    fn parses() {
        let d = parse(DOC).unwrap();
        assert_eq!(d.records.len(), 2);
        let r = &d.records[0];
        assert_eq!(r.kind.as_deref(), Some("Journal Article"));
        assert_eq!(r.authors, vec!["Doe, J.", "Roe, K."]);
        assert_eq!(r.title.as_deref(), Some("Alpha"));
        assert_eq!(r.year, Some(2001));
        let r2 = &d.records[1];
        assert_eq!(r2.kind, None);
        assert_eq!(r2.year, None);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<xml></xml>").is_none()); // no records
        assert!(parse(&[0xFF]).is_none());
    }
}
