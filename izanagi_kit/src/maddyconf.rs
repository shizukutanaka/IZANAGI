//! Maddy Mail Server `maddy.conf` 検出と構造カウント。
//!
//! `storage.*`/`log`/`state_dir` 等のグローバル + `smtp tcp://.. {...}`、
//! `imap tcp://.. {...}`、`delivery`、`tls`、`dmarc` 等のブロックモジュールを
//! 行頭モジュール + `key value` または `key value {` で分類する。
//!
//! ```
//! let c = izanagi_kit::maddyconf::parse(
//!     b"log off\nsmtp tcp://0.0.0.0:25 {\n  hostname mx.example.com\n}\n").unwrap();
//! assert_eq!(c.blocks, 1);
//! assert!(izanagi_kit::maddyconf::detect(b"state_dir /var/lib/maddy\nlog off\n"));
//! ```

/// 既知ブロックモジュール(行頭の語)。
const MODULES: &[&str] = &[
    "auth",
    "auth.local",
    "auth.remote",
    "bind",
    "chain",
    "check",
    "check.dkim",
    "check.dnsbl",
    "check.remote",
    "check.spf",
    "checks",
    "delivery",
    "dkim",
    "dmarc",
    "dns",
    "dsn",
    "filter",
    "imap",
    "imap.acl",
    "imap.filters",
    "limits",
    "log",
    "maildir",
    "mx_check",
    "outbound",
    "pigeonhole",
    "pipeline",
    "pipeline.chain",
    "pipeline.dedup",
    "pipeline.filter",
    "pipeline.chain*",
    "ratelimit",
    "remote",
    "replace",
    "security",
    "smtp",
    "smtp.dkim",
    "smtp.inbound",
    "submission",
    "storage",
    "storage.imapsql",
    "storage.maildir",
    "table",
    "table.local",
    "table.remote",
    "target",
    "tls",
    "track",
    "trusted_proxies",
];
/// グローバル/ブロック内既知キー(単一語も多い)。
const KEYS: &[&str] = &[
    "acme",
    "alpn",
    "always_check",
    "auth",
    "autodiscover",
    "autodiscover",
    "bl",
    "body_domain_mismatch",
    "body_forgery",
    "bounce",
    "cache",
    "certificates",
    "chain",
    "check",
    "client",
    "conn_limits",
    "creds",
    "debug",
    "delivery_target",
    "dkim",
    "dmarc",
    "dnsbl",
    "domain",
    "driver",
    "dsn",
    "enforce_early",
    "error",
    "flags",
    "from",
    "handover",
    "hostname",
    "idle_timeout",
    "imap_auth",
    "imap_delivery",
    "io_debug",
    "key",
    "lmtp",
    "log",
    "log_debug",
    "mail_from",
    "match_domain",
    "max_clients",
    "max_errors",
    "max_line_length",
    "max_msg_size",
    "max_received",
    "msg_store",
    "msgmetadata",
    "mx",
    "no",
    "no_ssl",
    "off",
    "on",
    "open_relay",
    "password",
    "path",
    "per_host",
    "pipe",
    "quarantine",
    "ratelimit",
    "raw",
    "read",
    "recipient",
    "reject",
    "remote",
    "remote_stores",
    "replace",
    "require_tls",
    "resolver",
    "retry",
    "runtime_dir",
    "shared",
    "sign",
    "sign_domains",
    "smtputf8",
    "spf",
    "sql",
    "sqlite3",
    "state_dir",
    "subject",
    "table",
    "target",
    "target_email",
    "target.smtp",
    "test",
    "tls",
    "tls_ciphers",
    "tls_curve",
    "tls_min_version",
    "write",
    "write_timeout",
];

/// Maddy 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `smtp tcp://.. {` / `imap ... {` / `tls file {` ブロック開始。
    pub blocks: usize,
    /// `key [value]` 既知代入/宣言行。
    pub options: usize,
    /// `}` 閉じブロック。
    pub closes: usize,
    /// `#`/`//` コメント行。
    pub comments: usize,
    /// 分類不能行。
    pub misc: usize,
}

/// 行が既知モジュールブロック開始か(`mod ... {` または `mod {`)。
fn block_head(t: &str) -> Option<&str> {
    let pos = t.find('{')?;
    let head = t[..pos].trim();
    head.split_whitespace().next()
}

/// b が maddy.conf かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let mut blks = 0;
    let mut opts = 0;
    for line in text.lines() {
        let t = line.trim();
        if let Some(m) = block_head(t) {
            if MODULES.contains(&m) {
                blks += 1;
            }
        } else if t
            .split_whitespace()
            .next()
            .is_some_and(|k| KEYS.contains(&k))
        {
            opts += 1;
        }
    }
    blks + opts >= 2
}

/// 構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        blocks: 0,
        options: 0,
        closes: 0,
        comments: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') || t.starts_with("//") {
            c.comments += 1;
            continue;
        }
        if t == "}" || t == "}," {
            c.closes += 1;
            continue;
        }
        if let Some(m) = block_head(t) {
            if MODULES.contains(&m) || m.split_whitespace().count() == 1 {
                c.blocks += 1;
            } else {
                c.misc += 1;
            }
            continue;
        }
        if t.split_whitespace()
            .next()
            .is_some_and(|k| KEYS.contains(&k) || k.contains('.'))
        {
            c.options += 1;
            continue;
        }
        c.misc += 1;
    }
    (c.blocks + c.options >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# maddy\nstate_dir /var/lib/maddy\nruntime_dir /run/maddy\nlog off\n\ntls file /etc/maddy/tls.pem /etc/maddy/tls.key\n\nsmtp tcp://0.0.0.0:25 {\n  hostname mx.example.com\n  debug yes\n}\n\ndelivery tcp://0.0.0.0:587 {\n  auth pass_table /etc/maddy/creds\n}\n";

    #[test]
    fn maddyconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.blocks, 2);
        assert_eq!(c.options, 7);
        assert_eq!(c.closes, 2);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn not_maddy() {
        assert!(!detect(b"random text\n"));
        assert!(!detect(b"[section]\nkey = value\n"));
    }
}
