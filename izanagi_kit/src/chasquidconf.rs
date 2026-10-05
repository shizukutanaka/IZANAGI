//! chasquid `chasquid.conf` (textproto `key: "value"`).
//!
//! ```
//! let b = b"hostname: \"mx.example.com\"\nmax_clients: 1024\nsmtp_address: \":smtp\"\nsubmission_address: \":submission\"\nsubmission_over_tls_address: \":submissions\"\nmonitoring_address: \":1099\"\nmail_delivery_agent_bin: \"procmail\"\nmail_delivery_agent_args: \"-f\"\ndata_dir: \"/var/lib/chasquid\"\n";
//! assert!(izanagi_kit::chasquidconf::detect(b));
//! let c = izanagi_kit::chasquidconf::Chasquid::parse(b).unwrap();
//! assert_eq!(c.assignments, 9);
//! ```
const KEYS: &[&str] = &[
    "hostname",
    "max_clients",
    "smtp_address",
    "submission_address",
    "submission_over_tls_address",
    "monitoring_address",
    "mail_delivery_agent_bin",
    "mail_delivery_agent_args",
    "mail_log_path",
    "dovecot_auth",
    "haproxy_incoming",
    "data_dir",
    "suffix_separators",
    "drop_characters",
    "mail_log_format",
];

/// Detect chasquid.conf.
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
    hits >= 3
}

/// Structural counts for chasquid.conf.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chasquid {
    /// Recognized key occurrences.
    pub keys: usize,
    /// `key: "value"` lines.
    pub assignments: usize,
    /// `#` comment lines.
    pub comments: usize,
}

impl Chasquid {
    /// Parse structural counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            assignments: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
            } else if tr.contains(':') {
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
        let b = b"hostname: \"mx.example.com\"\nmax_clients: 1024\nsmtp_address: \":smtp\"\nsubmission_address: \":submission\"\nsubmission_over_tls_address: \":submissions\"\nmonitoring_address: \":1099\"\nmail_delivery_agent_bin: \"procmail\"\ndata_dir: \"/var/lib/chasquid\"\n";
        assert!(detect(b));
        let c = Chasquid::parse(b).unwrap();
        assert_eq!(c.assignments, 8);
        assert!(c.keys >= 8);
    }

    #[test]
    fn rejects_ini() {
        let b = b"[server]\nfoo=bar\n";
        assert!(!detect(b));
        assert!(Chasquid::parse(b).is_none());
    }
}
