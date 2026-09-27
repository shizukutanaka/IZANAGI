//! Minimal reader for JATS article metadata (`<article-meta>` inside
//! `<front>`): DOI/PMID `<article-id pub-id-type="...">`, the
//! `<article-title>`, and `<contrib contrib-type="author">` names
//! (`<surname>` + `<given-names>`). Self-contained tag scanning.
//!
//! ```
//! use izanagi_kit::jats::parse;
//!
//! let j = parse(
//!     b"<article><front><article-meta>\
//!        <article-id pub-id-type=\"doi\">10.1/x</article-id>\
//!        <title-group><article-title>On parsing</article-title></title-group>\
//!        <contrib-group><contrib contrib-type=\"author\">\
//!        <name><surname>Doe</surname><given-names>Jane</given-names></name>\
//!        </contrib></contrib-group></article-meta></front></article>",
//! )
//! .unwrap();
//! assert_eq!(j.title.as_deref(), Some("On parsing"));
//! assert_eq!(j.id("doi").unwrap(), "10.1/x");
//! assert_eq!(j.authors[0].surname, "Doe");
//! ```

/// A `<contrib>` author name.
#[derive(Debug)]
pub struct Author {
    /// `<surname>` text.
    pub surname: String,
    /// `<given-names>` (absent for `collab`-style contribs).
    pub given: Option<String>,
}

/// Parsed JATS `<article-meta>`.
#[derive(Debug)]
pub struct Jats {
    /// `<article-title>` text (markup inside stripped).
    pub title: Option<String>,
    /// `<article-id pub-id-type="X">value</article-id>` pairs.
    pub ids: Vec<(String, String)>,
    /// `<contrib contrib-type="author">` names.
    pub authors: Vec<Author>,
}

impl Jats {
    /// First `<article-id>` whose `pub-id-type` equals `kind` (`doi`, `pmid`, ...).
    pub fn id(&self, kind: &str) -> Option<&str> {
        self.ids
            .iter()
            .find(|(k, _)| k == kind)
            .map(|(_, v)| v.as_str())
    }
}

fn text_between<'a>(src: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let a = src.find(open)? + open.len();
    let b = src[a..].find(close)? + a;
    Some(&src[a..b])
}

/// Strip `<...>` tags from an element body.
fn strip(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut rest = src;
    while let Some(a) = rest.find('<') {
        out.push_str(&rest[..a]);
        let b = match rest[a..].find('>') {
            Some(b) => a + b + 1,
            None => return out,
        };
        rest = &rest[b..];
    }
    out.push_str(rest);
    out
}

fn attr(tag: &str, key: &str) -> Option<String> {
    let pat = format!("{key}=\"");
    let at = tag.find(&pat)? + pat.len();
    let end = tag[at..].find('"')? + at;
    Some(tag[at..end].to_string())
}

/// Parse a JATS document (`<article-meta>` required). `None` without it.
pub fn parse(data: &[u8]) -> Option<Jats> {
    let src = std::str::from_utf8(data).ok()?;
    let meta = text_between(src, "<article-meta>", "</article-meta>")?;
    let mut j = Jats {
        title: None,
        ids: Vec::new(),
        authors: Vec::new(),
    };
    j.title = text_between(meta, "<article-title>", "</article-title>")
        .map(strip)
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty());
    // article-id elements (self-closing-safe scan)
    let mut rest = meta;
    while let Some(a) = rest.find("<article-id") {
        let close = rest[a..].find('>')? + a;
        let tag = &rest[a..close + 1];
        let kind = match attr(tag, "pub-id-type") {
            Some(k) => k,
            None => {
                rest = &rest[close + 1..];
                continue;
            }
        };
        let inner = match text_between(&rest[close + 1..], "", "</article-id>") {
            Some(t) => t,
            None => break,
        };
        j.ids.push((kind, strip(inner).trim().to_string()));
        let skip = close + 1 + inner.len() + "</article-id>".len();
        rest = &rest[skip..];
    }
    // authors
    let mut rest = meta;
    while let Some(a) = rest.find("<contrib") {
        let close = rest[a..].find('>')? + a;
        let tag = &rest[a..close + 1];
        if attr(tag, "contrib-type").as_deref() != Some("author") {
            rest = &rest[close + 1..];
            continue;
        }
        let inner = match text_between(&rest[close + 1..], "", "</contrib>") {
            Some(t) => t,
            None => break,
        };
        if let Some(n) = text_between(inner, "<name>", "</name>") {
            let surname = text_between(n, "<surname>", "</surname>")
                .map(|s| s.trim().to_string())
                .unwrap_or_default();
            let given = text_between(n, "<given-names>", "</given-names>")
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            j.authors.push(Author { surname, given });
        }
        let skip = close + 1 + inner.len() + "</contrib>".len();
        rest = &rest[skip..];
    }
    Some(j)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b"<article><front><article-meta>\
<article-id pub-id-type=\"doi\">10.1000/x</article-id>\
<article-id pub-id-type=\"pmid\">123</article-id>\
<title-group><article-title>Deep <italic>learning</italic></article-title></title-group>\
<contrib-group><contrib contrib-type=\"author\"><name><surname>Doe</surname>\
<given-names>Jane</given-names></name></contrib>\
<contrib contrib-type=\"editor\"><name><surname>Ed</surname></name></contrib>\
</contrib-group></article-meta></front></article>";

    #[test]
    fn parses() {
        let j = parse(DOC).unwrap();
        assert_eq!(j.title.as_deref(), Some("Deep learning")); // markup stripped
        assert_eq!(j.id("doi"), Some("10.1000/x"));
        assert_eq!(j.id("pmid"), Some("123"));
        assert_eq!(j.id("issn"), None);
        assert_eq!(j.authors.len(), 1); // editor skipped
        assert_eq!(j.authors[0].given.as_deref(), Some("Jane"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<article></article>").is_none()); // no article-meta
        assert!(parse(&[0xFF, 0x00]).is_none());
    }
}
