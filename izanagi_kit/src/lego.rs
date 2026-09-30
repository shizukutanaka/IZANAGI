//! Census of a lego ACME-client configuration (INI/env file).
//!
//! lego is usually configured via environment variables (`LEGO_*`,
//! `*_API_KEY`, `*_SECRET`, `*_PROPAGATION_TIMEOUT`, `*_POLLING_INTERVAL`,
//! `*_HTTP_TIMEOUT`, `*_TTL`, `EXEC_*`, `MANUAL_*`, `HTTPREQ_*`,
//! `WEBROOT_*`, `ALICLOUD_*`, `CLOUDFLARE_*`, `ROUTE53_*`, `GCE_*`,
//! `AZURE_*`, `OVH_*`, `DO_TOKEN`…) and via `--dns.resolvers`/`--email`/
//! `--key-type`/`--path`/`--server` CLI flags; this module parses a
//! `.env`/INI `KEY=value` file: `LEGO_EMAIL`/`LEGO_SERVER`/`LEGO_PATH`/
//! `LEGO_ACCOUNT_KEY`/`LEGO_KEY_TYPE`/`LEGO_CERT`/`LEGO_CERT_KEY`/
//! `LEGO_CA_SERVER_NAME`/`LEGO_DISABLE_CNAME_SUPPORT`/`LEGO_EXPERIMENTAL_*`,
//! plus provider `*_API_KEY`/`*_API_SECRET`/`*_API_TOKEN`/`*_PROPAGATION_TIMEOUT`/
//! `*_POLLING_INTERVAL`/`*_HTTP_TIMEOUT`/`*_TTL` suffixes, `export KEY=value`,
//! `key = value` (INI), `[section]` headers, `#`/`;` comments.
//!
//! ```rust
//! let c = izanagi_kit::lego::Lego::parse(
//!     b"LEGO_EMAIL=ops@example.com\nLEGO_KEY_TYPE=ec256\nCLOUDFLARE_API_KEY=x\n",
//! ).unwrap();
//! assert_eq!(c.assignments, 3);
//! ```
#![forbid(unsafe_code)]

/// Lego config census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lego {
    /// `KEY=value`/`export KEY=value`/`key = value` assignments.
    pub assignments: usize,
    /// `[section]` headers.
    pub sections: usize,
    /// `LEGO_*` runtime keys.
    pub lego_keys: usize,
    /// Provider credential/timeout suffix keys.
    pub provider_keys: usize,
    /// `#`/`;` comment lines.
    pub comments: usize,
}

/// Provider-ish suffixes.
const SUFFIX: &[&str] = &[
    "_API_KEY",
    "_API_SECRET",
    "_API_TOKEN",
    "_SECRET",
    "_TOKEN",
    "_PROPAGATION_TIMEOUT",
    "_POLLING_INTERVAL",
    "_HTTP_TIMEOUT",
    "_TTL",
    "_SERVICE_ACCOUNT_FILE",
    "_ZONE",
    "_CREDENTIALS",
    "_ENDPOINT",
    "_BASE_URL",
];

/// True if `b` looks like a lego env/ini file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("LEGO_") || t.contains("PROPAGATION_TIMEOUT") || t.contains("POLLING_INTERVAL"))
        && t.contains('=')
}

impl Lego {
    /// Parse a lego env/ini file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            assignments: 0,
            sections: 0,
            lego_keys: 0,
            provider_keys: 0,
            comments: 0,
        };
        for line in t.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with('#') || l.starts_with(';') {
                c.comments += 1;
            } else if l.starts_with('[') && l.ends_with(']') {
                c.sections += 1;
            } else {
                let s = l.strip_prefix("export ").unwrap_or(l);
                if let Some(eq) = s.find('=') {
                    c.assignments += 1;
                    let key = s[..eq].trim();
                    if key.starts_with("LEGO_") {
                        c.lego_keys += 1;
                    } else if SUFFIX.iter().any(|x| key.ends_with(x)) {
                        c.provider_keys += 1;
                    }
                }
            }
        }
        if c.assignments == 0 {
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
            "# lego env\n",
            "LEGO_EMAIL=ops@example.com\n",
            "LEGO_SERVER=https://acme-v02.api.letsencrypt.org/directory\n",
            "LEGO_KEY_TYPE=ec256\n",
            "LEGO_PATH=/etc/lego\n",
            "CLOUDFLARE_API_KEY=abc\n",
            "CLOUDFLARE_PROPAGATION_TIMEOUT=120\n",
            "CLOUDFLARE_POLLING_INTERVAL=2\n",
            "ROUTE53_TTL=60\n",
            "export DO_TOKEN=xyz\n",
        )
        .to_string()
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = Lego::parse(b.as_bytes()).unwrap();
        assert_eq!(c.assignments, 9);
        assert_eq!(c.lego_keys, 4);
        assert_eq!(c.provider_keys, 5);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[x]\na=b\n"));
        assert!(Lego::parse(b"# none\n").is_none());
    }
}
