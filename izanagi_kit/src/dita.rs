//! DITA XML document scanner.
//!
//! DITA topics and maps are XML files identifiable by either a `DOCTYPE`
//! line referencing `"-//OASIS//DTD DITA …"`, or a root element from the
//! DITA set (`topic`, `concept`, `task`, `reference`, `map`,
//! `bookmap`, `glossentry`, …) carrying `DITAArchVersion` or
//! `xmlns:dita` attributes.
//!
//! ```
//! let d = b"<?xml version=\"1\x2e0\"?>\
//! <!DOCTYPE topic PUBLIC \"-//OASIS//DTD DITA Topic//EN\" \"topic.dtd\">\
//! <topic id=\"t1\"><title>T</title></topic>";
//! let dt = izanagi_kit::dita::parse(d).unwrap();
//! assert_eq!(dt.root, "topic");
//! assert!(dt.doctype_dita);
//! assert_eq!(dt.title.as_deref(), Some("T"));
//! ```
//!
//! Reference: OASIS DITA 1.x specification — the topic-type element set
//! and the `DITAArchVersion` attribute convention.

/// Parsed DITA document properties.
#[derive(Debug, Clone, PartialEq)]
pub struct Dita {
    /// Root element name (`topic`, `map`, `concept`, …).
    pub root: String,
    /// `true` when a DITA `DOCTYPE` declaration is present.
    pub doctype_dita: bool,
    /// Declared `DITAArchVersion` attribute, if any.
    pub arch_version: Option<String>,
    /// First `<title>` contents, if present.
    pub title: Option<String>,
}

const ROOTS: &[&str] = &[
    "topic",
    "concept",
    "task",
    "reference",
    "troubleshooting",
    "map",
    "bookmap",
    "glossentry",
    "glossgroup",
    "ditaval",
    "subjectScheme",
    "learningContent",
    "machineryTask",
];

fn find_root(s: &str) -> Option<String> {
    let mut rest = s;
    loop {
        rest = rest.trim_start();
        if let Some(x) = rest.strip_prefix("<?") {
            rest = x.split_once("?>")?.1;
        } else if let Some(x) = rest.strip_prefix("<!--") {
            rest = x.split_once("-->")?.1;
        } else if let Some(x) = rest.strip_prefix("<!") {
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

fn attr(attrs: &str, key: &str) -> Option<String> {
    let start = attrs.find(key)? + key.len();
    let v = attrs.get(start..)?.strip_prefix('=')?;
    let v = v.trim_start_matches(['"', '\'']);
    let end = v.find(['"', '\''])?;
    Some(v[..end].to_string())
}

/// Parse a DITA document; `None` if no DITA markers are present.
pub fn parse(d: &[u8]) -> Option<Dita> {
    let text = core::str::from_utf8(d).ok()?;
    let doctype_dita = text.contains("//OASIS//") && text.contains("DITA");
    let root = find_root(text)?;
    let bare = root.rsplit(':').next()?;
    let known_root = ROOTS.contains(&bare);
    let root_open = text.find('<')?;
    let after_prolog = &text[root_open..];
    let tag_start = after_prolog.find(&root)?;
    let tag_end = after_prolog[tag_start..].find('>')? + tag_start;
    let attrs = &after_prolog[tag_start..tag_end];
    let arch_version = attr(attrs, "DITAArchVersion");
    // Require either a DITA doctype, a known root element, or an explicit
    // DITA attribute on the root.
    if !doctype_dita && !known_root && arch_version.is_none() && !attrs.contains("ditaarch") {
        return None;
    }
    if !doctype_dita && !known_root {
        return None;
    }
    let title = text
        .split("<title>")
        .nth(1)
        .and_then(|r| r.split("</title>").next())
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string());
    Some(Dita {
        root,
        doctype_dita,
        arch_version,
        title,
    })
}

/// `true` if the buffer looks like a DITA document.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b"<?xml version=\"1\x2e0\"?>\
<!DOCTYPE topic PUBLIC \"-//OASIS//DTD DITA Topic//EN\" \"topic.dtd\">\
<topic id=\"t1\"><title>T</title></topic>";

    #[test]
    fn parses() {
        let d = parse(DOC).unwrap();
        assert_eq!(d.root, "topic");
        assert!(d.doctype_dita);
        assert_eq!(d.title.as_deref(), Some("T"));
    }

    #[test]
    fn arch_attr() {
        let d = parse(b"<map DITAArchVersion=\"1\x2e3\"/>").unwrap();
        assert_eq!(d.root, "map");
        assert_eq!(d.arch_version.as_deref(), Some("1\x2e3"));
        assert!(!d.doctype_dita);
    }

    #[test]
    fn known_root_without_doctype() {
        assert_eq!(parse(b"<task id=\"x\"/>").unwrap().root, "task");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<html><body/></html>").is_none());
        assert!(parse(&[0xff]).is_none());
        assert!(parse(b"<book/>").is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(DOC));
        assert!(!detect(b"<topic"));
    }
}
