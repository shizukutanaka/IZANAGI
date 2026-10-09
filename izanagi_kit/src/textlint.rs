//! textlint `.textlintrc`(JSON)の検出と構造カウント。
//!
//! `rules:`/`filters:`/`rulePaths`/`severity`/`plugins`/`presets`/`overrides`
//! トップキーと、既知ルール/フィルタ名を識別する。
//!
//! ```
//! let c = izanagi_kit::textlint::parse(
//!     b"{\n  \"rules\": {\n    \"preset-ja-technical-writing\": true,\n    \"no-todo\": {\"severity\": \"warning\"}\n  },\n  \"filters\": {\n    \"comments\": true\n  }\n}\n").unwrap();
//! assert!(c.options >= 4);
//! assert!(izanagi_kit::textlint::detect(
//!     b"{\"rules\": {\"no-todo\": true, \"no-dead-link\": true}}\n"));
//! ```

use crate::textutil::strip_bom;
/// トップレベル既知キー。
const TOP_KEYS: &[&str] = &[
    "filters",
    "plugins",
    "presets",
    "rulePaths",
    "rules",
    "rulesBaseDirectory",
    "severity",
    "overrides",
];

/// 既知ルール/フィルタ名(サンプル)。
const RULE_NAMES: &[&str] = &[
    "comments",
    "no-dead-link",
    "no-todo",
    "preset-ja-technical-writing",
    "preset-japanese",
    "spellcheck-tech-word",
];

/// 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知トップ/ルールキー出現行数。
    pub options: usize,
    /// `//` コメント行数(JSONC)。
    pub comments: usize,
    /// その他の行数。
    pub misc: usize,
}

fn key_hits(t: &str) -> usize {
    let mut n = 0usize;
    for k in TOP_KEYS.iter().chain(RULE_NAMES.iter()) {
        if t.contains(&format!("\"{}\":", k)) || t.contains(&format!("{k}:")) {
            n += 1;
        }
    }
    n
}

/// `"key"` の直後に(空白/改行を挟んでも)`:` が来るか。
fn has_key(t: &str, key: &str) -> bool {
    let quoted = format!("\"{}\"", key);
    let mut rest = t;
    while let Some(i) = rest.find(&quoted) {
        let after = &rest[i + quoted.len()..];
        if after.trim_start().starts_with(':') {
            return true;
        }
        rest = &rest[i + 1..];
    }
    false
}

/// `.textlintrc` らしさを判定する(`rules`/`filters` トップキー必須)。
///
/// 非コメント行を連結してから判定するので、文字列値中の
/// `"rules"` や `//` コメント中のキー語では検出しない。
pub fn detect(input: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(input) else {
        return false;
    };
    let text = strip_bom(text);
    let mut joined = String::with_capacity(text.len());
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("//") {
            continue;
        }
        joined.push_str(t);
        joined.push('\n');
    }
    TOP_KEYS.iter().any(|k| has_key(&joined, k)) && has_key(&joined, "rules")
}

/// 構造をカウントする。
pub fn parse(input: &[u8]) -> Option<Counts> {
    if !detect(input) {
        return None;
    }
    let text = std::str::from_utf8(input).ok()?;
    let text = strip_bom(text);
    let mut c = Counts {
        options: 0,
        comments: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with("//") {
            c.comments += 1;
            continue;
        }
        if key_hits(t) > 0 {
            c.options += 1;
        } else {
            c.misc += 1;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"{\n  \"filters\": {\n    \"comments\": true\n  },\n  \"rules\": {\n    \"preset-ja-technical-writing\": true,\n    \"no-todo\": true,\n    \"no-dead-link\": { \"severity\": \"warning\" },\n    \"spellcheck-tech-word\": true\n  }\n}\n";

    #[test]
    fn textlint() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.options, 7);
        assert_eq!(c.misc, 4);
    }

    #[test]
    fn not_textlint() {
        assert!(!detect(b"{\"a\": 1}\n"));
        assert!(parse(b"text\n").is_none());
    }

    #[test]
    fn rules_marker_must_be_a_key() {
        // `"rules"` inside a string value or a `//` comment must not trigger.
        assert!(!detect(b"{\"note\": \"rules\", \"filters\": {}}\n"));
        assert!(!detect(
            b"// \"rules\": {}\n// \"filters\": {}\n{\"a\": 1}\n"
        ));
        // Whitespace between key and colon still counts.
        assert!(detect(b"{\"rules\" : {\"no-todo\": true}}\n"));
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
