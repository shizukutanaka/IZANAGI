//! AdGuard Home `AdGuardHome.yaml` config census.
//!
//! AdGuard Home top-level keys: `bind_host`, `bind_port`, `users`,
//! `dns`, `tls`, `dhcp`, `filtering`, `filters`, `whitelist_filters`,
//! `user_rules`, `querylog`, `statistics`, `clients`,
//! `auth_attempts`, `block_auth_min`, `http_proxy`, `theme`.
//!
//! ```rust
//! let k = b"bind_host: 0.0.0.0\nbind_port: 80\ndns:\n  bind_hosts: [0.0.0.0]\nfiltering:\n  protection_enabled: true\nquerylog:\n  enabled: true\n";
//! assert!(izanagi_kit::adguard::detect(k));
//! ```

/// Adguard config census.
#[derive(Debug, Clone)]
pub struct Adguard {
    /// `key:`/`key: value` lines.
    pub settings: usize,
    /// `- ` list items.
    pub items: usize,
    /// recognised top-level keys present.
    pub top_keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const STRONG: &[&str] = &[
    "querylog",
    "statistics",
    "whitelist_filters",
    "user_rules",
    "filtering",
    "auth_attempts",
    "block_auth_min",
    "http_api_pprof",
    "schema_version",
    "os_group",
    "os_user",
    "ratelimit",
    "ratelimit_whitelist",
    "refuse_any",
    "edns_client_subnet",
];

const WEAK: &[&str] = &[
    "bind_host",
    "bind_port",
    "users",
    "dns",
    "tls",
    "dhcp",
    "clients",
    "filters",
    "http_proxy",
    "language",
    "theme",
    "verbose",
    "log_compress",
    "log_localtime",
    "log_max_backups",
    "log_max_age",
    "log_max_size",
    "web_session_ttl",
    "debug_pprof",
];

fn top_key(line: &str) -> Option<&str> {
    if line.starts_with(' ') || line.starts_with('\t') || line.starts_with('-') {
        return None;
    }
    let s = line.trim_end();
    if s.is_empty() || s.starts_with('#') {
        return None;
    }
    match s.find(':') {
        Some(i) => {
            let k = s[..i].trim().trim_matches('"').trim_matches('\'');
            if k.is_empty() {
                None
            } else {
                Some(k)
            }
        }
        None => None,
    }
}

/// Detect an AdGuard Home `AdGuardHome.yaml`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // vendor-exclusive keys carry the weight; shared keys only count
    // once a strong anchor is present.
    let mut strong = 0usize;
    let mut weak = 0usize;
    for line in t.lines() {
        if let Some(k) = top_key(line) {
            if STRONG.contains(&k) {
                strong += 1;
            } else if WEAK.contains(&k) {
                weak += 1;
            }
        }
    }
    strong >= 1 && strong + weak >= 2
}

impl Adguard {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            settings: 0,
            items: 0,
            top_keys: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if s.starts_with("- ") {
                c.items += 1;
                continue;
            }
            if s.contains(':') {
                c.settings += 1;
                if let Some(k) = top_key(line) {
                    if STRONG.contains(&k) || WEAK.contains(&k) {
                        c.top_keys += 1;
                    }
                }
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"bind_host: 0.0.0.0\nbind_port: 80\ndns:\n  bind_hosts: [0.0.0.0]\nfiltering:\n  protection_enabled: true\nquerylog:\n  enabled: true\n";
        assert!(detect(b));
        let c = Adguard::parse(b).unwrap();
        assert!(c.top_keys >= 2);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"bind_host: 0.0.0.0\nbind_port: 80\n"));
        assert!(!detect(b"# filtering:\n# querylog:\ndns:\n  x: y\n"));
    }
}
