//! ZITADEL runtime config YAML census.
//!
//! ZITADEL config top-level keys: `ExternalDomain`,
//! `ExternalPort`, `ExternalSecure`, `TLS`, `Database`,
//! `Machine`, `Log`, `Tracing`, `Metrics`,
//! `SystemDefaults`, `DefaultInstance`, `Quotas`,
//! `Limits`, `Restrictions`, `Console`, `Login`, `OIDC`,
//! `SAML`, `IdP`, `InternalAuthZ`, `Eventstore`, `WebAuthN`,
//! `Actions`, `Notifications`.
//!
//! ```rust
//! let k = b"ExternalDomain: zitadel.example.com\nExternalPort: 443\nDatabase:\n  postgres:\n    Host: db\n";
//! assert!(izanagi_kit::zitadel::detect(k));
//! ```

/// ZITADEL config census.
#[derive(Debug, Clone)]
pub struct Zitadel {
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
    "ExternalDomain",
    "ExternalPort",
    "ExternalSecure",
    "SystemDefaults",
    "DefaultInstance",
    "Quotas",
    "Restrictions",
    "Console",
    "Login",
    "InternalAuthZ",
    "WebAuthN",
    "Actions",
];

const WEAK: &[&str] = &[
    "Database",
    "Machine",
    "Log",
    "Tracing",
    "Metrics",
    "TLS",
    "OIDC",
    "SAML",
    "IdP",
    "Limits",
    "Eventstore",
    "Notifications",
    "Caches",
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

/// Detect a ZITADEL config.
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

impl Zitadel {
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
        let b = b"ExternalDomain: zitadel.example.com\nExternalPort: 443\nDatabase:\n  postgres:\n    Host: db\n";
        assert!(detect(b));
        let c = Zitadel::parse(b).unwrap();
        assert!(c.top_keys >= 2);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"Database:\n  postgres:\n    Host: db\n"));
        assert!(!detect(b"# ExternalDomain: x\nLog: y\n"));
    }
}
