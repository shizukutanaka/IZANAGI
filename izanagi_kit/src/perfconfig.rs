//! `~/.perfconfig` (Linux perf 設定) 検出モジュール。
//!
//! perf の設定は INI 風の `[section]` + `key = value` 形式で、
//! `[annotate]`/`[tui]`/`[buildid]`/`[colors]`/`[call-graph]` 等の
//! セクションが使われる。
//!
//! ```
//! let b = br#"[tui]
//! report = on
//! top = on
//! [annotate]
//! hide_src_code = false
//! jump_arrows = on
//! [call-graph]
//! print-type = graph
//! "#;
//! let c = izanagi_kit::perfconfig::parse(b);
//! assert!(izanagi_kit::perfconfig::detect(b));
//! assert_eq!(c.sections, 3);
//! ```

const SECTIONS: &[&str] = &[
    "annotate",
    "buildid",
    "call-graph",
    "colors",
    "command",
    "diff",
    "dump",
    "help",
    "hist",
    "intel-pt",
    "kmem",
    "kvm",
    "llvm",
    "man",
    "pager",
    "prompt",
    "report",
    "script",
    "stat",
    "top",
    "trace",
    "tui",
];

const KEYS: &[&str] = &[
    "hide_src_code",
    "jump_arrows",
    "print-line",
    "print-type",
    "report",
    "show-nr-samples",
    "show-on-off-events",
    "show-total-period",
    "skip-missing",
    "sort-order",
    "top",
    "use_offset",
    "use_pid",
    "use_tid",
];

fn is_comment(t: &str) -> bool {
    t.starts_with('#') || t.starts_with(';')
}

fn section_name(t: &str) -> Option<&str> {
    if !(t.starts_with('[') && t.ends_with(']')) {
        return None;
    }
    Some(&t[1..t.len() - 1])
}

fn key_present(t: &str, k: &str) -> bool {
    if !t.starts_with(k) {
        return false;
    }
    let rest = t[k.len()..].trim_start();
    rest.starts_with('=')
}

/// `b` が .perfconfig に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut secs = 0usize;
    let mut in_sec = false;
    let mut keys = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || is_comment(tr) {
            continue;
        }
        if let Some(n) = section_name(tr) {
            in_sec = SECTIONS.contains(&n);
            if in_sec {
                secs += 1;
            }
            continue;
        }
        if in_sec && KEYS.iter().any(|k| key_present(tr, k)) {
            keys += 1;
        }
    }
    secs >= 1 && keys >= 1
}

/// .perfconfig の統計。
#[derive(Debug, Default, Clone)]
pub struct PerfConf {
    /// 既知セクション数。
    pub sections: usize,
    /// セクション内既知キー行数。
    pub keys: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を .perfconfig として統計する。
pub fn parse(b: &[u8]) -> PerfConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = PerfConf::default();
    let mut in_sec = false;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if is_comment(tr) {
            c.comments += 1;
            continue;
        }
        if let Some(n) = section_name(tr) {
            in_sec = SECTIONS.contains(&n);
            if in_sec {
                c.sections += 1;
            }
            continue;
        }
        if in_sec && KEYS.iter().any(|k| key_present(tr, k)) {
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
        let b = br#"[tui]
report = on
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.sections, 1);
        assert_eq!(c.keys, 1);
    }

    #[test]
    fn detects_annotate() {
        let b = br#"[annotate]
hide_src_code = false
jump_arrows = on
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 2);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"[tui]\n"));
        assert!(!detect(b"[foo]\nreport = on\n"));
        assert!(!detect(b"key = 1\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.sections, 0);
    }
}
