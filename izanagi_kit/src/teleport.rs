//! Teleport `teleport.yaml` config census.
//!
//! Teleport top-level keys: `teleport`, `auth_service`,
//! `proxy_service`, `ssh_service`, `kubernetes_service`,
//! `db_service`, `app_service`, `windows_desktop_service`,
//! `discovery_service`, `okta_service`, `jamf_service`,
//! `eice_service`, `version`, `storage`, `ad`, `cas`,
//! `oidc`, `saml`, `github`.
//!
//! ```rust
//! let k = b"teleport:\n  nodename: node1\nauth_service:\n  enabled: yes\nproxy_service:\n  enabled: yes\nssh_service:\n  enabled: yes\n";
//! assert!(izanagi_kit::teleport::detect(k));
//! ```

/// Teleport config census.
#[derive(Debug, Clone)]
pub struct Teleport {
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
    "teleport",
    "auth_service",
    "proxy_service",
    "ssh_service",
    "kubernetes_service",
    "db_service",
    "app_service",
    "windows_desktop_service",
    "discovery_service",
    "okta_service",
    "jamf_service",
    "eice_service",
];

const WEAK: &[&str] = &[
    "version",
    "storage",
    "ad",
    "cas",
    "oidc",
    "saml",
    "github",
    "auth",
    "kubernetes",
    "databases",
    "apps",
    "windows_desktop",
    "discovery",
];

fn top_key(line: &str) -> Option<&str> {
    if line.starts_with(' ') || line.starts_with('\t') || line.starts_with('-') {
        return None;
    }
    let s = line.trim_end();
    if s.starts_with('#') {
        return None;
    }
    match s.find(':') {
        Some(i) => Some(s[..i].trim()),
        None => None,
    }
}

/// Detect a Teleport `teleport.yaml`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
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

impl Teleport {
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
        let b = b"teleport:\n  nodename: node1\nauth_service:\n  enabled: yes\nproxy_service:\n  enabled: yes\nssh_service:\n  enabled: yes\n";
        assert!(detect(b));
        let c = Teleport::parse(b).unwrap();
        assert!(c.top_keys >= 2);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"storage:\n  type: etcd\n"));
        assert!(!detect(b"# auth_service:\n# proxy_service:\n"));
    }
}
