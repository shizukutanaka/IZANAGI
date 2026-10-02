//! Haraka SMTP サーバの `config/*.ini`(`smtp.ini` 等)と `config/plugins` の検出と構造カウント。
//!
//! `[main]`/`[tls]`/`[dkim]` セクション + `key = value` 代入、あるいは
//! 1 行 1 プラグインの `config/plugins` リストを識別する。
//!
//! ```
//! let c = izanagi_kit::harakaconf::parse(
//!     b"[main]\nport = 25\nnodes = cpus\n[tls]\nrejectUnauthorized = true\n").unwrap();
//! assert_eq!(c.sections, 2);
//! assert!(izanagi_kit::harakaconf::detect(b"[main]\nsmtpgreeting = mail\n"));
//! ```

/// 既知セクション。
const SECTIONS: &[&str] = &[
    "main",
    "tls",
    "dkim",
    "spf",
    "bounce",
    "deny",
    "dnsbl",
    "fcrdns",
    "helo.checks",
    "host_list",
    "karma",
    "limit",
    "log.syslog",
    "mail_from",
    "messagesniffer",
    "outbound",
    "queue",
    "ratelimit",
    "rcpt_to",
    "record_envelope_addresses",
    "relay",
    "rspamd",
    "smtpapi",
    "spamassassin",
    "tls.ini",
    "watch",
];
/// 既知キー。
const KEYS: &[&str] = &[
    "always_add_headers",
    "authorized_extensions",
    "bad_connections",
    "bad_karma",
    "bind_ip",
    "bounce_expires",
    "bounce_template",
    "cert",
    "check_karma",
    "concurrent",
    "connect_timeout",
    "daemonize",
    "delay",
    "deny_includes_uuid",
    "disable_tls",
    "doc_chain_filename",
    "domain",
    "expiry_days",
    "forward_comment",
    "graceful_shutdown",
    "header_key",
    "hide_version",
    "host_header",
    "hosts_to_exclude",
    "incoming",
    "invalid_doc_values",
    "is_resolvable",
    "key",
    "limit_tarpitting",
    "log_level",
    "logfile",
    "main",
    "max_mime_parts",
    "max_received",
    "me",
    "message_size_limit",
    "nodes",
    "no_tls_hosts",
    "port",
    "privileged_user",
    "put_msg_on_queue",
    "reject",
    "reject_nxdomain",
    "rejectUnauthorized",
    "relay_dest_domains",
    "rename_headers",
    "requestedCipher",
    "requestCert",
    "respectAuth",
    "send_ehlo_message",
    "smtp_return_code",
    "smtp_response_code",
    "smtpgreeting",
    "spool_after",
    "spool_dir",
    "strict_rfc1869",
    "temp_fail_queue_time",
    "timeout",
    "tls_version",
    "uuid",
    "weight",
];

/// Haraka 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `[...]` セクション。
    pub sections: usize,
    /// `key = value` 既知代入。
    pub options: usize,
    /// `;`/`#` コメント行。
    pub comments: usize,
    /// プラグイン名/裸の値(plugins リスト)。
    pub plugins: usize,
    /// 分類不能行。
    pub misc: usize,
}

/// b が Haraka ini/plugins かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let mut secs = 0;
    let mut opts = 0;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('[')
            && t.ends_with(']')
            && SECTIONS.iter().any(|s| t[1..t.len() - 1] == **s)
        {
            secs += 1;
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
        plugins: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with(';') || t.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if t.starts_with('[') && t.ends_with(']') {
            c.sections += 1;
            continue;
        }
        if let Some(p) = t.find('=') {
            if KEYS.contains(&t[..p].trim()) {
                c.options += 1;
            } else {
                c.misc += 1;
            }
            continue;
        }
        // `config/plugins` 形式: `plugin/name` または裸名。
        if t.split_whitespace().count() <= 2
            && t.chars().all(|ch| {
                ch.is_ascii_alphanumeric() || ch == '/' || ch == '.' || ch == '_' || ch == '-'
            })
        {
            c.plugins += 1;
            continue;
        }
        c.misc += 1;
    }
    (c.sections + c.options + c.plugins >= 2).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"; Haraka\n[main]\nport = 25\nnodes = cpus\nsmtpgreeting = mail.example.com\n[tls]\nkey = config/tls_key.pem\ncert = config/tls_cert.pem\n";

    #[test]
    fn harakaconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 2);
        assert_eq!(c.options, 5);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn plugins_list() {
        let c = parse(b"dnsbl\naccess\nbackscatterer\n").unwrap();
        assert_eq!(c.plugins, 3);
    }

    #[test]
    fn not_haraka() {
        assert!(!detect(b"[other]\nx = y\n"));
        assert!(!detect(b"hello\n"));
    }
}
