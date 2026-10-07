//! ZoneMTA `zonemta.toml`.
//!
//! ```
//! let b = b"[api]\nhost = \"127.0.0.1\"\nport = 8080\n[smtp]\nenabled = [\"smtp\", \"smtps\"]\n[smtp.feed]\nenabled = true\nport = 2525\n[zones.default]\npool = \"default\"\n[dbs]\nserver = \"leveldb\"\n";
//! assert!(izanagi_kit::zonemtaconf::detect(b));
//! let c = izanagi_kit::zonemtaconf::Zonemta::parse(b).unwrap();
//! assert!(c.sections >= 5);
//! ```
const SECTION_KEYS: &[&str] = &[
    "[api]",
    "[smtp]",
    "[smtps]",
    "[zones]",
    "[plugins]",
    "[dbs]",
    "[dns]",
    "[queue]",
    "[log]",
    "[user]",
    "[imap]",
    "[pops]",
    "[feeder]",
    "[feeder.breaker]",
    "[pools]",
    "[mx-route]",
    "[metrics]",
    "[tracing]",
    "[body-structure]",
];
const KEYS: &[&str] = &[
    "host",
    "port",
    "enabled",
    "authentication",
    "queueTimeout",
    "maxRecipients",
    "recipientLimit",
    "connections",
    "sendAttempts",
    "minBackoff",
    "maxBackoff",
    "maxConnTime",
    "zones",
    "pool",
    "processes",
    "connection",
    "dedicated",
    "disabled",
    "queue",
    "plugins",
    "leveldb",
    "redis",
    "mongo",
    "server",
    "senderDomains",
    "rewriteDomains",
    "allowSmtp",
    "mxPort",
    "mxHost",
    "logLevel",
    "loggelf",
    "mailingLists",
    "rewrite",
    "source",
    "dkim",
    "notifySpam",
    "rejectingMailFrom",
    "throttling",
    "default_ip",
    "auths",
    "delivery",
    "headers",
    "skipPlugins",
    "relaying",
    "zone",
    "from",
    "to",
    "c\u{63}",
    "bc\u{63}",
    "subject",
    "messageId",
    "dkim.domainName",
    "dkim.keySelector",
    "returnPath",
    "date",
    "starttls",
    "ignoreTLS",
    "requireTLS",
    "cert",
    "key",
    "ca",
    "DHparam",
    "disableREVERSE",
    "disableHTTP",
    "apiKeys",
    "userPasswords",
    "logEntries",
    "storeAll",
];

/// ZoneMTA-specific markers generic TOML configs never carry.
const STRONG: &[&str] = &[
    "[zones.",
    "[smtp.feed]",
    "[feeder",
    "mxHost",
    "mxPort",
    "queueTimeout",
    "senderDomains",
    "rewriteDomains",
];

fn section_line(t: &str, name: &str) -> bool {
    t.lines().any(|l| l.trim_start().starts_with(name))
}

fn key_present(t: &str, k: &str) -> bool {
    // TOML keys are `key = value` lines; a bare substring of the key name
    // anywhere in the file does not count.
    t.lines().any(|l| {
        l.trim_start()
            .strip_prefix(k)
            .is_some_and(|r| r.trim_start().starts_with('='))
    })
}

/// Detect zonemta.toml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    if STRONG.iter().any(|k| t.contains(k)) {
        return true;
    }
    let section_hits = SECTION_KEYS.iter().filter(|s| section_line(t, s)).count();
    let key_hits = KEYS.iter().filter(|k| key_present(t, k)).count();
    section_hits >= 2 && key_hits >= 5
}

/// Structural counts for zonemta.toml.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Zonemta {
    /// Recognized key occurrences.
    pub keys: usize,
    /// `[table]` TOML tables.
    pub sections: usize,
    /// `key = value` lines.
    pub assignments: usize,
    /// `#` comment lines.
    pub comments: usize,
}

impl Zonemta {
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
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"[api]\nhost = \"127.0.0.1\"\nport = 8080\n[smtp]\nenabled = [\"smtp\", \"smtps\"]\n[smtp.feed]\nenabled = true\nport = 2525\n[zones.default]\npool = \"default\"\n[dbs]\nserver = \"leveldb\"\n";
        assert!(detect(b));
        let c = Zonemta::parse(b).unwrap();
        assert_eq!(c.sections, 5);
        assert_eq!(c.assignments, 7);
        assert!(c.keys >= 8);
    }

    #[test]
    fn rejects_toml() {
        let b = b"[package]\nname = \"foo\"\n";
        assert!(!detect(b));
        assert!(Zonemta::parse(b).is_none());
        // Generic sections plus generic keys are not zonemta.
        let g = b"[api]\nhost = \"x\"\nport = 1\n[dns]\nserver = \"y\"\nenabled = true\n";
        assert!(!detect(g));
    }
}
