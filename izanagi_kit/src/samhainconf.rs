//! `samhainrc` (Samhain HIDS) 検出モジュール。
//!
//! Samhain (ホスト型侵入検知) の設定は `[Log]`/`[Misc]`/
//! `[Attributes]`/`[Suid]`/`[EventSeverity]`/`[IgnoreAll]`/
//! `[IgnoreMissing]`/`[Database]`/`[PortCheck]`/`[Herald]`/
//! `[Users]`/`[Kernel]` 等のセクションと `dir`/`file`/
//! `severity`/`export`/`check` キーで構成される。
//!
//! ```
//! let b = br#"[Attributes]
//! file = /etc/shadow
//! dir = 99/etc
//! [Log]
//! MailSeverity = crit
//! PrintSeverity = info
//! "#;
//! let c = izanagi_kit::samhainconf::parse(b);
//! assert!(izanagi_kit::samhainconf::detect(b));
//! assert_eq!(c.sections, 2);
//! ```

const SECTIONS: &[&str] = &[
    "[Attributes]",
    "[Cron]",
    "[Database]",
    "[EventSeverity]",
    "[Herald]",
    "[IgnoreAll]",
    "[IgnoreMissing]",
    "[IgnoreNone]",
    "[IgnoreReadOnly]",
    "[Kernel]",
    "[Log]",
    "[LogMon]",
    "[Misc]",
    "[PortCheck]",
    "[ReadOnly]",
    "[Suid]",
    "[UserFiles]",
    "[Users]",
];

const KEYS: &[&str] = &[
    "ChecksumTest",
    "Daemon",
    "Database",
    "dir",
    "dir0",
    "dir1",
    "file",
    "FileLog",
    "Facility",
    "LogMon",
    "LogSeverity",
    "Mail",
    "MailSeverity",
    "MailAddress",
    "PidFile",
    "PrintSeverity",
    "SamhainDB",
    "SamhainPurgeDays",
    "SuidCheckFile",
    "SetClientName",
    "SetTimeServer",
    "TrustedUser",
];

fn key_present(t: &str) -> bool {
    let k = t.split('=').next().unwrap_or("").trim();
    KEYS.contains(&k)
}

/// `b` が samhainrc に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut secs = 0usize;
    let mut keys = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') || tr.starts_with(';') {
            continue;
        }
        if SECTIONS.contains(&tr) {
            secs += 1;
        } else if key_present(tr) {
            keys += 1;
        }
    }
    (secs >= 1 && keys >= 2) || secs >= 2 || keys >= 3
}

/// samhainrc の統計。
#[derive(Debug, Default, Clone)]
pub struct SamhainConf {
    /// 既知セクション数。
    pub sections: usize,
    /// 既知キー行数。
    pub keys: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を samhainrc として統計する。
pub fn parse(b: &[u8]) -> SamhainConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = SamhainConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') || tr.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if SECTIONS.contains(&tr) {
            c.sections += 1;
        } else if key_present(tr) {
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
        let b = br#"[Attributes]
file = /etc/shadow
dir = 99/etc
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.sections, 1);
    }

    #[test]
    fn detects_sections() {
        let b = br#"[Log]
[Misc]
"#;
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"[Log]\n"));
        assert!(!detect(b"[Attributes]\nfoo = x\n"));
        assert!(!detect(b"dir = /etc\nfile = /etc/shadow\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.sections, 0);
    }
}
