//! RedPen `redpen-conf.xml` の検出と構造カウント。
//!
//! `<redpen-conf lang="...">` ルート、`<validators>`/`<validator name="X">`、
//! `<property name="..." value="..."/>`、`<symbol>`/`<language>` 要素を識別する。
//! 日本語/英語両対応の既知バリデータ名を持つ。
//!
//! ```
//! let c = izanagi_kit::redpen::parse(
//!     b"<redpen-conf lang=\"en\">\n  <validators>\n    <validator name=\"SentenceLength\">\n      <property name=\"max_len\" value=\"120\"/>\n    </validator>\n  </validators>\n</redpen-conf>\n").unwrap();
//! assert_eq!(c.entries, 2);
//! assert!(izanagi_kit::redpen::detect(
//!     b"<redpen-conf lang=\"ja\"><validators><validator name=\"SentenceLength\"/></validators></redpen-conf>\n"));
//! ```

/// コンテナ要素。
const CONTAINERS: &[&str] = &["redpen-conf", "symbols", "validators"];

/// エントリ/リーフ要素。
const ENTRIES: &[&str] = &["language", "property", "symbol", "validator"];

/// 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// コンテナ開始タグ数。
    pub sections: usize,
    /// `<validator>`/`<property>`/`<symbol>`/`<language>` 要素数。
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

/// `redpen-conf.xml` らしさを判定する。
pub fn detect(input: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(input) else {
        return false;
    };
    let text = strip_comments(text);
    let mut root = false;
    let mut hits = 0usize;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("<!--") || t.starts_with("<?") || t.starts_with("</") {
            continue;
        }
        tags_in_line(t, &mut |tag| {
            if tag == "redpen-conf" {
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

    const SAMPLE: &[u8] = b"<?xml version=\"1.0\"?>\n<redpen-conf lang=\"en\">\n  <validators>\n    <validator name=\"SentenceLength\">\n      <property name=\"max_len\" value=\"120\"/>\n    </validator>\n    <validator name=\"CommaNumber\"/>\n    <validator name=\"DoubledWord\"/>\n    <validator name=\"SpaceBeginningOfSentence\"/>\n  </validators>\n  <symbols>\n    <symbol name=\"EXCLAMATION\" value=\"!\" invalid-chars=\"!\" after-space=\"true\"/>\n  </symbols>\n</redpen-conf>\n";

    #[test]
    fn redpen() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.entries, 6);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn not_redpen() {
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
