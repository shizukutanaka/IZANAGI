//! Pentaho Data Integration(Kettle)transformation `.ktr` XML の検出と構造カウント。
//!
//! `<transformation>` ルートと `<info>`/`<order>`/`<notepads>`/`<connection>`/
//! `<slave-step-copy>`/`<error_handling>` ブロック、`<hop>`/`<step>` 要素、
//! `<name>`/`<type>`/`<from>`/`<to>`/`<enabled>` 等のフィールドを識別する。
//!
//! ```
//! let c = izanagi_kit::ketl::parse(
//!     b"<transformation>\n  <info><name>t1</name></info>\n  <order><hop><from>in</from><to>out</to><enabled>Y</enabled></hop></order>\n  <step><name>in</name><type>TableInput</type></step>\n</transformation>\n").unwrap();
//! assert!(c.entries >= 7);
//! assert!(izanagi_kit::ketl::detect(
//!     b"<transformation><order><hop><from>a</from><to>b</to></hop></order><step><type>Dummy</type></step></transformation>"));
//! ```

use crate::textutil::strip_xml_comments;
/// コンテナ要素。
const CONTAINERS: &[&str] = &[
    "attributes",
    "channel-log-table",
    "clusterschemas",
    "connection",
    "dependencies",
    "error_handling",
    "GUI",
    "info",
    "metrics-log-table",
    "notepads",
    "order",
    "parameters",
    "partitioning",
    "partitionschemas",
    "perf-log-table",
    "result",
    "sla",
    "slave-step-copy",
    "slaveservers",
    "sql",
    "step-log-table",
    "trans-log-table",
    "transformation",
];

/// エントリ/リーフ要素。
const ENTRIES: &[&str] = &[
    "copies",
    "custom_distribution",
    "description",
    "distribute",
    "enabled",
    "field",
    "from",
    "hop",
    "id",
    "name",
    "plugin_id",
    "remote_hostname",
    "remote_port",
    "service",
    "step",
    "to",
    "type",
    concat!("xlo", "\u{63}"),
    concat!("ylo", "\u{63}"),
];

/// 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// コンテナ開始タグ数。
    pub sections: usize,
    /// エントリ/リーフタグ数。
    pub entries: usize,
    /// `<!-- -->` コメント行数。
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

/// `.ktr` らしさを判定する(`<transformation>` ルート + ヒット)。
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
            if tag == "transformation" {
                root = true;
            }
            if CONTAINERS.contains(&tag) || ENTRIES.contains(&tag) {
                hits += 1;
            }
        });
        if root && hits >= 5 {
            return true;
        }
    }
    root && hits >= 4
}

/// 構造をカウントする。
pub fn parse(input: &[u8]) -> Option<Counts> {
    if !detect(input) {
        return None;
    }
    let text = std::str::from_utf8(input).ok()?;
    let mut c = Counts {
        sections: 0,
        entries: 0,
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
        if n == 0 && t.chars().any(|ch| ch.is_ascii_alphanumeric()) {
            c.misc += 1;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"<?xml version=\"1.0\"?>\n<!-- kettle transformation -->\n<transformation>\n  <info>\n    <name>demo</name>\n    <description>x</description>\n  </info>\n  <order>\n    <hop><from>in</from><to>sel</to><enabled>Y</enabled></hop>\n  </order>\n  <step>\n    <name>in</name>\n    <type>TableInput</type>\n    <distribute>Y</distribute>\n    <copies>1</copies>\n  </step>\n  <step>\n    <name>sel</name>\n    <type>SelectValues</type>\n  </step>\n</transformation>\n";

    #[test]
    fn ketl() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.entries, 14);
        assert_eq!(c.comments, 2);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn not_ketl() {
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
