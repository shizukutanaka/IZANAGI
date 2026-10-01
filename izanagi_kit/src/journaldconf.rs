//! systemd-journald 設定ファイル (`journald.conf` / `journald.conf.d/*.conf`) の解析。
//!
//! `[Journal]`/`[Upload]` セクションと `Storage`/`SystemMaxUse`/`ForwardToSyslog`
//! 等の既知キーを持つ設定を検出し、セクション数・エントリ数・既知キー数を
//! 整数で返す。
//!
//! # Examples
//!
//! ```
//! use izanagi_kit::journaldconf;
//!
//! let text = br#"[Journal]
//! Storage=persistent
//! Compress=yes
//! SystemMaxUse=1G
//! ForwardToSyslog=no
//! "#;
//!
//! assert!(journaldconf::detect(text));
//! let c = journaldconf::parse(text).unwrap();
//! assert_eq!(c.sections, 1);
//! assert_eq!(c.entries, 4);
//! assert_eq!(c.known_keys, 4);
//! ```

/// `[Journal]`/`[Upload]` の既知キー。
const KNOWN_KEYS: &[&str] = &[
    "Storage",
    "Compress",
    "Seal",
    "SplitMode",
    "SyncIntervalSec",
    "RateLimitIntervalSec",
    "RateLimitInterval",
    "RateLimitBurst",
    "SystemMaxUse",
    "SystemKeepFree",
    "SystemMaxFileSize",
    "SystemMaxFiles",
    "RuntimeMaxUse",
    "RuntimeKeepFree",
    "RuntimeMaxFileSize",
    "RuntimeMaxFiles",
    "MaxRetentionSec",
    "MaxFileSec",
    "ForwardToSyslog",
    "ForwardToKMsg",
    "ForwardToConsole",
    "ForwardToWall",
    "ForwardToSocket",
    "TTYPath",
    "MaxLevelStore",
    "MaxLevelSyslog",
    "MaxLevelKMsg",
    "MaxLevelConsole",
    "MaxLevelWall",
    "MaxLevelSocket",
    "LineMax",
    "Audit",
    "ReadKMsg",
    "Socket",
    "URL",
    "ServerKeyFile",
    "ServerCertificateFile",
    "TrustedCertificateFile",
    "NetworkTimeoutSec",
];

/// セクション名の既知名。
const KNOWN_SECTIONS: &[&str] = &["Journal", "Upload", "Journald"];

/// journald.conf の集計。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `[…]` セクション数。
    pub sections: usize,
    /// 既知セクション数。
    pub known_sections: usize,
    /// `Key=Value` エントリ数。
    pub entries: usize,
    /// 既知キーを持つエントリ数。
    pub known_keys: usize,
    /// `ForwardTo*`/`MaxLevel*` 系キーの数。
    pub forwarding_keys: usize,
}

/// `b` が journald.conf らしいかを返す。
pub fn detect(b: &[u8]) -> bool {
    let c = match parse(b) {
        Some(c) => c,
        None => return false,
    };
    (c.known_sections >= 1 && c.known_keys >= 2) || c.known_keys >= 3
}

/// journald.conf を解析して `Counts` を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let s = std::str::from_utf8(b).ok()?;
    let mut counts = Counts {
        sections: 0,
        known_sections: 0,
        entries: 0,
        known_keys: 0,
        forwarding_keys: 0,
    };
    let mut saw_any = false;
    for raw in s.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            counts.sections += 1;
            saw_any = true;
            let name = &line[1..line.len() - 1];
            if KNOWN_SECTIONS.contains(&name) {
                counts.known_sections += 1;
            }
            continue;
        }
        let Some(eq) = line.find('=') else {
            continue;
        };
        let key = line[..eq].trim();
        if key.is_empty() {
            continue;
        }
        counts.entries += 1;
        saw_any = true;
        if KNOWN_KEYS.contains(&key) {
            counts.known_keys += 1;
        }
        if key.starts_with("ForwardTo") || key.starts_with("MaxLevel") {
            counts.forwarding_keys += 1;
        }
    }
    if !saw_any {
        return None;
    }
    Some(counts)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = br#"# journald.conf
[Journal]
Storage=persistent
Compress=yes
Seal=yes
RateLimitIntervalSec=30s
RateLimitBurst=10000
SystemMaxUse=4G
SystemKeepFree=1G
RuntimeMaxUse=512M
ForwardToSyslog=yes
ForwardToWall=no
MaxLevelStore=debug
LineMax=48K
Audit=yes

[Upload]
URL=https://logs.example.com
ServerKeyFile=/etc/ssl/journal.pem
"#;

    #[test]
    fn detects_journald() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 2);
        assert_eq!(c.known_sections, 2);
        assert_eq!(c.entries, 15);
        assert_eq!(c.known_keys, 15);
        assert_eq!(c.forwarding_keys, 3);
    }

    #[test]
    fn rejects_other_text() {
        assert!(!detect(b"[section]\nfoo=bar\n"));
        assert!(!detect(b"hello"));
    }
}
