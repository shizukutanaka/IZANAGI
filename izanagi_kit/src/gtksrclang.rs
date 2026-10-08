//! GtkSourceView language definition (`.lang`) parser.
//!
//! Detects GtkSourceView `.lang` files by their `<language id="...">` root
//! with `<metadata>`/`<styles>`/`<definitions>` blocks, `<context>` rules
//! containing `<match>`/`<start>`/`<end>`/`<include>`/`<keyword>`/
//! `<sub-pattern>`/`<context ref=`-style tags and `<style id=`/
//! `map-to=`/`<default-regex-options>` attributes.
//!
//! ```
//! let b = b"<language id=\"demo\" name=\"Demo\" version=\"2.0\">\n<metadata>\n<property name=\"mimetypes\">text/x-demo</property>\n</metadata>\n<definitions>\n<context id=\"demo\">\n<include><context ref=\"def:comment\"/></include>\n</context>\n</definitions>\n</language>\n";
//! assert!(izanagi_kit::gtksrclang::detect(b));
//! let c = izanagi_kit::gtksrclang::Gtksrc::parse(b).unwrap();
//! assert!(c.keys >= 3);
//! ```

use crate::textutil::strip_xml_comments;
/// Parsed GtkSourceView .lang summary.
#[derive(Debug, Clone)]
pub struct Gtksrc {
    /// Recognized tag/attribute occurrences.
    pub keys: usize,
    /// Container tags (`<language`/`<metadata>`/`<property`/`<styles>`/`<style`/`<definitions>`/`<context`/`<include>`/`<default-regex-options>`).
    pub container_keys: usize,
    /// Matcher tags (`<match>`/`<start>`/`<end>`/`<keyword>`/`<sub-pattern>`/`<extended>`/`<simple>`/`<match id=`).
    pub match_keys: usize,
    /// Attribute/esc keys (`ref=`/`style-ref=`/`id=`/`name=`/`map-to=`/`where=`/`class=`/`%{x}`/`def:`.`_name`/`<escape>`/`<error>`/`<insensitive>`/`<regex>`/`<suffix>`/`<prefix>`/`line-continue`).
    pub attr_keys: usize,
    /// `<!-- -->` comment lines.
    pub comments: usize,
}

/// Container tags.
const CONTAINER_KEYS: &[&str] = &[
    "<language",
    "<metadata>",
    "<property",
    "<styles>",
    "<style ",
    "<definitions>",
    "<context ",
    "<context>",
    "<include>",
    "<include>",
    "<default-regex-options",
];

/// Matcher tags.
const MATCH_KEYS: &[&str] = &[
    "<match>",
    "<match ",
    "<start>",
    "<end>",
    "<keyword>",
    "<sub-pattern",
    "<extended>",
    "<simple>",
];

/// Attribute/esc keys.
const ATTR_KEYS: &[&str] = &[
    "ref=\"",
    "style-ref=",
    "id=\"",
    "map-to=",
    "where=",
    "class=",
    "%{",
    "def:",
    "_name",
    "<escape>",
    "<insensitive>",
    "<regex",
    "<suffix>",
    "<prefix>",
    "line-continue",
];

/// Anchor tokens.
const ANCHORS: &[&str] = &["<language", "<context", "def:"];

/// Distinctive keys for detection.
const ALL: &[&str] = &[
    "<language",
    "<metadata>",
    "<styles>",
    "<definitions>",
    "<context",
    "style-ref=",
    "map-to=",
    "def:",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(k)
}

/// Detect a GtkSourceView .lang file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_xml_comments(t);
    let hits = ALL.iter().filter(|k| key_present(&t, k)).count();
    let anchor = ANCHORS.iter().any(|a| t.contains(a));
    hits >= 2 && anchor
}

impl Gtksrc {
    /// Count categories in a .lang file. Returns `None` when the input
    /// does not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            container_keys: 0,
            match_keys: 0,
            attr_keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with("<!--") {
                c.comments += 1;
            }
        }
        for k in CONTAINER_KEYS {
            c.container_keys += t.matches(k).count();
        }
        for k in MATCH_KEYS {
            c.match_keys += t.matches(k).count();
        }
        for k in ATTR_KEYS {
            c.attr_keys += t.matches(k).count();
        }
        c.keys = c.container_keys + c.match_keys + c.attr_keys;
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"<!-- lang -->\n<language id=\"demo\" _name=\"Demo\" version=\"2.0\">\n<metadata>\n<property name=\"mimetypes\">text/x-demo</property>\n<property name=\"globs\">*.demo</property>\n</metadata>\n<styles><style id=\"comment\" _name=\"Comment\" map-to=\"def:comment\"/></styles>\n<definitions>\n<context id=\"demo\">\n<include><context ref=\"def:comment\"/></include>\n</context>\n<context id=\"str\" style-ref=\"string\">\n<start>\"</start><end>\"</end>\n</context>\n<context id=\"kw\" style-ref=\"keyword\">\n<keyword>for</keyword><keyword>while</keyword>\n</context>\n</definitions>\n</language>\n";
        assert!(detect(b));
        let c = Gtksrc::parse(b).unwrap();
        assert_eq!(c.comments, 1);
        assert!(c.container_keys >= 5);
        assert!(c.match_keys >= 3);
        assert!(c.attr_keys >= 4);
        assert!(c.keys >= 12);
    }

    #[test]
    fn rejects_plain_xml() {
        assert!(!detect(b"<root><a>1</a></root>"));
        assert!(Gtksrc::parse(b"a = b\n").is_none());
    }

    #[test]
    fn xml_comments_are_stripped() {
        let t = strip_xml_comments(
            "<a><!-- hidden
<config -->x</a>",
        );
        assert_eq!(t, "<a>x</a>");
        let u = strip_xml_comments("<a><!-- unterminated");
        assert_eq!(u, "<a>");
    }
}
