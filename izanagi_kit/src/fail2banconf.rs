//! `jail.conf` / `jail.local` / `fail2ban.conf` (Fail2ban) 検出モジュール。
//!
//! Fail2ban の設定は INI 形式で、`[DEFAULT]`/`[sshd]` 等の
//! jail セクションと `enabled`/`bantime`/`findtime`/`maxretry`/
//! `logpath`/`filter`/`action`/`ignoreip`/`port`/`banaction`
//! 等のキーで構成される。
//!
//! ```
//! let b = br#"[DEFAULT]
//! bantime = 10m
//! findtime = 10m
//! maxretry = 5
//! [sshd]
//! enabled = true
//! filter = sshd
//! logpath = /var/log/auth.log
//! "#;
//! let c = izanagi_kit::fail2banconf::parse(b);
//! assert!(izanagi_kit::fail2banconf::detect(b));
//! assert_eq!(c.keys, 6);
//! ```

const KEYS: &[&str] = &[
    "action",
    "action_",
    "actionabuseipdb",
    "action_mw",
    "action_mwl",
    "actionstart",
    "actionstop",
    "bantime",
    "bantime.increment",
    "bantime.multipliers",
    "banaction",
    "banaction_allports",
    "banaction_routing",
    "chain",
    "checkcan",
    "checktime",
    "datepattern",
    "dbfile",
    "dbpurgeage",
    "destemail",
    "enabled",
    "encoding",
    "failregex",
    "findtime",
    "filter",
    "ignoreregex",
    "ignoreip",
    "ignoreself",
    "ignoresystem",
    "journalmatch",
    "known/api",
    "logencoding",
    "logpath",
    "logtarget",
    "logtime",
    "maxlines",
    "maxretry",
    "mode",
    "mta",
    "mta_stock",
    "norestored",
    "nsswitch",
    "port",
    "protocall",
    "protocol",
    "sender",
    "sendername",
    "socket",
    "usedns",
    "usessl",
];

fn key_present(t: &str) -> bool {
    let k = t.split('=').next().unwrap_or("").trim();
    KEYS.contains(&k)
}

/// `b` が Fail2ban 設定に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut secs = 0usize;
    let mut keys = 0usize;
    let mut defaults = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') || tr.starts_with(';') {
            continue;
        }
        if tr == "[DEFAULT]" {
            defaults += 1;
            secs += 1;
        } else if tr.starts_with('[') && tr.ends_with(']') {
            secs += 1;
        } else if key_present(tr) {
            keys += 1;
        }
    }
    (defaults >= 1 && keys >= 2) || keys >= 3 || (secs >= 2 && keys >= 2)
}

/// Fail2ban 設定の統計。
#[derive(Debug, Default, Clone)]
pub struct Fail2banConf {
    /// セクション数。
    pub sections: usize,
    /// 既知キー行数。
    pub keys: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を Fail2ban 設定として統計する。
pub fn parse(b: &[u8]) -> Fail2banConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = Fail2banConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') || tr.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if tr.starts_with('[') && tr.ends_with(']') {
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
        let b = br#"[DEFAULT]
bantime = 10m
maxretry = 5
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.sections, 1);
        assert_eq!(c.keys, 2);
    }

    #[test]
    fn detects_keys_only() {
        let b = br#"enabled = true
filter = sshd
logpath = /var/log/auth.log
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 3);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"[DEFAULT]\nfoo = x\n"));
        assert!(!detect(b"[server]\nport = 22\nenabled = true\n"));
        assert!(!detect(b"enabled = true\nport = 22\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.sections, 0);
    }
}
