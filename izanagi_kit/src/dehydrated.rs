//! Census of a dehydrated (`config`/`config.d/*.sh`) configuration.
//!
//! Shell-style `KEY=value` (and `export KEY=value`/`KEY="v"`) covering
//! dehydrated knobs: `CA`/`CA_TERMS`/`ACCOUNTDIR`/`ACCOUNT_KEY`/
//! `ACCOUNT_EMAIL`/`ACCOUNT_ID`/`CHALLENGETYPE`/`CHALLENGE_DIR`/`CERTDIR`/
//! `DOMAINS_D`/`DOMAINS_TXT`/`HOOK`/`HOOK_CHAIN`/`KEYSIZE`/`KEY_ALGO`/
//! `RENEW_DAYS`/`WELLKNOWN`/`CONTACT_EMAIL`/`IP_VERSION`/`BASEDIR`/`PIDFILE`/
//! `OPENSSL`/`LOCKFILE`/`API`/`PRIVKEY`/`CHAIN`/`FULLCHAIN`/`OCSP_MUST_STAPLE`/
//! `STAGING`/`DEHYDRATED_USER`/`DEHYDRATED_GROUP`, plus `declare`/array
//! assignments (`DOMAINS_TXT` lists as `foo.com bar.com` per line in domains.txt
//! count separately as `domains`), `#` comments.
//!
//! ```rust
//! let c = izanagi_kit::dehydrated::Dehydrated::parse(
//!     b"CA=https://acme-v02.api.letsencrypt.org/directory\nCHALLENGETYPE=http-01\n",
//! ).unwrap();
//! assert_eq!(c.assignments, 2);
//! ```
#![forbid(unsafe_code)]

/// Dehydrated configuration census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dehydrated {
    /// `KEY=value`/`export KEY=value`/`declare -x KEY=value` assignments.
    pub assignments: usize,
    /// Entries in an included `domains.txt`-style list (`name [alt …]` lines).
    pub domains: usize,
    /// `#` comment lines.
    pub comments: usize,
    /// Known dehydrated keys seen (`CA`/`CHALLENGETYPE`/…).
    pub known: usize,
}

/// Recognised dehydrated config keys.
const KEYS: &[&str] = &[
    "CA",
    "CA_TERMS",
    "ACCOUNTDIR",
    "ACCOUNT_KEY",
    "ACCOUNT_EMAIL",
    "ACCOUNT_ID",
    "CHALLENGETYPE",
    "CHALLENGE_DIR",
    "CERTDIR",
    "DOMAINS_D",
    "DOMAINS_TXT",
    "HOOK",
    "HOOK_CHAIN",
    "KEYSIZE",
    "KEY_ALGO",
    "RENEW_DAYS",
    "WELLKNOWN",
    "CONTACT_EMAIL",
    "IP_VERSION",
    "BASEDIR",
    "PIDFILE",
    "OPENSSL",
    "LOCKFILE",
    "API",
    "PRIVKEY",
    "CHAIN",
    "FULLCHAIN",
    "OCSP_MUST_STAPLE",
    "STAGING",
    "DEHYDRATED_USER",
    "DEHYDRATED_GROUP",
    "OUTPUT_DIR",
    "SUBDIR",
    "INTERMEDIATE",
    "ALPNPORT",
    "TMPDIR",
];

/// True if `b` looks like dehydrated config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("CHALLENGETYPE")
        || t.contains("CA_TERMS")
        || t.contains("WELLKNOWN")
        || t.contains("DOMAINS_TXT")
        || t.contains("acme-v0"))
        && t.contains('=')
}

impl Dehydrated {
    /// Parse a dehydrated config into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            assignments: 0,
            domains: 0,
            comments: 0,
            known: 0,
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
            let mut s = l;
            if let Some(r) = s.strip_prefix("export ") {
                s = r;
            } else if let Some(r) = s.strip_prefix("declare ") {
                // `declare -x/-a/-r KEY=…` etc.
                s = r.split_whitespace().next_back().unwrap_or(r);
            }
            if let Some(eq) = s.find('=') {
                c.assignments += 1;
                let key = s[..eq].trim();
                if KEYS.contains(&key) {
                    c.known += 1;
                }
            } else {
                // domains.txt style: `example.com www.example.com` or `- www`
                let first = s.split_whitespace().next().unwrap_or("");
                if !first.is_empty()
                    && (first.contains('.')
                        || first.starts_with('-')
                        || first.starts_with('*')
                        || first.contains('@'))
                    && !s.contains(' ')
                    || s.split_whitespace().count() > 1 && first.contains('.')
                {
                    c.domains += 1;
                }
            }
        }
        if c.assignments == 0 && c.domains == 0 {
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
            "# dehydrated\n",
            "CA=https://acme-v02.api.letsencrypt.org/directory\n",
            "CA_TERMS=1\n",
            "CHALLENGETYPE=http-01\n",
            "CONTACT_EMAIL=admin@example.com\n",
            "KEYSIZE=4096\n",
            "KEY_ALGO=rsa\n",
            "RENEW_DAYS=30\n",
            "WELLKNOWN=/var/www/dehydrated\n",
            "HOOK=/etc/dehydrated/hook.sh\n",
            "export ACCOUNTDIR=/etc/dehydrated/accounts\n",
            "BASEDIR=/etc/dehydrated\n",
            "example.com www.example.com\n",
            "another.example.org\n",
        )
        .to_string()
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = Dehydrated::parse(b.as_bytes()).unwrap();
        assert_eq!(c.assignments, 11);
        assert_eq!(c.known, 11);
        assert_eq!(c.domains, 2);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[x]\na=b\n"));
        assert!(Dehydrated::parse(b"# none\n").is_none());
    }
}
