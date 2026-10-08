//! OpenSSL `openssl.cnf` configuration format.
//!
//! openssl.cnf is INI-ish with `[ section ]` headers, `key = value`
//! settings, `${VAR}` / `$ENV::VAR` references, `.include` directives,
//! `oid_section`/`new_oids` OID registration, and
//! `distinguished_name`/`req_extensions`/`x509_extensions` link keys.
//!
//! ```
//! let b = concat!(
//!     "[ req ]\n",
//!     "default_bits = 2048\n",
//!     "distinguished_name = dn\n",
//!     "x509_extensions = v3_ca\n",
//!     "[ dn ]\n",
//!     "CN = example.com\n",
//!     "[ v3_ca ]\n",
//!     "basicConstraints = critical,CA:true\n",
//!     "subjectKeyIdentifier = hash\n",
//!     "# tail\n"
//! ).as_bytes();
//! assert!(izanagi_kit::openssl::detect(b));
//! let c = izanagi_kit::openssl::Openssl::parse(b).unwrap();
//! assert_eq!(c.sections, 3);
//! ```

/// Parsed openssl.cnf summary.
#[derive(Debug, Clone)]
pub struct Openssl {
    /// `[ section ]` headers.
    pub sections: usize,
    /// `key = value` settings.
    pub entries: usize,
    /// `${VAR}`/`$ENV::VAR`/`$section::key` references.
    pub references: usize,
    /// `.include` directives.
    pub includes: usize,
    /// `oid_section`/`new_oids`/`oid_file` entries.
    pub oids: usize,
    /// Link keys (`distinguished_name`, `x509_extensions`, `req_extensions`, `extensions`, `certificate`, `private_key`, `serial`, `database`, `new_certs_dir`, `default_ca`, `crl_dir`, `policy`, `unique_subject`, `copy_extensions`, `default_days`, `default_md`, `preserve`, `email_in_dn`, `nameopt`, `certopt`, `rand_serial`, `authorityInfoAccess`, `crlDistributionPoints`, `subjectKeyIdentifier`, `keyUsage`, `extendedKeyUsage`, `basicConstraints`, `nsComment`, `nsCertType`, `nsBaseUrl`, `nsRevocationUrl`, `nsRenewalUrl`, `nsCaRevocationUrl`, `nsCaPolicyUrl`, `nsSslServerName`, `issuerAltName`, `subjectAltName`, `authorityKeyIdentifier`, `crlNumber`, `crl_extensions`, `crlExtension`, `policy_*,` `policy_*` `crlDistributionPoints`, `policy_print`, `message_digest`, `name_constraints`, `no_check`, `OCSP`/`ocsp`/`openssl`, `msie_hack`, `nsCaPolicyUrl` openssl-specific entries.
    pub links: usize,
    /// `#`/`;` comment lines.
    pub comments: usize,
}

/// OpenSSL-specific vocabulary — `[x]` sections and `k = v` lines are
/// shared with every ini-style config and prove nothing on their own.
const VOCAB: &[&str] = &[
    "alt_names",
    "authoritykeyidentifier",
    "basicconstraints",
    "ca_default",
    "distinguished_name",
    "default_bits",
    "default_md",
    "keyusage",
    "ns_comment",
    "oid_section",
    "randfile",
    "req_distinguished_name",
    "subjectaltname",
    "subjectkeyidentifier",
    "uniquesubject",
    "v3_ca",
    "v3_req",
    "x509",
];

fn vocab_hit(tr: &str) -> bool {
    let low = tr.to_ascii_lowercase();
    if VOCAB.iter().any(|v| low.contains(v)) {
        return true;
    }
    // `oid`-prefixed word (`oid_section`, `oid_file`, `oid_table`…);
    // `android`/`void` do not qualify
    low.split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .any(|w| w.starts_with("oid"))
}

/// Whether the buffer looks like openssl.cnf.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut sections = 0usize;
    let mut score = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') || tr.starts_with(';') {
            continue;
        }
        if tr.starts_with('[') && tr.ends_with(']') {
            sections += 1;
            score += 1;
        } else if tr.contains('=')
            && (tr.contains("oid") || tr.contains("section") || tr.contains('_'))
        {
            score += 1;
        }
    }
    let mut vocab = 0usize;
    if score >= 3 && sections >= 1 {
        vocab += t.lines().map(str::trim).filter(|l| vocab_hit(l)).count();
    }
    score >= 3 && sections >= 1 && vocab >= 1
}

impl Openssl {
    /// Parses an openssl.cnf summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            entries: 0,
            references: 0,
            includes: 0,
            oids: 0,
            links: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with('#') || tr.starts_with(';') {
                c.comments += 1;
                continue;
            }
            if tr.starts_with(".include") {
                c.includes += 1;
                continue;
            }
            if tr.starts_with('[') && tr.ends_with(']') {
                c.sections += 1;
                continue;
            }
            let Some((k, v)) = tr.split_once('=') else {
                continue;
            };
            c.entries += 1;
            let v = v.trim();
            if v.contains("${") || v.contains("$ENV::") || v.contains('$') {
                c.references += 1;
            }
            let k = k.trim();
            if k.contains("oid") || k.contains("new_oids") || k.contains("oid_file") {
                c.oids += 1;
            }
            const LINK_KEYS: &[&str] = &[
                "distinguished_name",
                "x509_extensions",
                "req_extensions",
                "extensions",
                "certificate",
                "private_key",
                "serial",
                "database",
                "new_certs_dir",
                "default_ca",
                "crl_dir",
                "policy",
                "unique_subject",
                "copy_extensions",
                "default_days",
                "default_md",
                "preserve",
                "email_in_dn",
                "nameopt",
                "certopt",
                "rand_serial",
            ];
            if LINK_KEYS.contains(&k) || k.contains("crl_extensions") {
                c.links += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_openssl() {
        let b = concat!(
            "[ req ]\n",
            "default_bits = 2048\n",
            "distinguished_name = dn\n",
            "x509_extensions = v3_ca\n",
            ".include /etc/ssl/extra.cnf\n",
            "[ dn ]\n",
            "CN = example.com\n",
            "O = ${ORG}\n",
            "[ v3_ca ]\n",
            "basicConstraints = critical,CA:true\n",
            "subjectKeyIdentifier = hash\n",
            "# tail\n"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Openssl::parse(b).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.includes, 1);
        assert_eq!(c.references, 1);
        assert_eq!(c.links, 2);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"a = b\nc = d\n"));
        assert!(Openssl::parse(b"x").is_none());
    }
}
