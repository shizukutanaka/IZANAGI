//! `.core` (FuseSoC core file) 検出モジュール。
//!
//! FuseSoC のコア記述は YAML で、必須の `CAPI=2:` ヘッダ行と
//! `name:`/`filesets:`/`targets:`/`providers:`/`generate:`/
//! `scripts:`/`vpi:` キーが特徴。
//!
//! ```
//! let b = br#"CAPI=2:
//! name: ::example:1.0.0
//! description: Example core
//! filesets:
//!   rtl:
//!     files:
//!       - top.v
//! targets:
//!   default:
//!     filesets: [rtl]
//! "#;
//! let c = izanagi_kit::fusesoc::parse(b);
//! assert!(izanagi_kit::fusesoc::detect(b));
//! assert!(c.has_capi);
//! ```

const KEYS: &[&str] = &[
    "depend",
    "depends",
    "description",
    "filesets",
    "flags",
    "flow_options",
    "generate",
    "hooks",
    "name",
    "parameters",
    "provider",
    "providers",
    "scripts",
    "targets",
    "tools",
    "vlnv",
    "vpi",
];

fn is_comment(t: &str) -> bool {
    t.starts_with('#')
}

fn top_key(t: &str, k: &str) -> bool {
    // top-level (unindented) YAML key
    if t.starts_with(' ') || t.starts_with('\t') || t.starts_with('-') {
        return false;
    }
    let Some(colon) = t.find(':') else {
        return false;
    };
    t[..colon].trim() == k
}

/// `b` が FuseSoC .core ファイルに見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut capi = false;
    let mut keys = 0usize;
    for l in t.lines() {
        let tr = l.trim_end();
        if tr.trim().is_empty() || is_comment(tr.trim()) {
            continue;
        }
        if tr.trim() == "CAPI=2:" {
            capi = true;
            continue;
        }
        if KEYS.iter().any(|k| top_key(tr, k)) {
            keys += 1;
        }
    }
    capi && keys >= 1
}

/// .core の統計。
#[derive(Debug, Default, Clone)]
pub struct FusesocCore {
    /// `CAPI=2:` 行があったか。
    pub has_capi: bool,
    /// 既知トップレベルキー数。
    pub keys: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を FuseSoC .core として統計する。
pub fn parse(b: &[u8]) -> FusesocCore {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = FusesocCore::default();
    for l in t.lines() {
        let tr = l.trim_end();
        if tr.trim().is_empty() {
            continue;
        }
        if is_comment(tr.trim()) {
            c.comments += 1;
            continue;
        }
        if tr.trim() == "CAPI=2:" {
            c.has_capi = true;
            continue;
        }
        if KEYS.iter().any(|k| top_key(tr, k)) {
            c.keys += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"CAPI=2:
name: ::example:1.0.0
"#;
        assert!(detect(b));
        let c = parse(b);
        assert!(c.has_capi);
        assert_eq!(c.keys, 1);
    }

    #[test]
    fn detects_full() {
        let b = br#"CAPI=2:
name: ::example:1.0.0
filesets:
  rtl:
    files:
      - top.v
targets:
  default:
    filesets: [rtl]
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 3);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"name: x\nfilesets:\ntargets:\n"));
        assert!(!detect(b"CAPI=2:\n"));
        assert!(!detect(b"key: value\nother: v2\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert!(!c.has_capi);
    }
}
