//! `twpol.txt` / `tw.cfg` (Tripwire) 検出モジュール。
//!
//! Tripwire のポリシーファイルは `パス -> $(MASK) (オプション);`
//! 形式のルール、`@@section`/`@@ifhost`/`@@end` ディレクティブ、
//! `RULENAME = {` のルールセクション定義で構成される。
//!
//! ```
//! let b = br#"@@section GLOBAL
//! TWROOT = "/usr/sbin";
//! @@section FS
//! SEC_CRIT = $(IgnoreNone)-SHa;
//! /etc -> $(SEC_CRIT);
//! /bin -> $(SEC_CRIT) (recurse=0);
//! "#;
//! let c = izanagi_kit::tripwireconf::parse(b);
//! assert!(izanagi_kit::tripwireconf::detect(b));
//! assert_eq!(c.rules, 2);
//! ```

fn is_arrow_rule(t: &str) -> bool {
    // /path -> $(MASK) (opts); または name -> $(MASK);
    t.contains("->") && t.contains("$(") && t.trim_end_matches(';').contains('$')
}

fn is_directive(t: &str) -> bool {
    t.starts_with("@@") || (t.contains("$(") && t.ends_with(';')) || is_arrow_rule(t)
}

/// `b` が Tripwire ポリシーに見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut arrows = 0usize;
    let mut stmts = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') {
            continue;
        }
        if is_arrow_rule(tr) {
            arrows += 1;
            stmts += 1;
        } else if is_directive(tr) {
            stmts += 1;
        }
    }
    arrows >= 1 && stmts >= 3
}

/// Tripwire ポリシーの統計。
#[derive(Debug, Default, Clone)]
pub struct TripwireConf {
    /// `->` ルール行数。
    pub rules: usize,
    /// ディレクティブ/変数定義行数。
    pub statements: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を Tripwire ポリシーとして統計する。
pub fn parse(b: &[u8]) -> TripwireConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = TripwireConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if is_arrow_rule(tr) {
            c.rules += 1;
            c.statements += 1;
        } else if is_directive(tr) {
            c.statements += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"@@section GLOBAL
TWROOT = "/usr/sbin";
SEC_CRIT = $(IgnoreNone)-SHa;
/etc -> $(SEC_CRIT);
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.rules, 1);
    }

    #[test]
    fn detects_multi_rules() {
        let b = br#"@@section FS
SEC = $(IgnoreAll)-a;
/bin -> $(SEC);
/sbin -> $(SEC);
"#;
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"/etc -> $(SEC);\n"));
        assert!(!detect(b"foo -> bar\nbaz -> qux\nquux -> corge\n"));
        assert!(!detect(b"@@section GLOBAL\nx = 1;\ny = 2;\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.rules, 0);
    }
}
