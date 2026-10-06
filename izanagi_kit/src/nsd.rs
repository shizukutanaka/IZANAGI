//! NSD `nsd.conf` config census.
//!
//! NSD config is YAML-ish top-level sections: `server:`, `zone:`,
//! `pattern:`, `key:`, `remote-control:`, `dnstap:`, `tls:`,
//! `verify:` with indented `name`/`zonefile`/`ip-address`/
//! `notify`/`provide-xfr`/`request-xfr`/`include` subkeys.
//!
//! ```rust
//! let k = b"server:\n    ip-address: 127.0.0.1\n    zonefile: \"%s.zone\"\nzone:\n    name: example.com\n    zonefile: example.com.zone\nremote-control:\n    control-enable: yes\n";
//! assert!(izanagi_kit::nsd::detect(k));
//! ```

/// Nsd config census.
#[derive(Debug, Clone)]
pub struct Nsd {
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
    "zone",
    "pattern",
    "remote-control",
    "dnstap",
    "tls",
    "verify",
    "key",
    "tsig",
];

const WEAK: &[&str] = &[
    "server",
    "include",
    "nsd-checkconf",
    "verbosity",
    "logfile",
    "pidfile",
    "database",
    "zonelistfile",
    "username",
    "chroot",
    "statistics",
    "xfrdfile",
    "xfrdir",
    "rrl-size",
    "rrl-ratelimit",
    "hide-version",
    "hide-identity",
    "version",
    "ip-address",
    "ipv4-edns-size",
    "ipv6-edns-size",
    "reuseport",
    "tcp-count",
    "server-count",
    "round-robin",
    "minimal-responses",
    "confine-to-zone",
    "refuse-any",
    "zonefiles-check",
    "zonefiles-write",
    "database-diffmode",
    "drop-updates",
    "answer-aaaa",
    "assert-sigexpired",
    "debug-mode",
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

/// Detect an `nsd.conf`.
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
    strong >= 2 || (strong >= 1 && weak >= 2)
}

impl Nsd {
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
        let b = b"server:\n    ip-address: 127.0.0.1\n    zonefile: \"%s.zone\"\nzone:\n    name: example.com\n    zonefile: example.com.zone\nremote-control:\n    control-enable: yes\n";
        assert!(detect(b));
        let c = Nsd::parse(b).unwrap();
        assert!(c.top_keys >= 2);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"server:\n  x: 1\ninclude:\n  y: 2\n"));
        assert!(!detect(b"# zone:\n# pattern:\nserver:\n  x: 1\n"));
    }
}
