//! Sieve メールフィルタスクリプト(RFC 5228)の検出と構造カウント。
//!
//! `require`・`if`/`elsif`/`else` 制御・テスト・`fileinto`/`redirect` 等の
//! アクションを行頭キーワードで分類する。
//!
//! ```
//! let c = izanagi_kit::sievescript::parse(
//!     b"require [\"fileinto\"];\nif header :contains \"from\" \"boss\" {\n  fileinto \"work\";\n}\n").unwrap();
//! assert_eq!(c.requires, 1);
//! assert_eq!(c.actions, 1);
//! assert!(izanagi_kit::sievescript::detect(b"require \"reject\";"));
//! ```

use crate::textutil::strip_bom;
/// 既知 Sieve テスト名。
const TESTS: &[&str] = &[
    "address",
    "allof",
    "anyof",
    "body",
    "date",
    "envelope",
    "exists",
    "false",
    "header",
    "metadata",
    "not",
    "size",
    "string",
    "true",
    "valid_notify_method",
];
/// 既知 Sieve アクション。
const ACTIONS: &[&str] = &[
    "addflag",
    "addheader",
    "deleteheader",
    "discard",
    "ereject",
    "fileinto",
    "keep",
    "notify",
    "redirect",
    "reject",
    "removeflag",
    "set",
    "setflag",
    "stop",
    "vacation",
];
/// 制御キーワード。
const CONTROLS: &[&str] = &["if", "elsif", "else", "require", "forEveryPart", "break"];

/// 行が空白区切りで開始する最初の語。
fn head(line: &str) -> &str {
    line.split(|ch: char| ch.is_whitespace() || ch == '(' || ch == '{' || ch == ';')
        .next()
        .unwrap_or("")
}

/// Sieve 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `require` 行。
    pub requires: usize,
    /// `if`/`elsif`/`else` 行。
    pub controls: usize,
    /// 既知テスト名の出現(if/elsif 行内)。
    pub tests: usize,
    /// 既知アクションの行。
    pub actions: usize,
    /// `{` で開くブロック。
    pub blocks: usize,
    /// `#`/`//` コメント行。
    pub comments: usize,
    /// 分類不能行。
    pub misc: usize,
}

/// Sieve 固有の動詞(`if`/`elsif`/`set`/`stop`/`keep`/`header` 等は
/// 他言語でも現れるため除外)。最低1件要求。
const EXCLUSIVE_VERBS: &[&str] = &[
    "addflag",
    "addheader",
    "allof",
    "anyof",
    "deleteheader",
    "discard",
    "envelope",
    "ereject",
    "fileinto",
    "forEveryPart",
    "imap4flags",
    "notify",
    "redirect",
    "reject",
    "removeflag",
    "setflag",
    "vacation",
    "valid_notify_method",
];

/// b が Sieve スクリプトかどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let text = strip_bom(text);
    let mut req = 0;
    let mut other = 0;
    let mut exclusive = 0;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') || t.starts_with("//") {
            continue;
        }
        // `require [`/`require "x";` は Sieve 宣言。JS の `require(` は除く。
        if t.starts_with("require ") || t.starts_with("require\t") {
            req += 1;
            exclusive += 1;
        }
        let h = head(t);
        if CONTROLS.contains(&h) || ACTIONS.contains(&h) || TESTS.contains(&h) {
            other += 1;
            if EXCLUSIVE_VERBS.contains(&h) {
                exclusive += 1;
            }
        }
    }
    exclusive >= 1 && (req >= 1 || other >= 2)
}

/// Sieve スクリプトの構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let text = strip_bom(text);
    let mut c = Counts {
        requires: 0,
        controls: 0,
        tests: 0,
        actions: 0,
        blocks: 0,
        comments: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') || t.starts_with("//") {
            c.comments += 1;
            continue;
        }
        if t.contains('{') {
            c.blocks += t.matches('{').count();
        }
        let t = t.strip_prefix('}').unwrap_or(t).trim();
        if t.is_empty() {
            continue;
        }
        let h = head(t);
        if h == "require" {
            c.requires += 1;
        } else if h == "if" || h == "elsif" || h == "else" {
            c.controls += 1;
            for w in t.split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_') {
                if TESTS.contains(&w) {
                    c.tests += 1;
                }
            }
        } else if ACTIONS.contains(&h) {
            c.actions += 1;
        } else if TESTS.contains(&h) {
            // 複数行テストの継続行。
            for w in t.split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_') {
                if TESTS.contains(&w) {
                    c.tests += 1;
                }
            }
        } else {
            c.misc += 1;
        }
    }
    (c.requires + c.controls + c.actions >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# sieve\nrequire [\"fileinto\", \"vacation\"];\n\nif allof (header :contains \"from\" \"boss\",\n          size :over 100K) {\n  fileinto \"work\";\n  stop;\n} elsif address :domain :is \"to\" \"example.com\" {\n  redirect \"me@home\";\n} else {\n  keep;\n}\nvacation \"away\";\n";

    #[test]
    fn sieve() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.requires, 1);
        assert_eq!(c.controls, 3);
        assert_eq!(c.tests, 4);
        assert_eq!(c.actions, 5);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn not_sieve() {
        assert!(!detect(b"key = value\n[section]\n"));
        assert!(!detect(b"hello world\n"));
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
