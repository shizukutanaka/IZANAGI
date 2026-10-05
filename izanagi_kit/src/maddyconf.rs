//! maddy `maddy.conf` (exim-like module block syntax).
//!
//! ```
//! let b = b"hostname mx.example.org\ntls file /etc/maddy/certs/fullchain.pem /etc/maddy/certs/privkey.pem\nsmtp tcp://0.0.0.0:25 {\n  limits {\n    all rate 20 1s\n  }\n}\nsubmission tls://0.0.0.0:465 tcp://0.0.0.0:587 {\n  auth &local_authdb\n}\n";
//! assert!(izanagi_kit::maddyconf::detect(b));
//! let c = izanagi_kit::maddyconf::Maddy::parse(b).unwrap();
//! assert!(c.module_blocks >= 2);
//! ```
const MODULE_HEADS: &[&str] = &[
    "smtp",
    "submission",
    "imap",
    "endpoints",
    "delivery",
    "storage.imapsql",
    "storage.maildirs",
    "auth.pass_table",
    "auth.shadow",
    "auth.pam",
    "auth.sasl",
    "auth.external",
    "auth.fallback",
    "tls",
    "limits",
    "log",
    "checks",
    "check",
    "modify",
    "replace",
    "rewrite",
    "routing",
    "lookup",
    "match",
    "rcpt_check",
    "sender_check",
    "dmar\u{63}",
    "dkim",
    "mx_check",
    "auth_check",
    "pipeline",
    "msgpipeline",
    "dummy",
    "debug",
];
const KEYS: &[&str] = &[
    "hostname",
    "primary_domain",
    "tls",
    "state_dir",
    "runtime_dir",
    "log",
    "hostname_verification",
    "limits",
    "all",
    "ip",
    "domain",
    "rate",
    "concurrency",
    "inbound",
    "autoguess",
    "auth",
    "lmtp",
    "delivery",
    "target",
    "sender",
    "rcpt",
    "domains",
    "destinations",
    "rewrite",
    "reject",
    "accept",
    "filter",
    "defer",
    "file",
    "args",
    "stdin",
    "stdout",
    "stderr",
    "timeout",
    "required_score",
    "header",
    "score_threshold",
    "mem_limit",
    "local_authdb",
    "local_storage",
    "local_finals",
    "local_routing",
    "local_checks",
    "remote",
    "default",
    "match",
    "lookup",
    "dnsbl",
    "policy",
    "require_tls_match",
    "mx_check",
    "dns_resolver",
    "require_mx_match",
    "earlydns",
    "conn_check",
    "sender_match",
    "rcpt_check",
    "error",
    "perm_error",
    "quarantine",
    "debug",
    "verbose",
    "modular",
    "io_debug",
    "outbound",
    "insecure",
    "buffer",
    "imap",
    "sql",
    "driver",
    "dsn",
    "junk_mailbox",
    "specialuse",
    "compress",
    "endpoint",
    "auth_map",
    "msgid_domain",
    "write_timeout",
    "read_timeout",
    "enable_login",
    "enable_auth",
    "insecure_auth",
    "lazy_expunge",
];

/// Detect maddy.conf.
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
    let module = t.lines().any(|l| {
        let tr = l.trim_start();
        MODULE_HEADS.iter().any(|m| tr.starts_with(m)) && tr.contains("://") && tr.contains('{')
    });
    module || (t.contains("hostname") && hits >= 5)
}

/// Structural counts for maddy.conf.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Maddy {
    /// Recognized key occurrences.
    pub keys: usize,
    /// `<module> <endpoint> {` blocks (smtp/submission/imap/...).
    pub module_blocks: usize,
    /// Bare directive lines inside/outside blocks.
    pub directives: usize,
    /// `#` comment lines.
    pub comments: usize,
}

impl Maddy {
    /// Parse structural counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            module_blocks: 0,
            directives: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
            } else if MODULE_HEADS.iter().any(|m| tr.starts_with(m))
                && tr.contains("://")
                && tr.contains('{')
            {
                c.module_blocks += 1;
            } else if !tr.is_empty() && tr != "}" && !tr.starts_with('&') {
                c.directives += 1;
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
        let b = b"hostname mx.example.org\ntls file /etc/maddy/certs/fullchain.pem /etc/maddy/certs/privkey.pem\nstate_dir /var/lib/maddy\nruntime_dir /run/maddy\nlog stderr_ts debug\nsmtp tcp://0.0.0.0:25 {\n  dmarc yes\n}\nsubmission tls://0.0.0.0:465 tcp://0.0.0.0:587 {\n  auth &local_authdb\n}\nimap tcp://0.0.0.0:143 tls://0.0.0.0:993 {\n  auth &local_authdb\n  storage &local_mailboxes\n}\n";
        assert!(detect(b));
        let c = Maddy::parse(b).unwrap();
        assert_eq!(c.module_blocks, 3);
        assert!(c.directives >= 8);
        assert!(c.keys >= 10);
    }

    #[test]
    fn rejects_nginx() {
        let b = b"server {\n  listen 80;\n}\n";
        assert!(!detect(b));
        assert!(Maddy::parse(b).is_none());
    }
}
