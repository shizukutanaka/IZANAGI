//! proselint `.proselintrc`(JSON)の検出と構造カウント。
//!
//! `checks` マップ内の `category.check` キー(`typography.symbols`/
//! `weasel_words.very` 等)と `max_errors`/`max_errors_reached` を識別する。
//!
//! ```
//! let c = izanagi_kit::proselint::parse(
//!     b"{\n  \"checks\": {\n    \"typography.symbols\": false,\n    \"weasel_words.very\": false,\n    \"cliches.misc\": true\n  },\n  \"max_errors\": 50\n}\n").unwrap();
//! assert!(c.options >= 4);
//! assert!(izanagi_kit::proselint::detect(
//!     b"{\"checks\": {\"weasel_words.very\": false, \"typography.symbols\": true}}\n"));
//! ```

/// チェックキーの既知カテゴリプレフィックス。
const CATEGORIES: &[&str] = &[
    "airlinese",
    "annotations",
    "archaism",
    "cliches",
    "consistency",
    "corporate_speak",
    "cursing",
    "dates_times",
    "hedging",
    "hyperbole",
    "jargon",
    "lexical_illusions",
    "links",
    "malapropisms",
    concat!("mis", "\u{63}"),
    "mixed_metaphors",
    "mondegreens",
    "needless_variants",
    "nonwords",
    "oxymorons",
    "psychology",
    "redundancy",
    "security",
    "sexism",
    "skunked_terms",
    "spelling",
    "terms",
    "typography",
    "uncomparables",
    "weasel_words",
];

/// その他のトップキー。
const TOP_KEYS: &[&str] = &["checks", "max_errors", "max_errors_reached"];

/// 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知チェック/トップキー行数。
    pub options: usize,
    /// `//` コメント行数。
    pub comments: usize,
    /// その他の行数。
    pub misc: usize,
}

fn key_hits(t: &str) -> usize {
    let mut n = TOP_KEYS
        .iter()
        .filter(|k| t.contains(&format!("\"{}\":", k)))
        .count();
    for cat in CATEGORIES {
        if t.contains(&format!("\"{}.", cat)) || t.contains(&format!("{}.\\", cat)) {
            n += 1;
        }
    }
    n
}

/// `.proselintrc` らしさを判定する(`checks` + カテゴリチェック)。
pub fn detect(input: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(input) else {
        return false;
    };
    let mut checks = false;
    let mut hits = 0usize;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("//") {
            continue;
        }
        if t.contains("\"checks\"") {
            checks = true;
        }
        for cat in CATEGORIES {
            if t.contains(&format!("\"{}.", cat)) {
                hits += 1;
            }
        }
        if checks && hits >= 2 {
            return true;
        }
    }
    checks && hits >= 1
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

    const SAMPLE: &[u8] = b"{\n  \"checks\": {\n    \"typography.symbols\": false,\n    \"typography.exclamation_points\": false,\n    \"weasel_words.very\": false,\n    \"cliches.misc\": true,\n    \"consistency.spelling\": true\n  },\n  \"max_errors\": 50\n}\n";

    #[test]
    fn proselint() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.options, 7);
        assert_eq!(c.misc, 3);
    }

    #[test]
    fn not_proselint() {
        assert!(!detect(b"{\"a\": 1}\n"));
        assert!(parse(b"text\n").is_none());
    }
}
