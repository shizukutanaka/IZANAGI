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

/// `.textlintrc` らしさを判定する(`rules`/`filters` トップキー必須)。
pub fn detect(input: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(input) else {
        return false;
    };
    let mut hits = 0usize;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("//") {
            continue;
        }
        hits += TOP_KEYS
            .iter()
            .filter(|k| t.contains(&format!("\"{}\":", k)))
            .count();
        if hits >= 1 && text.contains("\"rules\"") {
            return true;
        }
    }
    false
}

/// 構造をカウントする。
pub fn parse(input: &[u8]) -> Option<Counts> {
    if !detect(input) {
        return None;
    }
    let text = std::str::from_utf8(input).ok()?;
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
}
