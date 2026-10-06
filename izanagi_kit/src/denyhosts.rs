//! `denyhosts.conf` 検出モジュール。
//!
//! DenyHosts の設定は `KEY = value` 形式で、
//! `SECURE_LOG`/`HOSTS_DENY`/`PURGE_DENY`/`BLOCK_SERVICE`/
//! `DENY_THRESHOLD_INVALID`/`DENY_THRESHOLD_VALID`/`WORK_DIR`/
//! `DAEMON_LOG`/`LOCK_FILE`/`SYNC_*`/`AGE_RESET_*`/`SMTP_*`
//! 等の大文字キーで構成される。
//!
//! ```
//! let b = br#"SECURE_LOG = /var/log/auth.log
//! HOSTS_DENY = /etc/hosts.deny
//! PURGE_DENY = 1d
//! BLOCK_SERVICE = ALL
//! DENY_THRESHOLD_INVALID = 5
//! "#;
//! let c = izanagi_kit::denyhosts::parse(b);
//! assert!(izanagi_kit::denyhosts::detect(b));
//! assert_eq!(c.keys, 5);
//! ```

const KEYS: &[&str] = &[
    "ADMIN_EMAIL",
    "AGE_RESET_ALLOWED",
    "AGE_RESET_BANNED",
    "AGE_RESET_INVALID",
    "AGE_RESET_ROOT",
    "AGE_RESET_VALID",
    "ALLOWED_HOSTS_HOSTNAME_LOOKUP",
    "BLOCK_SERVICE",
    "CFG",
    "DAEMON_LOG",
    "DAEMON_LOG_FORMAT",
    "DAEMON_LOG_TIME_FORMAT",
    "DAEMON_PURGE",
    "DAEMON_SLEEP",
    "DENY_THRESHOLD_INVALID",
    "DENY_THRESHOLD_ROOT",
    "DENY_THRESHOLD_RESTRICTED",
    "DENY_THRESHOLD_VALID",
    "ETC_DIR",
    "FORMAT_DATE_TIME",
    "HOSTNAME_LOOKUP",
    "HOSTS_DENY",
    "LOCK_FILE",
    "PLUGIN_DENY",
    "PLUGIN_PURGE",
    "PURGE_DENY",
    "PURGE_THRESHOLD",
    "RESET_ON_SUCCESS",
    "SECURE_LOG",
    "SECURE_ROOT",
    "SMTP_DATE_FORMAT",
    "SMTP_FROM",
    "SMTP_HOST",
    "SMTP_PASSWORD",
    "SMTP_PORT",
    "SMTP_SUBJECT",
    "SMTP_USERNAME",
    "SMTPSSL",
    "SUSPICIOUS_LOGIN_REPORT_ALLOWED_HOSTS",
    "SYNC_DOWNLOAD",
    "SYNC_DOWNLOAD_RESILIENCY",
    "SYNC_DOWNLOAD_THRESHOLD",
    "SYNC_INTERVAL",
    "SYNC_SERVER",
    "SYNC_UPLOAD",
    "SYSLOG_REPORT",
    "USERDEF_FAILED_ENTRY_REGEX",
    "WORK_DIR",
];

fn key_present(t: &str) -> bool {
    let k = t.split('=').next().unwrap_or("").trim();
    KEYS.contains(&k)
}

/// `b` が denyhosts.conf に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut keys = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') {
            continue;
        }
        if key_present(tr) {
            keys += 1;
        }
    }
    keys >= 3
}

/// denyhosts.conf の統計。
#[derive(Debug, Default, Clone)]
pub struct DenyhostsConf {
    /// 既知キー行数。
    pub keys: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を denyhosts.conf として統計する。
pub fn parse(b: &[u8]) -> DenyhostsConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = DenyhostsConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if key_present(tr) {
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
        let b = br#"SECURE_LOG = /var/log/auth.log
HOSTS_DENY = /etc/hosts.deny
PURGE_DENY = 1d
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 3);
    }

    #[test]
    fn detects_compact() {
        let b = br#"BLOCK_SERVICE=ALL
DENY_THRESHOLD_INVALID=5
WORK_DIR=/var/lib/denyhosts
"#;
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"SECURE_LOG = x\nHOSTS_DENY = y\n"));
        assert!(!detect(b"FOO = a\nBAR = b\nBAZ = c\nQUX = d\n"));
        assert!(!detect(b"secure_log = x\nhosts_deny = y\npurge_deny = z\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.keys, 0);
    }
}
