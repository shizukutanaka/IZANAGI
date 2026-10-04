//! Haraka `config/*.ini` and `config/plugins` plugin list.
//!
//! ```
//! let b = b"[core]\nlisten=:::25\nsmtputf8=true\nplugins=queue/smtp_forward,dkim_sign\n[tls]\nkey=tls_key.pem\ncert=tls_cert.pem\n";
//! assert!(izanagi_kit::harakaconf::detect(b));
//! let c = izanagi_kit::harakaconf::Haraka::parse(b).unwrap();
//! assert!(c.sections >= 2);
//! ```
const KEYS: &[&str] = &[
    "listen",
    "smtputf8",
    "nodes",
    "user",
    "group",
    "daemonize",
    "daemon_log_file",
    "daemon_pid_file",
    "spool_dir",
    "max_children",
    "connection_timeout",
    "close_gracefully",
    "smtp_port",
    "smtps_port",
    "cluster_http_port",
    "inbound_timeout",
    "outbound_timeout",
    "header_timeout",
    "max_mime_parts",
    "max_received_headers",
    "max_headers",
    "max_line_length",
    "max_data_line_length",
    "strict_rfc1869",
    "me",
    "deny_includes_uuid",
    "early_talker",
    "banner",
    "include_milliseconds",
    "authentication_required",
    "auth_timeout",
    "smtpgreeting",
    "broken_auth_clients",
    "main",
    "spool_after",
    "timeout",
    "timeout_start_data",
    "idle_timeout",
    "max_unrecognized_commands",
    "bad_command_limit",
    "message_start_timeout",
    "smtpgreeting_includes_uuid",
    "block_me",
    "whitelist_exceptions",
    "host_header",
    "port",
    "host",
    "key",
    "cert",
    "key_password",
    "requestCert",
    "rejectUnauthorized",
    "secureProtocol",
    "ciphers",
    "dhparam",
    "honorCipherOrder",
    "no_tls_hosts",
    "hide_version",
    "gracedelay",
    "queue_dir",
    "outbound_enable_tls",
    "always_ok",
    "check_mx",
    "dns",
    "relay_all",
    "relay_domains",
    "protocols",
    "priv_passkey",
    "cleanup_headers",
    "send_mail",
];

/// Detect a Haraka INI config or plugin list.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut hits = 0usize;
    for k in KEYS {
        if t.contains(k) {
            hits += 1;
        }
    }
    let plugin_lines = t
        .lines()
        .filter(|l| {
            let tr = l.trim();
            !tr.is_empty()
                && !tr.starts_with(';')
                && !tr.starts_with('#')
                && !tr.contains('=')
                && !tr.contains('[')
                && (tr.contains("queue/") || tr.contains("dkim") || tr.contains("spamassassin"))
        })
        .count();
    (t.contains("[core]") || t.contains("[tls]") || hits >= 4) || plugin_lines >= 2
}

/// Structural counts for a Haraka config.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Haraka {
    /// Recognized key occurrences.
    pub keys: usize,
    /// `[name]` INI sections.
    pub sections: usize,
    /// `key=value` lines.
    pub assignments: usize,
    /// `config/plugins`-style bare plugin lines.
    pub plugins: usize,
    /// `;`/`#` comment lines.
    pub comments: usize,
}

impl Haraka {
    /// Parse structural counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            sections: 0,
            assignments: 0,
            plugins: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with(';') || tr.starts_with('#') {
                c.comments += 1;
            } else if tr.starts_with('[') && tr.ends_with(']') {
                c.sections += 1;
            } else if tr.contains('=') {
                c.assignments += 1;
            } else if !tr.is_empty()
                && !tr.contains(' ')
                && (tr.contains('/')
                    || tr.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_'))
            {
                c.plugins += 1;
            }
        }
        for k in KEYS {
            c.keys += t.matches(k).count();
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"[core]\nlisten=:::25\nsmtputf8=true\nmax_children=10\n[tls]\nkey=tls_key.pem\ncert=tls_cert.pem\n";
        assert!(detect(b));
        let c = Haraka::parse(b).unwrap();
        assert_eq!(c.sections, 2);
        assert_eq!(c.assignments, 5);
        assert!(c.keys >= 7);
    }

    #[test]
    fn detects_plugins_list() {
        let b = b"queue/smtp_forward\ndkim_sign\nspamassassin\n";
        assert!(detect(b));
        let c = Haraka::parse(b).unwrap();
        assert_eq!(c.plugins, 3);
    }

    #[test]
    fn rejects_ini() {
        let b = b"[section]\nfoo=bar\n";
        assert!(!detect(b));
        assert!(Haraka::parse(b).is_none());
    }
}
