//! `.qpf` (Quartus Project File) 検出モジュール。
//!
//! Quartus のプロジェクトファイルは `QUARTUS_VERSION = "..."`、`DATE = `、
//! `PROJECT_REVISION = "..."` の固定ヘッダ行を持つ。
//!
//! ```
//! let b = br#"QUARTUS_VERSION = "18.0"
//! DATE = "12:00:00  January 01, 2024"
//!
//! # Revisions
//!
//! PROJECT_REVISION = "top"
//! "#;
//! let c = izanagi_kit::qpf::parse(b);
//! assert!(izanagi_kit::qpf::detect(b));
//! assert!(c.has_quartus_version);
//! ```

fn is_comment(t: &str) -> bool {
    t.starts_with('#')
}

fn kv_line(t: &str, k: &str) -> bool {
    let Some(eq) = t.find('=') else {
        return false;
    };
    t[..eq].trim() == k
}

/// `b` が Quartus Project File に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut version = false;
    let mut revision = false;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || is_comment(tr) {
            continue;
        }
        if kv_line(tr, "QUARTUS_VERSION") {
            version = true;
        } else if kv_line(tr, "PROJECT_REVISION") {
            revision = true;
        }
    }
    version && revision
}

/// .qpf の統計。
#[derive(Debug, Default, Clone)]
pub struct Qpf {
    /// `QUARTUS_VERSION` 行があったか。
    pub has_quartus_version: bool,
    /// `PROJECT_REVISION` 行数。
    pub revisions: usize,
    /// その他の `KEY = VALUE` 行数。
    pub other_assignments: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を .qpf として統計する。
pub fn parse(b: &[u8]) -> Qpf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = Qpf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if is_comment(tr) {
            c.comments += 1;
            continue;
        }
        if kv_line(tr, "QUARTUS_VERSION") {
            c.has_quartus_version = true;
        } else if kv_line(tr, "PROJECT_REVISION") {
            c.revisions += 1;
        } else if tr.contains('=') {
            c.other_assignments += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"QUARTUS_VERSION = "18.0"
PROJECT_REVISION = "top"
"#;
        assert!(detect(b));
        let c = parse(b);
        assert!(c.has_quartus_version);
        assert_eq!(c.revisions, 1);
    }

    #[test]
    fn detects_full() {
        let b = br#"QUARTUS_VERSION = "18.0"
DATE = "12:00:00  January 01, 2024"

# Revisions

PROJECT_REVISION = "top"
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.other_assignments, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"QUARTUS_VERSION = \"18.0\"\n"));
        assert!(!detect(b"PROJECT_REVISION = \"top\"\n"));
        assert!(!detect(b"key = value\nother = x\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert!(!c.has_quartus_version);
    }
}
