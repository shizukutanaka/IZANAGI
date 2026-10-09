//! W3C Widget `config.xml` の検出と構造カウント。
//!
//! `<widget>` ルート(`xmlns="http://www.w3.org/ns/widgets"`)と `<name>`/
//! `<description>`/`<author>`/`<content src=>`/`<icon src=>`/`<license>`/
//! `<access origin=>`/`<feature>`/`<param>`/`<preference>`/`<span>` 要素を識別する。
//!
//! ```
//! let c = izanagi_kit::widgetxml::parse(
//!     b"<widget xmlns=\"http://www.w3.org/ns/widgets\">\n  <name>app</name>\n  <content src=\"index.html\"/>\n  <access origin=\"*\"/>\n  <feature name=\"geolocation\"/>\n</widget>\n").unwrap();
//! assert!(c.entries >= 4);
//! assert!(izanagi_kit::widgetxml::detect(
//!     b"<widget xmlns=\"http://www.w3.org/ns/widgets\"><name>x</name><content src=\"i.html\"/></widget>"));
//! ```

use crate::textutil::strip_xml_comments;
/// コンテナ要素。
const CONTAINERS: &[&str] = &["widget"];

/// エントリ/リーフ要素。
const ENTRIES: &[&str] = &[
    "access",
    "author",
    "content",
    "description",
    "feature",
    "icon",
    "image",
    "license",
    "name",
    "param",
    "preference",
    "span",
];

/// 既知属性名。
const ATTRS: &[&str] = &[
    "dir=",
    "email=",
    "height=",
    "href=",
    "id=",
    "lang=",
    "name=",
    "origin=",
    "readonly=",
    "required=",
    "short=",
    "src=",
    "value=",
    "version=",
    "viewmodes=",
    "width=",
    "xmlns=",
];

/// 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// コンテナ開始タグ数(`<widget>`)。
    pub sections: usize,
    /// エントリ要素数。
    pub entries: usize,
    /// 既知属性出現数。
    pub attributes: usize,
    /// コメント行数。
    pub comments: usize,
    /// その他の行数。
    pub misc: usize,
}

fn tags_in_line(t: &str, f: &mut dyn FnMut(&str)) {
    let b = t.as_bytes();
    let mut i = 0usize;
    while i < b.len() {
        if b[i] == b'<' && i + 1 < b.len() && b[i + 1].is_ascii_alphabetic() {
            let mut j = i + 1;
            while j < b.len() && (b[j].is_ascii_alphanumeric() || b[j] == b'-' || b[j] == b'_') {
                j += 1;
            }
            f(&t[i + 1..j]);
            i = j;
        } else {
            i += 1;
        }
    }
}

fn attrs_in(t: &str) -> usize {
    ATTRS.iter().map(|a| t.matches(a).count()).sum()
}

/// `config.xml`(W3C widget)らしさを判定する。
pub fn detect(input: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(input) else {
        return false;
    };
    let text = strip_xml_comments(text);
    let mut hits = 0usize;
    let mut root = false;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("<!--") || t.starts_with("<?") {
            continue;
        }
        if t.starts_with("</") {
            continue;
        }
        tags_in_line(t, &mut |tag| {
            if tag == "widget" {
                root = true;
            }
            if CONTAINERS.contains(&tag) || ENTRIES.contains(&tag) {
                hits += 1;
            }
        });
        if root && hits >= 3 {
            return true;
        }
    }
    root && hits >= 2
}

/// 構造をカウントする。
pub fn parse(input: &[u8]) -> Option<Counts> {
    if !detect(input) {
        return None;
    }
    let text = strip_xml_comments(std::str::from_utf8(input).ok()?);
    let mut c = Counts {
        sections: 0,
        entries: 0,
        attributes: 0,
        comments: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with("<!--") || t.starts_with("<?") {
            c.comments += 1;
            continue;
        }
        if t.starts_with("</") {
            continue;
        }
        let mut n = 0usize;
        tags_in_line(t, &mut |tag| {
            if CONTAINERS.contains(&tag) {
                c.sections += 1;
                n += 1;
            } else if ENTRIES.contains(&tag) {
                c.entries += 1;
                n += 1;
            }
        });
        c.attributes += attrs_in(t);
        if n == 0 && attrs_in(t) == 0 && t.chars().any(|ch| ch.is_ascii_alphanumeric()) {
            c.misc += 1;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"<?xml version=\"1.0\"?>\n<widget xmlns=\"http://www.w3.org/ns/widgets\" id=\"x\" version=\"1.0\">\n  <name short=\"app\">My App</name>\n  <description>demo</description>\n  <author href=\"https://e.com\" email=\"a@b.c\">me</author>\n  <content src=\"index.html\"/>\n  <icon src=\"icon.png\" width=\"128\" height=\"128\"/>\n  <access origin=\"*\"/>\n  <feature name=\"geolocation\" required=\"false\"/>\n  <preference name=\"key\" value=\"v\" readonly=\"true\"/>\n  <license>MIT</license>\n</widget>\n";

    #[test]
    fn widgetxml() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 1);
        assert_eq!(c.entries, 9);
        assert_eq!(c.attributes, 16);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn not_widget() {
        assert!(!detect(b"<root><a/></root>\n"));
        assert!(parse(b"text\n").is_none());
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
