//! Census of a certbot `cli.ini`/`letsencrypt.ini` file.
//!
//! `key = value` INI-style lines without sections: `server`,
//! `email`, `domains`/`domain`, `authenticator`/`installer`,
//! `webroot`/`webroot-path`/`webroot-map`, `standalone`/`apache`/
//! `nginx`/`manual`/`dns-*` plugin flags, `cert-name`,
//! `key-type`/`rsa-key-size`/`elliptic-curve`/`curve`,
//! `must-staple`, `staple-ocsp`, `uir`, `redirect`/`no-redirect`/
//! `hsts`/`no-hsts`, `agree-tos`, `register-unsafely-without-email`,
//! `preferred-challenges`, `pre-hook`/`post-hook`/`deploy-hook`/
//! `renew-hook`, `work-dir`/`logs-dir`/`config-dir`,
//! `dry-run`/`staging`/`test-cert`/`force-renewal`/`csr`/
//! `reuse-key`/`no-reuse-key`/`http-01-port`/`http01-port`/
//! `break-my-certs`/`allow-subset-of-names`. `#` comments.
//!
//! ```rust
//! let c = izanagi_kit::certbot::Certbot::parse(
//!     b"email = me@example.com\nauthenticator = webroot\nwebroot-path = /var/www\n",
//! ).unwrap();
//! assert_eq!(c.keys, 1);
//! assert_eq!(c.auth, 2);
//! ```
#![forbid(unsafe_code)]

/// certbot cli.ini census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Certbot {
    /// `key = value` settings.
    pub keys: usize,
    /// `pre-hook`/`post-hook`/`deploy-hook`/`renew-hook` hooks.
    pub hooks: usize,
    /// `authenticator`/`installer`/`webroot`/`standalone`/`apache`/`nginx`/`manual`/`dns-*` auth lines.
    pub auth: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Hook keys.
const HOOKS: &[&str] = &["pre-hook", "post-hook", "deploy-hook", "renew-hook"];

/// Auth-related keys/flags.
const AUTH: &[&str] = &[
    "authenticator",
    "installer",
    "webroot",
    "webroot-path",
    "webroot-map",
    "standalone",
    "apache",
    "nginx",
    "manual",
    "dns-cloudflare",
    "dns-route53",
    "dns-digitalocean",
    "dns-cloudxns",
    "dns-dnsimple",
    "dns-google",
    "dns-rfc2136",
    "dns-luadns",
    "dns-ovh",
    "preferred-challenges",
];

/// True if `b` looks like certbot cli.ini.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("authenticator") || t.contains("webroot-path") || t.contains("letsencrypt"))
        || (t.contains("agree-tos") && t.contains("email"))
}

impl Certbot {
    /// Parse cli.ini into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            hooks: 0,
            auth: 0,
            comments: 0,
        };
        for line in t.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with('#') {
                c.comments += 1;
                continue;
            }
            let Some(eq) = l.find('=') else {
                continue;
            };
            let key = l[..eq].trim();
            if HOOKS.contains(&key) {
                c.hooks += 1;
            } else if AUTH.contains(&key) || key.starts_with("dns-") {
                c.auth += 1;
            } else {
                c.keys += 1;
            }
        }
        if c.keys + c.auth + c.hooks == 0 {
            return None;
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> String {
        concat!(
            "# certbot\n",
            "email = me@example.com\n",
            "authenticator = webroot\n",
            "webroot-path = /var/www\n",
            "domains = a.com,b.com\n",
            "key-type = ecdsa\n",
            "deploy-hook = systemctl reload nginx\n",
            "agree-tos = true\n",
        )
        .to_string()
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = Certbot::parse(b.as_bytes()).unwrap();
        assert_eq!(c.keys, 4);
        assert_eq!(c.auth, 2);
        assert_eq!(c.hooks, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[server]\nport = 1\n"));
        assert!(Certbot::parse(b"# none\n").is_none());
    }
}
