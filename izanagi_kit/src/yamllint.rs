//! yamllint 設定ファイル(`.yamllint`/`.yamllint.yaml`)の検出と構造カウント。
//!
//! `extends`/`rules`/`ignore`/`yaml-files`/`locale`/`ignore-from-file` トップキーと
//! `rules:` ブロック内の既知ルール名(`braces`/`line-length`/`truthy`/
//! `document-start`/`indentation`/`key-duplicates` 等)を識別する。
//!
//! ```
//! let c = izanagi_kit::yamllint::parse(
//!     b"extends: default\nrules:\n  line-length:\n    max: 120\n  truthy: disable\n").unwrap();
//! assert_eq!(c.sections, 2);
//! assert_eq!(c.rules, 2);
//! assert!(izanagi_kit::yamllint::detect(b"rules:\n  truthy: disable\n"));
//! ```

/// 既知トップレベルキー。
const TOP_KEYS: &[&str] = &[
    "extends",
    "ignore",
    "ignore-from-file",
    "locale",
    "rules",
    "yaml-files",
];

/// 既知ルール名。
const RULES: &[&str] = &[
    "anchors",
    "braces",
    "brackets",
    "colons",
    "commas",
    "comments",
    "comments-indentation",
    "document-end",
    "document-start",
    "empty-lines",
    "empty-values",
    "float-values",
    "hyphens",
    "indentation",
    "key-duplicates",
    "key-ordering",
    "line-length",
    "new-line-at-end-of-file",
    "new-lines",
    "octal-values",
    "quoted-strings",
    "spacing",
    "trailing-spaces",
    "truthy",
];

/// yamllint 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// トップレベルキー行。
    pub sections: usize,
    /// `rules:` 内の既知ルールエントリ。
    pub rules: usize,
    /// ルール内オプション(`max:`/`level:`/`forbid-*`/`allowed-*` 等の葉キー)。
    pub options: usize,
    /// `#` コメント行。
    pub comments: usize,
    /// 分類不能行(リスト要素・継続スカラ等)。
    pub misc: usize,
}

/// `key:` 先頭のキー名(行末 `:` or `: value`)。
fn yaml_key(t: &str) -> Option<&str> {
    let p = t.find(':')?;
    let k = t[..p].trim();
    if k.is_empty()
        || !k
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        None
    } else {
        Some(k)
    }
}
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// b が yamllint 設定かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let text = strip_bom(text);
    let mut hits = 0;
    let mut in_rules = false;
    for line in text.lines() {
        let t = line.split('#').next().map_or("", |s| s).trim_end();
        if t.trim().is_empty() {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        if indent == 0 {
            in_rules = t.trim() == "rules:";
            if yaml_key(t).is_some_and(|k| TOP_KEYS.contains(&k)) {
                hits += 1;
            }
        } else if in_rules && yaml_key(t).is_some_and(|k| RULES.contains(&k)) {
            hits += 1;
        }
    }
    hits >= 2
}

/// 構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let text = strip_bom(text);
    let mut c = Counts {
        sections: 0,
        rules: 0,
        options: 0,
        comments: 0,
        misc: 0,
    };
    let mut in_rules = false;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') {
            c.comments += 1;
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        if indent == 0 {
            in_rules = t == "rules:";
            if yaml_key(t).is_some_and(|k| TOP_KEYS.contains(&k)) {
                c.sections += 1;
            } else {
                c.misc += 1;
            }
            continue;
        }
        if let Some(k) = yaml_key(t.split('#').next().map_or(t, |s| s.trim())) {
            if in_rules && indent <= 2 && RULES.contains(&k) {
                c.rules += 1;
            } else {
                c.options += 1;
            }
            continue;
        }
        c.misc += 1;
    }
    (c.sections >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# yamllint\nextends: default\nignore: |\n  *.j2\nrules:\n  braces:\n    max-spaces-inside: 1\n  line-length:\n    max: 120\n    level: warning\n  truthy: disable\n  document-start:\n    present: false\n";

    #[test]
    fn yamllint() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.rules, 4);
        assert_eq!(c.options, 4);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 1);
    }

    #[test]
    fn not_yamllint() {
        assert!(!detect(b"key: value\nother: thing\n"));
        assert!(!detect(b"rules:\n  made-up-rule: enable\n"));
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
