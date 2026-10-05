//! Stalwart Mail Server `config.toml`.
//!
//! ```
//! let b = b"server.hostname = \"mail.example.com\"\nserver.tls.certificate = \"default\"\nserver.listener.smtp.bind = \"[::]:25\"\nstorage.data = \"rocksdb\"\nstorage.blob = \"s3\"\nimap.auth.allow-plain-text = false\n";
//! assert!(izanagi_kit::stalwartconf::detect(b));
//! let c = izanagi_kit::stalwartconf::Stalwart::parse(b).unwrap();
//! assert!(c.keys >= 6);
//! ```
const FAMILIES: &[&str] = &[
    "server.",
    "imap.",
    "jmap.",
    "sieve.",
    "spam-filter.",
    "spamd.",
    "storage.",
    "store.",
    "directory.",
    "tracer.",
    "metrics.",
    "cluster.",
    "enterprise.",
    "webhooks.",
    "oauth.",
    "report.",
    "resolver.",
    "remote.",
    "queue.",
    "session.",
    "auth.",
    "signature.",
    "certificate.",
    "acme.",
    "snmp.",
    "managesieve.",
    "http.",
    "smtp.",
    "lmtp.",
    "pop3.",
];
const KEYS: &[&str] = &[
    "server.hostname",
    "server.tls",
    "server.listener",
    "server.max-network-requests",
    "server.sockets",
    "server.thread-pool",
    "server.proxy",
    "server.http",
    "server.permissive-cors",
    "server.x-forwarded",
    "imap.auth",
    "imap.request",
    "imap.socket",
    "imap.protocol",
    "jmap.protocol",
    "jmap.session",
    "jmap.auth",
    "jmap.rate-limit",
    "jmap.event-source",
    "jmap.push",
    "jmap.web-sockets",
    "jmap.upload",
    "jmap.mail",
    "sieve.limits",
    "sieve.untrusted",
    "sieve.from-name",
    "sieve.from-addr",
    "spam-filter.lists",
    "spam-filter.rules",
    "spam-filter.spam-trap",
    "spam-filter.bayes",
    "spam-filter.dnsbl",
    "spam-filter.surbl",
    "spam-filter.pyzor",
    "spam-filter.expiry",
    "spam-filter.trusted-domains",
    "storage.data",
    "storage.blob",
    "storage.lookup",
    "storage.fts",
    "storage.directory",
    "storage.encryption",
    "store.driver",
    "directory.type",
    "directory.cache",
    "tracer.method",
    "tracer.level",
    "tracer.ansi",
    "tracer.rotate",
    "metrics.history",
    "metrics.prometheus",
    "cluster.node-id",
    "cluster.bind-addr",
    "cluster.advertise-addr",
    "cluster.seed-nodes",
    "cluster.discovery",
    "enterprise.license",
    "oauth.key",
    "oauth.auth",
    "oauth.max-auth-attempts",
    "oauth.token",
    "report.analysis",
    "report.addresses",
    "resolver.dns",
    "resolver.attempts",
    "resolver.preserve-intermediates",
    "resolver.txt",
    "resolver.mx",
    "resolver.ip",
    "remote.smtp",
    "remote.protocol",
    "remote.tls",
    "remote.limits",
    "remote.ip-pool",
    "queue.schedule",
    "queue.report",
    "queue.max-num-attempts",
    "queue.requiretls",
    "session.ehlo",
    "session.timeout",
    "session.transfer-limit",
    "session.frequency",
    "session.rcpt",
    "session.data",
    "session.duration",
    "auth.mechanisms",
    "auth.require-tls",
    "signature.algorithms",
    "signature.headers",
    "signature.report",
    "certificate.default",
    "acme.provider",
    "acme.contact",
    "acme.directory",
];

/// Detect a Stalwart config.
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
    hits >= 2 || hits >= 1 && t.contains("server.listener")
}

/// Structural counts for a Stalwart config.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stalwart {
    /// Recognized key and family-prefix occurrences.
    pub keys: usize,
    /// `[table]` TOML tables.
    pub sections: usize,
    /// `key = value` lines.
    pub assignments: usize,
    /// `#` comment lines.
    pub comments: usize,
}

impl Stalwart {
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
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
            } else if tr.starts_with('[') && tr.ends_with(']') {
                c.sections += 1;
            } else if tr.contains('=') {
                c.assignments += 1;
            }
        }
        for k in KEYS {
            c.keys += t.matches(k).count();
        }
        for f in FAMILIES {
            c.keys += t.matches(f).count();
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"server.hostname = \"mail.example.com\"\nserver.tls.certificate = \"default\"\nserver.listener.smtp.bind = \"[::]:25\"\nstorage.data = \"rocksdb\"\nimap.auth.allow-plain-text = false\nspam-filter.lists.spamhaus = true\n";
        assert!(detect(b));
        let c = Stalwart::parse(b).unwrap();
        assert_eq!(c.assignments, 6);
        assert!(c.keys >= 10);
    }

    #[test]
    fn rejects_toml() {
        let b = b"[package]\nname = \"foo\"\nversion = \"1.0\"\n";
        assert!(!detect(b));
        assert!(Stalwart::parse(b).is_none());
    }
}
