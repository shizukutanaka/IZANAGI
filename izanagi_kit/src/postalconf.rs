//! Postal `postal.yml` (YAML mail delivery platform config).
//!
//! ```
//! let b = b"web:\n  host: postal.example.com\n  protocol: https\nmain_db:\n  host: localhost\n  username: postal\n  password: p0st@l\n  database: postal\nmessage_db:\n  host: localhost\n  username: postal\n  database: postal-server-1\nsmtp:\n  host: 127.0.0.1\n  port: 2525\n";
//! assert!(izanagi_kit::postalconf::detect(b));
//! let c = izanagi_kit::postalconf::Postal::parse(b).unwrap();
//! assert!(c.sections >= 4);
//! ```
const SECTION_KEYS: &[&str] = &[
    "web:",
    "smtp_server:",
    "dns:",
    "rails:",
    "smtp:",
    "general:",
    "main_db:",
    "message_db:",
    "logging:",
    "rspamd:",
    "clamav:",
    "spam:",
    "fast_server:",
    "web_server:",
    "worker:",
    "smtp_client:",
    "smtp_relay:",
    "gelf:",
    "push:",
    "lets_encrypt:",
    "ip_pool:",
    "ip_pools:",
    "tracking:",
    "monitoring:",
    "metrics:",
    "migrations:",
];
const KEYS: &[&str] = &[
    "host",
    "protocol",
    "port",
    "username",
    "password",
    "database",
    "name",
    "domain",
    "private_key_path",
    "return_path",
    "route_domain",
    "track_domain",
    "helo_name",
    "relay_hostname",
    "relay_ips",
    "mx_records",
    "smtp_port",
    "spf_include",
    "dmarc_record",
    "dkim_private_key_path",
    "ssl_port",
    "max_message_size",
    "default_pool",
    "suppress_response_recipient",
    "use_ip_pools",
    "log_filename",
    "log_level",
    "syslog_enabled",
    "syslog_facility",
    "enabled",
    "proxies",
    "default_bind_address",
    "health_server_port",
    "quiet",
    "spawn_timeout",
    "timeout",
    "default_helo_name",
    "open_timeout",
    "read_timeout",
    "write_timeout",
    "ssl_key_path",
    "ssl_cert_path",
    "ssl_protocols",
    "ssl_ciphers",
    "ssl_version",
    "ssl_verify_mode",
    "ssl_cert_chain_file",
    "max_threads",
    "threads",
    "environment",
    "secret_key",
    "first_page_heading",
    "first_page_tagline",
    "default_sender",
    "url",
    "path",
    "hostnames",
    "cluster_name",
    "api_key",
    "verify_ssl",
    "app_name",
    "devise_secret_key",
    "initial_root_domain",
    "from_address",
    "from_name",
    "auto_provision_server",
    "default_delivery_pool",
    "retry_period",
    "hold_period",
    "max_send_rate",
    "max_message_retention",
    "web_delivery_options",
    "global_rate_limit",
];

/// Detect postal.yml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut hits = 0usize;
    for l in t.lines() {
        let tr = l.trim_end();
        if !l.starts_with(' ') && !l.starts_with('\t') && SECTION_KEYS.contains(&tr) {
            hits += 1;
        }
    }
    hits >= 2
}

/// Structural counts for postal.yml.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Postal {
    /// Recognized key occurrences.
    pub keys: usize,
    /// Top-level `name:` and nested `name:` YAML sections.
    pub sections: usize,
    /// `name: value` leaf lines.
    pub leaves: usize,
    /// `#` comment lines.
    pub comments: usize,
}

impl Postal {
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
            leaves: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
            } else if (SECTION_KEYS.contains(&tr) && !l.starts_with(' ') && !l.starts_with('\t'))
                || (tr.ends_with(':') && !tr.starts_with('-'))
            {
                c.sections += 1;
            } else if tr.contains(':') && !tr.starts_with('-') {
                c.leaves += 1;
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
        let b = b"web:\n  host: postal.example.com\n  protocol: https\nmain_db:\n  host: localhost\n  username: postal\n  password: p0st@l\n  database: postal\nmessage_db:\n  host: localhost\n  database: postal-server-1\nsmtp:\n  host: 127.0.0.1\n  port: 2525\ndns:\n  mx_records: mx.postal.example.com\n";
        assert!(detect(b));
        let c = Postal::parse(b).unwrap();
        assert_eq!(c.sections, 5);
        assert!(c.leaves >= 10);
        assert!(c.keys >= 12);
    }

    #[test]
    fn rejects_yaml() {
        let b = b"foo:\n  a: 1\nbar:\n  b: 2\n";
        assert!(!detect(b));
        assert!(Postal::parse(b).is_none());
    }
}
