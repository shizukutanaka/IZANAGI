//! Fail2ban 設定ファイル (`jail.conf`/`jail.d/*.conf`/`fail2ban.conf`/
//! `filter.d/*.conf`/`action.d/*.conf`) の解析。
//!
//! `[DEFAULT]` + jail セクション名 (`[sshd]` 等) と `enabled`/`maxretry`/
//! `findtime`/`bantime`/`filter`/`action`/`logpath`/`failregex`/`ignoreregex`
//! 系キーを持つ設定を検出し、jail 数・既知キー数を整数で返す。
//!
//! # Examples
//!
//! ```
//! use izanagi_kit::fail2ban;
//!
//! let text = br#"[DEFAULT]
//! bantime = 3600
//! findtime = 600
//! maxretry = 3
//!
//! [sshd]
//! enabled = true
//! filter = sshd
//! logpath = /var/log/auth.log
//! "#;
//!
//! assert!(fail2ban::detect(text));
//! let c = fail2ban::parse(text).unwrap();
//! assert_eq!(c.sections, 2);
//! assert_eq!(c.jails, 1);
//! assert_eq!(c.known_keys, 6);
//! ```

/// fail2ban の既知キー(jail.conf / fail2ban.conf / filter.d / action.d 共通)。
const KNOWN_KEYS: &[&str] = &[
    "enabled",
    "port",
    "logpath",
    "maxretry",
    "findtime",
    "bantime",
    "bantime.increment",
    "bantime.factor",
    "bantime.formula",
    "bantime.multipliers",
    "bantime.maxtime",
    "bantime.rndtime",
    "bantime.overalljails",
    "ignoreip",
    "action",
    "banaction",
    "banaction_allports",
    "banaction-bf",
    "actionstart",
    "actionstop",
    "actioncheck",
    "actionban",
    "actionunban",
    "actionrepair",
    "actionflush",
    "actionview",
    "actionreport",
    "filter",
    "failregex",
    "ignoreregex",
    "prefregex",
    "datepattern",
    "journalmatch",
    "backend",
    "usedns",
    "logencoding",
    "logtimezone",
    "protocol",
    "mode",
    "mangles",
    "chain",
    "chainopt",
    "iptables",
    "nftables",
    "multiport",
    "allports",
    "portrange",
    "returntype",
    "defaultprotocol",
    "destemail",
    "sender",
    "sendername",
    "mta",
    "mta-target",
    "mta-additional-opt",
    "mta-env",
    "mailcmd",
    "mailargs",
    "whois",
    "whoiscmd",
    "nslookup",
    "dockerurl",
    "jail",
    "jailclient",
    "dbfile",
    "dbpurgeage",
    "dbmaxmatches",
    "dbfilechksum",
    "socket",
    "pidfile",
    "loglevel",
    "logtarget",
    "syslogsocket",
    "syslogsync",
    "allowipv6",
    "regex",
    "prepend_data",
    "multiline",
    "init",
    "initdir",
    "initparam",
    "initpidfile",
    "pkgname",
    "caCertFile",
    "check",
];

/// セクション名が `[DEFAULT]`/`[Definition]` 以外の場合 jail とみなす。
fn is_jail_section(name: &str) -> bool {
    !matches!(name, "DEFAULT" | "Definition" | "INCLUDES" | "Overrides")
}

/// fail2ban 設定の集計。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `[…]` セクション総数。
    pub sections: usize,
    /// jail セクション数 (`[DEFAULT]`/`[Definition]` 以外)。
    pub jails: usize,
    /// `key = value` エントリ数。
    pub entries: usize,
    /// 既知キーを持つエントリ数。
    pub known_keys: usize,
    /// `action*`/`banaction*`/`failregex`/`ignoreregex` 系エントリ数。
    pub regex_and_actions: usize,
}

/// `b` が fail2ban 設定ファイルらしいかを返す。
pub fn detect(b: &[u8]) -> bool {
    let c = match parse(b) {
        Some(c) => c,
        None => return false,
    };
    (c.sections >= 1 && c.known_keys >= 3) || c.known_keys >= 5
}

/// fail2ban 設定を解析して `Counts` を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let s = std::str::from_utf8(b).ok()?;
    let mut counts = Counts {
        sections: 0,
        jails: 0,
        entries: 0,
        known_keys: 0,
        regex_and_actions: 0,
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
            if is_jail_section(&line[1..line.len() - 1]) {
                counts.jails += 1;
            }
            continue;
        }
        let Some(eq) = line.find('=') else {
            continue;
        };
        let key = line[..eq].trim().to_ascii_lowercase();
        if key.is_empty()
            || !key
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'_' | b'-' | b'*'))
        {
            continue;
        }
        counts.entries += 1;
        saw_any = true;
        if KNOWN_KEYS.contains(&key.as_str()) {
            counts.known_keys += 1;
        }
        if key.starts_with("action")
            || key.starts_with("banaction")
            || key == "failregex"
            || key == "ignoreregex"
            || key == "prefregex"
        {
            counts.regex_and_actions += 1;
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

    const SAMPLE: &[u8] = br#"[DEFAULT]
bantime = 3600
findtime = 600
maxretry = 3
ignoreip = 127.0.0.1/8
banaction = iptables-multiport
backend = auto

[sshd]
enabled = true
filter = sshd
logpath = /var/log/auth.log
maxretry = 4

[nginx-http-auth]
enabled = true
filter = nginx-http-auth
port = http,https
logpath = /var/log/nginx/error.log

[recidive]
enabled = true
filter = recidive
banaction_allports = iptables-allports
bantime = 86400
"#;

    #[test]
    fn detects_fail2ban() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 4);
        assert_eq!(c.jails, 3);
        assert_eq!(c.entries, 18);
        assert_eq!(c.known_keys, 18);
        assert_eq!(c.regex_and_actions, 2);
    }

    #[test]
    fn rejects_other_text() {
        assert!(!detect(b"[section]\nfoo=bar\n"));
        assert!(!detect(b"hello"));
    }
}
