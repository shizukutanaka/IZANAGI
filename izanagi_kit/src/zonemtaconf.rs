//! ZoneMTA `zonemta.toml` の検出と構造カウント。
//!
//! `[api]`/`[smtp]`/`[zones]`/`[plugins]`/`[dkim]`/`[queue]` 等の
//! テーブル + ZoneMTA 固有のキー(`feeder`/`deliveryConcurrency`/
//! `processes`/`relay`/`pool`/`gelf`/`log`/`hostname`/`disableIPv6` 等)を
//! 分類する。
//!
//! ```
//! let c = izanagi_kit::zonemtaconf::parse(
//!     b"[api]\nport = 8080\n[smtp]\nfeeder = false\n").unwrap();
//! assert_eq!(c.sections, 2);
//! assert!(izanagi_kit::zonemtaconf::detect(b"[smtp]\nfeeder = true\n"));
//! ```

/// 既知セクション(テーブル名または先頭セグメント)。
const SECTIONS: &[&str] = &[
    "api", "asn", "auth", "bounces", "bull", "clamav", "db", "dkim", "delivery", "feeder",
    "forward", "gelf", "general", "hostname", "ip", "log", "maildrop", "mx", "plugins", "pool",
    "pools", "queue", "redis", "rewrite", "send", "smtp", "srs", "tls", "wildduck", "zone",
    "zones",
];
/// 既知キー。
const KEYS: &[&str] = &[
    "allowIPv6",
    "alwaysAnonymize",
    "apiKey",
    "auth",
    "authExpireTime",
    "authentication",
    "bind",
    "bounces",
    "command",
    "component",
    "connection",
    "connections",
    "debug",
    "deferAllErrors",
    "deliveries",
    "deliveryConcurrency",
    "disableIPv6",
    "disabled",
    "dkim",
    "dropHeaders",
    "enabled",
    "excludeDomains",
    "feeder",
    "forwardedHeader",
    "gelf",
    "grayLogEnabled",
    "hostname",
    "ignoreEnvelope",
    "includeInMessageId",
    "key",
    "level",
    "listen",
    "log",
    "logPath",
    "mailFrom",
    "maxAuthFailures",
    "maxConnections",
    "maxErrors",
    "maxRetries",
    "maxTime",
    "messageExpire",
    "mongo",
    "name",
    "origin",
    "password",
    "path",
    "pidfile",
    "plugins",
    "pool",
    "port",
    "preferIPv4",
    "processes",
    "processes.recipients",
    "queue",
    "queueDb",
    "redis",
    "redisEventMaxAge",
    "registerBuiltin",
    "reject",
    "relay",
    "requireAuth",
    "requireTLS",
    "rewrite",
    "sendHosts",
    "server",
    "srs",
    "subject",
    "syslog",
    "threads",
    "tls",
    "transport",
    "useSMTP",
    "version",
    "zone",
];

/// ZoneMTA 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `[...]`/`[[...]]` テーブルヘッダ。
    pub sections: usize,
    /// `key = value` 既知代入。
    pub options: usize,
    /// `#`/`;`/`//` コメント行。
    pub comments: usize,
    /// 未知キー代入。
    pub unknown: usize,
    /// 分類不能行。
    pub misc: usize,
}

/// b が zonemta.toml かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let mut secs = 0;
    let mut opts = 0;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') {
            let inner = t
                .trim_matches(|c| c == '[' || c == ']')
                .split('.')
                .next()
                .unwrap_or("");
            if SECTIONS.contains(&inner) {
                secs += 1;
            }
        } else if t.find('=').is_some_and(|p| KEYS.contains(&t[..p].trim())) {
            opts += 1;
        }
    }
    secs + opts >= 2
}

/// 構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        options: 0,
        comments: 0,
        unknown: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') || t.starts_with("//") || t.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if t.starts_with('[') && (t.ends_with(']') || t.ends_with("]]")) {
            c.sections += 1;
            continue;
        }
        if let Some(p) = t.find('=') {
            if KEYS.contains(&t[..p].trim()) {
                c.options += 1;
            } else {
                c.unknown += 1;
            }
            continue;
        }
        c.misc += 1;
    }
    (c.sections + c.options >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# zonemta\n[api]\nport = 8080\nenabled = true\n\n[smtp]\nfeeder = false\nmaxConnections = 100\n\n[zones]\ndefault = \"default\"\n";

    #[test]
    fn zonemtaconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.options, 4);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn not_zonemta() {
        assert!(!detect(b"[package]\nname = \"x\"\n"));
        assert!(!detect(b"hello\n"));
    }
}
