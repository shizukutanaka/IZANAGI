//! DocBook XML document scanner.
//!
//! DocBook documents are XML files whose root element is one of `book`,
//! `article`, `chapter`, `section`, `set`, `part`, `appendix`,
//! `preface`, or `reference`, optionally namespaced under
//! `http://docbook.org/ns/docbook` (DocBook 5) or carrying a
//! `version`/`role` attribute (DocBook 3/4 SGML heritage).
//!
//! ```
//! let d = b"<?xml version=\"1\x2e0\"?>\
//!     <book xmlns=\"http://docbook.org/ns/docbook\" version=\"5\x2e0\">\
//!     <title>T</title></book>";
//! let db = izanagi_kit::docbook::parse(d).unwrap();
//! assert_eq!(db.root, "book");
//! assert!(db.namespaced);
//! assert_eq!(db.title.as_deref(), Some("T"));
//! ```
//!
//! Reference: DocBook TDG (OASIS DocBook 5.x specification) — the
//! `book`/`article`/`chapter` root element set and the
//! `http://docbook.org/ns/docbook` namespace.

/// Parsed DocBook document properties.
#[derive(Debug, Clone, PartialEq)]
pub struct Docbook {
    /// Root element name (`book`, `article`, `chapter`, …).
    pub root: String,
    /// `true` when the root declares the DocBook 5 namespace.
    pub namespaced: bool,
    /// Declared `version` attribute on the root element, if any.
    pub version: Option<String>,
    /// First `<title>` contents anywhere in the document, if present.
    pub title: Option<String>,
}

const ROOTS: &[&str] = &[
    "book",
    "article",
    "chapter",
    "section",
    "set",
    "part",
    "appendix",
    "preface",
    "reference",
    "sect1",
    "simplesect",
];

fn find_root(s: &str) -> Option<String> {
    // Skip the XML prolog and comments/DOCTYPE, then read the first tag.
    let mut rest = s;
    loop {
        rest = rest.trim_start();
        if let Some(x) = rest.strip_prefix("<?") {
            rest = x.split_once("?>")?.1;
        } else if let Some(x) = rest.strip_prefix("<!--") {
            rest = x.split_once("-->")?.1;
        } else if let Some(x) = rest.strip_prefix("<!") {
            // <!DOCTYPE …> — no internal subset handling needed.
            rest = x.split_once('>')?.1;
        } else {
            break;
        }
    }
    let tag = rest.strip_prefix('<')?;
    let name: String = tag
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == ':' || *c == '_')
        .collect();
    if name.is_empty() {
        return None;
    }
    Some(name)
}

/// Parse a DocBook document; `None` if the root is not a DocBook element.
pub fn parse(d: &[u8]) -> Option<Docbook> {
    let text = core::str::from_utf8(d).ok()?;
    let root = find_root(text)?;
    let bare = root.rsplit(':').next()?;
    if !ROOTS.contains(&bare) {
        return None;
    }
    // The root tag's attribute block ends at the first `>` after its name.
    let root_open = text.find('<')?;
    let after_prolog = &text[root_open..];
    let tag_start = after_prolog.find(&root)?;
    let tag_end = after_prolog[tag_start..].find('>')? + tag_start;
    let attrs = &after_prolog[tag_start..tag_end];
    let namespaced = attrs.contains("docbook.org/ns/docbook");
    let version = attrs.split("version=").nth(1).and_then(|v| {
        let v = v.trim_start_matches(['"', '\'']);
        let end = v.find(['"', '\''])?;
        Some(v[..end].to_string())
    });
    let title = text
        .split("<title>")
        .nth(1)
        .and_then(|r| r.split("</title>").next())
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string());
    Some(Docbook {
        root,
        namespaced,
        version,
        title,
    })
}

/// `true` if the buffer looks like a DocBook document.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b"<?xml version=\"1\x2e0\"?>\
<book xmlns=\"http://docbook.org/ns/docbook\" version=\"5\x2e0\">\
<title>T</title><chapter><title>C</title></chapter></book>";

    #[test]
    fn parses() {
        let d = parse(DOC).unwrap();
        assert_eq!(d.root, "book");
        assert!(d.namespaced);
        assert_eq!(d.version.as_deref(), Some("5\x2e0"));
        assert_eq!(d.title.as_deref(), Some("T"));
    }

    #[test]
    fn unnamespaced_article() {
        let d = parse(b"<article><title>Hi</title></article>").unwrap();
        assert_eq!(d.root, "article");
        assert!(!d.namespaced);
    }

    #[test]
    fn doctype_skipped() {
        let d = parse(b"<!DOCTYPE book><book/>").unwrap();
        assert_eq!(d.root, "book");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<html><body/></html>").is_none());
        assert!(parse(&[0xff, 0x00]).is_none());
        assert!(parse(b"<book").is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(DOC));
        assert!(!detect(b"<article"));
    }
}
