//! revive 設定ファイル(`revive.toml`)の検出と構造カウント。
//!
//! トップレベルスカラ(`ignoreGeneratedHeader`/`severity`/`confidence`/
//! `errorCode`/`warningCode`/`enableAllRules`/`setExitStatus`/
//! `specifyDisableReason`/`maxOpenFiles` 等)と `[rule.<name>]` テーブル、
//! テーブル内 `arguments`/`severity` を識別する。
//!
//! ```
//! let c = izanagi_kit::reviveconf::parse(
//!     b"severity = \"warning\"\nconfidence = 0.8\n\n[rule.blank-imports]\n[rule.var-naming]\n").unwrap();
//! assert_eq!(c.options, 2);
//! assert_eq!(c.rules, 2);
//! assert!(izanagi_kit::reviveconf::detect(b"enableAllRules = true\n[rule.exported]\n"));
//! ```

use crate::textutil::strip_bom;
/// 既知トップレベルスカラキー。
const TOP_KEYS: &[&str] = &[
    "confidence",
    "directives",
    "enableAllRules",
    "errorCode",
    "ignoreGeneratedHeader",
    "maxOpenFiles",
    "severity",
    "setExitStatus",
    "specifyDisableReason",
    "warningCode",
];

/// 既知ルール名(抜粋)。
const RULES: &[&str] = &[
    "add-constant",
    "arguments-limit",
    "banned-characters",
    "bare-return",
    "blank-imports",
    "bool-literal-in-expr",
    "call-to-g\u{63}",
    "cognitive-complexity",
    "comment-spacings",
    "confusing-naming",
    "confusing-results",
    "constant-logical-expr",
    "context-as-argument",
    "context-keys-type",
    "datarace",
    "deep-exit",
    "defer",
    "dot-imports",
    "duplicated-imports",
    "early-return",
    "empty-block",
    "empty-lines",
    "enforce-map-style",
    "enforce-repeated-arg-type-style",
    "enforce-slice-style",
    "error-naming",
    "error-return",
    "error-strings",
    "errorf",
    "exported",
    "file-header",
    "flag-parameter",
    "function-length",
    "function-result-limit",
    "get-return",
    "identical-branches",
    "if-return",
    "import-naming",
    "import-shadowing",
    "increment-decrement",
    "indent-error-flow",
    "line-length-limit",
    "max-public-structs",
    "modifies-parameter",
    "modifies-value-receiver",
    "nested-structs",
    "optimize-operands-order",
    "package-comments",
    "range",
    "range-val-address",
    "range-val-in-closure",
    "receiver-naming",
    "redefines-builtin-id",
    "redundant-import-alias",
    "string-format",
    "string-of-int",
    "struct-tag",
    "superfluous-else",
    "time-equal",
    "time-naming",
    "unconditional-recursion",
    "unexported-naming",
    "unexported-return",
    "unhandled-error",
    "unnecessary-stmt",
    "unreachable-code",
    "unused-parameter",
    "unused-receiver",
    "use-any",
    "useless-break",
    "var-declaration",
    "var-naming",
    "waitgroup-by-value",
];

/// revive 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// トップレベル `key = value` 代入。
    pub options: usize,
    /// `[rule.<name>]` テーブル(既知名のみカウント)。
    pub rules: usize,
    /// テーブル内設定行(`arguments`/`severity` 等)。
    pub settings: usize,
    /// `#` コメント行。
    pub comments: usize,
    /// 分類不能行(未知キー・配列継続等)。
    pub misc: usize,
}

/// `key = value` のキー部分。
fn kv_key(t: &str) -> Option<&str> {
    let p = t.find('=')?;
    let k = t[..p].trim().trim_matches('"').trim_matches('\'');
    if k.is_empty()
        || !k
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        None
    } else {
        Some(k)
    }
}

/// `[rule.<name>]` テーブル名。
fn rule_table(t: &str) -> Option<&str> {
    let inner = t.strip_prefix('[')?.strip_suffix(']')?;
    let name = inner.strip_prefix("rule.")?;
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

/// b が revive.toml かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let text = strip_bom(text);
    let mut hits = 0;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if rule_table(t).is_some() || kv_key(t).is_some_and(|k| TOP_KEYS.contains(&k)) {
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
        options: 0,
        rules: 0,
        settings: 0,
        comments: 0,
        misc: 0,
    };
    let mut in_rule = false;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if let Some(name) = rule_table(t) {
            in_rule = true;
            if RULES.contains(&name) {
                c.rules += 1;
            } else {
                c.misc += 1;
            }
            continue;
        }
        if t.starts_with('[') && t.ends_with(']') {
            in_rule = false;
            c.misc += 1;
            continue;
        }
        if let Some(k) = kv_key(t) {
            if in_rule {
                c.settings += 1;
            } else if TOP_KEYS.contains(&k) {
                c.options += 1;
            } else {
                c.misc += 1;
            }
            continue;
        }
        c.misc += 1;
    }
    (c.options + c.rules >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# revive\nignoreGeneratedHeader = false\nseverity = \"warning\"\nconfidence = 0.8\nerrorCode = 0\nwarningCode = 0\nenableAllRules = true\n\n[rule.blank-imports]\n[rule.context-as-argument]\n[rule.var-naming]\n  arguments = [[\"ID\"], [\"VM\"]]\n[rule.exported]\n  severity = \"error\"\n";

    #[test]
    fn reviveconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.options, 6);
        assert_eq!(c.rules, 4);
        assert_eq!(c.settings, 2);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn not_revive() {
        assert!(!detect(b"[package]\nname = \"x\"\n"));
        assert!(!detect(b"foo = 1\n"));
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
