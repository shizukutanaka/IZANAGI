//! Census of a minica/minica.conf-style PKI configuration.
//!
//! INI-like `key = value` with `[section]` headers, covering CA and issuance
//! knobs: `[ca]`/`[issuer]`/`[dn]`/`[policy]`/`[server]`/`[crl]`/`[ocsp]`/
//! `[extensions]`/`[key]` sections, keys `ca_cert`/`ca_key`/`ca_alg`/`ca_path`/
//! `common_name`/`country`/`organization`/`organizational_unit`/`locality`/
//! `state`/`alt_names`/`key_usage`/`ext_key_usage`/`validity`/`max_path_length`/
//! `pathlen`/`crl_url`/`ocsp_url`/`issuer_url`/`key_size`/`key_alg`/`serial`/
//! `signature_alg`/`profile`/`store`/`database`/`passwd`, `#`/`;` comments,
//! and `=` continuations.
//!
//! ```rust
//! let c = izanagi_kit::minica::Minica::parse(
//!     b"[ca]\nca_cert = ca.pem\nca_key = ca.key\n[dn]\ncommon_name = Test CA\n",
//! ).unwrap();
//! assert_eq!(c.sections, 2);
//! ```
#![forbid(unsafe_code)]

/// Minica configuration census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Minica {
    /// `[section]` headers.
    pub sections: usize,
    /// `key = value` assignments.
    pub assignments: usize,
    /// DN fields (`common_name`/`country`/`organization`/`organizational_unit`/
    /// `locality`/`state`/`email`/`province`).
    pub dn_fields: usize,
    /// URL/path fields (`*_url`/`ca_cert`/`ca_key`/`database`/`store`/`passwd`).
    pub paths: usize,
    /// `#`/`;` comment lines.
    pub comments: usize,
}

/// DN-ish keys.
const DN: &[&str] = &[
    "common_name",
    "country",
    "organization",
    "organizational_unit",
    "locality",
    "state",
    "email",
    "province",
];

/// Path/URL-ish keys.
const PATHS: &[&str] = &[
    "ca_cert",
    "ca_key",
    "crl_url",
    "ocsp_url",
    "issuer_url",
    "database",
    "store",
    "passwd",
    "ca_path",
];

/// True if `b` looks like a minica conf.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("ca_cert") || t.contains("common_name") || t.contains("ext_key_usage"))
        && t.contains('=')
}

impl Minica {
    /// Parse a minica conf into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            assignments: 0,
            dn_fields: 0,
            paths: 0,
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
            } else if let Some(eq) = l.find('=') {
                c.assignments += 1;
                let key = l[..eq].trim();
                if DN.iter().any(|d| key.eq_ignore_ascii_case(d)) {
                    c.dn_fields += 1;
                }
                if PATHS.iter().any(|p| key.eq_ignore_ascii_case(p)) {
                    c.paths += 1;
                }
            }
        }
        if c.assignments == 0 && c.sections == 0 {
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
            "# minica\n",
            "[ca]\n",
            "ca_cert = ca.pem\n",
            "ca_key = ca.key\n",
            "ca_alg = ec\n",
            "[dn]\n",
            "common_name = Test CA\n",
            "country = JP\n",
            "organization = Example\n",
            "[extensions]\n",
            "key_usage = cRLSign, digitalSignature\n",
            "ext_key_usage = serverAuth, clientAuth\n",
            "[crl]\n",
            "crl_url = http://ca.example.com/ca.crl\n",
            "[ocsp]\n",
            "ocsp_url = http://ocsp.example.com\n",
        )
        .to_string()
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = Minica::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 5);
        assert_eq!(c.dn_fields, 3);
        assert_eq!(c.paths, 4);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[main]\nfoo = 1\n"));
        assert!(Minica::parse(b"# none\n").is_none());
    }
}
