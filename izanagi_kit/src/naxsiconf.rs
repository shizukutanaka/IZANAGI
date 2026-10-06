//! `naxsi` ルール設定 検出モジュール。
//!
//! Naxsi (nginx WAF) のルールファイルは `MainRule`/`BasicRule`/
//! `LearningMode`/`CheckRule`/`DeniedUrl`/`IgnoreIP`/`IgnoreCIDR`/
//! `DeniedUrl`/`SecRulesEnabled`/`SecRulesDisabled` 等の
//! ディレクティブで構成される。
//!
//! ```
//! let b = br#"LearningMode;
//! SecRulesEnabled;
//! DeniedUrl "/RequestDenied";
//! CheckRule "$SQL >= 8" BLOCK;
//! CheckRule "$RFI >= 8" BLOCK;
//! BasicRule wl:1310,1311 "mz:$ARGS_VAR:foo";
//! "#;
//! let c = izanagi_kit::naxsiconf::parse(b);
//! assert!(izanagi_kit::naxsiconf::detect(b));
//! assert_eq!(c.directives, 6);
//! ```

const DIRECTIVES: &[&str] = &[
    "BasicRule",
    "CheckRule",
    "DeniedUrl",
    "IgnoreCIDR",
    "IgnoreIP",
    "IgnoreUrl",
    "LearningMode",
    "MainRule",
    "SecRulesDisabled",
    "SecRulesEnabled",
    "ScoreZone",
    "WhitelistedID",
];

fn naxsi_head(t: &str) -> bool {
    let head = t.split_whitespace().next().unwrap_or("");
    let head = head.trim_end_matches(';');
    DIRECTIVES.contains(&head)
}

/// `b` が naxsi ルール設定に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut dirs = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') || tr.starts_with("//") {
            continue;
        }
        if naxsi_head(tr) {
            dirs += 1;
        }
    }
    dirs >= 2
}

/// naxsi ルール設定の統計。
#[derive(Debug, Default, Clone)]
pub struct NaxsiConf {
    /// Naxsi ディレクティブ行数。
    pub directives: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を naxsi ルール設定として統計する。
pub fn parse(b: &[u8]) -> NaxsiConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = NaxsiConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') || tr.starts_with("//") {
            c.comments += 1;
            continue;
        }
        if naxsi_head(tr) {
            c.directives += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"LearningMode;
SecRulesEnabled;
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.directives, 2);
    }

    #[test]
    fn detects_rules() {
        let b = br#"MainRule "str:abc" "msg:x" "mz:ARGS" 's:$SQL:8';
BasicRule wl:1 "mz:$ARGS_VAR:x";
"#;
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"LearningMode;\n"));
        assert!(!detect(b"checkrule a\nbasicrule b\n"));
        assert!(!detect(b"# MainRule x\n# BasicRule y\n# CheckRule z\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.directives, 0);
    }
}
