//! Privoxy `config` census.
//!
//! Privoxy config is flat `directive value` lines (`#` comments):
//! `listen-address 127.0.0.1:8118`, `actionsfile match-all.action`,
//! `filterfile default.filter`, `forward-socks5 / 127.0.0.1:9050 .`,
//! `toggle 1`, `logdir`, `confdir`, `debug`, `enforce-blocks`.
//!
//! ```rust
//! let k = b"confdir /etc/privoxy\nlogdir /var/log/privoxy\nlisten-address 127.0.0.1:8118\nactionsfile match-all.action\nfilterfile default.filter\n";
//! assert!(izanagi_kit::privoxy::detect(k));
//! ```

/// Privoxy config census.
#[derive(Debug, Clone)]
pub struct Privoxy {
    /// `directive value` lines.
    pub settings: usize,
    /// recognised directives present.
    pub keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const STRONG: &[&str] = &[
    "listen-address",
    "actionsfile",
    "filterfile",
    "toggle",
    "enable-remote-toggle",
    "enable-edit-actions",
    "enforce-blocks",
    "forward-socks4a",
    "forward-socks5",
    "forward-socks5t",
    "forwarded-connect-retries",
    "buffer-limit",
    "accept-intercepted-requests",
    "keep-alive-timeout",
    "socket-timeout",
    "single-threaded",
    "permit-access",
    "deny-access",
    "intercept-only",
    "default-server-timeout",
    "connection-sharing",
    "split-large-forms",
    "allow-cgi-request-crunching",
];

const WEAK: &[&str] = &[
    "confdir",
    "templdir",
    "temporary-directory",
    "logdir",
    "logfile",
    "jarfile",
    "debug",
    "user-manual",
    "trust-info-url",
    "admin-address",
    "proxy-info-url",
    "hostname",
    "receive-buffer-size",
    "enable-proxy-authentication-forwarding",
    "handle-as-empty-doc-returns-ok",
    "handle-as-image",
];

fn directive(line: &str) -> Option<&str> {
    let s = line.trim();
    if s.is_empty() || s.starts_with('#') {
        return None;
    }
    let k = s.split([' ', '\t', ':']).next()?.trim();
    if k.is_empty() {
        None
    } else {
        Some(k)
    }
}

/// Detect a Privoxy `config` file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // Privoxy directive names (`actionsfile`, `forward-socks5`, …)
    // are exclusive; shared keys only count with a strong anchor.
    let mut strong = 0usize;
    let mut weak = 0usize;
    for line in t.lines() {
        if let Some(k) = directive(line) {
            if STRONG.contains(&k) {
                strong += 1;
            } else if WEAK.contains(&k) {
                weak += 1;
            }
        }
    }
    strong >= 2 || (strong >= 1 && weak >= 2)
}

impl Privoxy {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            settings: 0,
            keys: 0,
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
            c.settings += 1;
            if let Some(k) = directive(line) {
                if STRONG.contains(&k) || WEAK.contains(&k) {
                    c.keys += 1;
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
        let b = b"confdir /etc/privoxy\nlogdir /var/log/privoxy\nlisten-address 127.0.0.1:8118\nactionsfile match-all.action\nfilterfile default.filter\n";
        assert!(detect(b));
        let c = Privoxy::parse(b).unwrap();
        assert!(c.keys >= 3);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"logdir /var/log/x\nconfdir /etc/x\n"));
        assert!(!detect(
            b"# listen-address 127.0.0.1:8118\n# actionsfile x\nlogdir /y\n"
        ));
    }
}
