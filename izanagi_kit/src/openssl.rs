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

/// Whether the buffer looks like openssl.cnf.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut score = 0usize;
    let mut strong = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') || tr.starts_with(';') {
            continue;
        }
        if tr.starts_with('[') && tr.ends_with(']') {
            score += 1;
            if is_openssl_section(&tr[1..tr.len() - 1]) {
                strong += 1;
            }
        } else if tr.contains('=')
            && (tr.contains("oid") || tr.contains("section") || tr.contains('_'))
        {
            score += 1;
            if is_openssl_key(tr) {
                strong += 1;
            }
        }
    }
    // `[sec]` + `key_name = v` describes most INI files; an openssl.cnf
    // also carries at least one OpenSSL-flavoured section or key.
    score >= 3 && strong >= 1
}

/// Section names characteristic of openssl.cnf (`[ req ]`, `[ v3_ca ]`,
/// `[ dn ]`, `[ oid_section ]`, …).
fn is_openssl_section(inner: &str) -> bool {
    let s = inner.trim();
    s == "req"
        || s == "dn"
        || s == "ca"
        || s == "tsa"
        || s == "crl"
        || s == "v3_ca"
        || s == "usr_cert"
        || s == "oid_section"
        || s == "new_oids"
        || s == "distinguished_name"
        || s == "req_distinguished_name"
        || s.contains("v3")
        || s.contains("x509")
        || s.contains("_ca")
        || s.ends_with("_ext")
        || s.contains("policy")
        || s.contains("alt_names")
}

/// Keys/values characteristic of openssl.cnf content lines.
fn is_openssl_key(line: &str) -> bool {
    let l = line.to_ascii_lowercase();
    l.contains("oid")
        || l.contains("distinguished_name")
        || l.contains("basicconstraints")
        || l.contains("keyusage")
        || l.contains("subjectaltname")
        || l.contains("authoritykey")
        || l.contains("x509")
        || l.contains("nscomment")
        || l.contains("default_bits")
        || l.contains("default_md")
        || l.contains("default_days")
        || l.contains("unique_subject")
        || l.contains("new_certs")
        || l.contains("copy_extensions")
        || l.contains("randfile")
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
