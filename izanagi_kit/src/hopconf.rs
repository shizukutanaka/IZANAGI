//! Apache Hop pipeline/workflow XML の検出と構造カウント。
//!
//! `<pipeline>`(transform: `<order>`/`<hop>`/`<transform>` + `<info>`)と
//! `<workflow>`(action: `<actions>`/`<action>` + `<hops>`/`<hop>`)両形式を識別する。
//!
//! ```
//! let c = izanagi_kit::hopconf::parse(
//!     b"<pipeline>\n  <info><name>p</name></info>\n  <order><hop><from>a</from><to>b</to><enabled>Y</enabled></hop></order>\n  <transform><name>a</name><type>TableInput</type></transform>\n</pipeline>\n").unwrap();
//! assert!(c.entries >= 6);
//! assert!(izanagi_kit::hopconf::detect(
//!     b"<workflow><actions><action><name>x</name><type>START</type></action></actions></workflow>"));
//! ```

/// コンテナ要素。
const CONTAINERS: &[&str] = &[
    "actions",
    "attributes",
    "custom_distribution",
    "dependencies",
    "error_handling",
    "GUI",
    "hops",
    "info",
    "metadata_injection",
    "notepads",
    "order",
    "parameters",
    "partitioning",
    "pipeline",
    "result",
    "sla",
    "transforms",
    "workflow",
];

/// エントリ/リーフ要素。
const ENTRIES: &[&str] = &[
    "action",
    "capgroup_size",
    "capture_transform_performance",
    "copies",
    "copies_string",
    "description",
    "distribute",
    "enabled",
    "evaluation",
    "field",
    "filename",
    "fname",
    "from",
    "hop",
    "name",
    "on_error",
    "on_main",
    "to",
    "transform",
    "type",
    "unconditional",
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
fn strip_comments(t: &str) -> String {
    let mut out = String::with_capacity(t.len());
    let mut rest = t;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        match rest[start + 4..].find("-->") {
            Some(end) => rest = &rest[start + 4 + end + 3..],
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Hop pipeline/workflow らしさを判定する。
pub fn detect(input: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(input) else {
        return false;
    };
    let text = strip_comments(text);
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
            if tag == "pipeline" || tag == "workflow" {
                root = true;
            }
            if CONTAINERS.contains(&tag) || ENTRIES.contains(&tag) {
                hits += 1;
            }
        });
        if root && hits >= 4 {
            return true;
        }
    }
    root && hits >= 3
}

/// 構造をカウントする。
pub fn parse(input: &[u8]) -> Option<Counts> {
    if !detect(input) {
        return None;
    }
    let text = strip_comments(std::str::from_utf8(input).ok()?);
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

    const SAMPLE: &[u8] = b"<pipeline>\n  <info>\n    <name>demo</name>\n    <capgroup_size>1</capgroup_size>\n    <capture_transform_performance>N</capture_transform_performance>\n  </info>\n  <order>\n    <hop><from>in</from><to>out</to><enabled>Y</enabled></hop>\n  </order>\n  <transform>\n    <name>in</name>\n    <type>TableInput</type>\n    <distribute>Y</distribute>\n  </transform>\n  <transform>\n    <name>out</name>\n    <type>TextFileOutput</type>\n  </transform>\n</pipeline>\n";

    #[test]
    fn hopconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.entries, 14);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn not_hop() {
        assert!(!detect(b"<root><a/></root>\n"));
        assert!(parse(b"text\n").is_none());
    }

    #[test]
    fn xml_comments_are_stripped() {
        let t = strip_comments(
            "<a><!-- hidden
<config -->x</a>",
        );
        assert_eq!(t, "<a>x</a>");
        let u = strip_comments("<a><!-- unterminated");
        assert_eq!(u, "<a>");
    }
}
