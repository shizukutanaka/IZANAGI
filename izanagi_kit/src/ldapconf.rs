//! OpenLDAP `ldap.conf` / `.ldaprc` census.
//!
//! Whitespace-separated `KEY value…` directives:
//! `URI`/`BASE`/`BINDDN`/`HOST`/`PORT`/`REFERRALS`/`SIZELIMIT`/`TIMELIMIT`/
//! `DEREF`/`SCOPE`/`TIMEOUT`/`NETWORK_TIMEOUT`/`SASL_*`/`TLS_*`/`GSSAPI_*`/
//! `KRB5_*`/`BIND_POLICY`/`WHEN_*`/`VERSION`.
//!
//! ```rust
//! let l = "URI ldap://ds.internal\nBASE dc=corp,dc=example,dc=com\nTLS_REQCERT demand\n";
//! let c = izanagi_kit::ldapconf::Ldapconf::parse(l.as_bytes()).unwrap();
//! assert_eq!(c.settings, 3);
//! ```

/// ldap.conf census.
#[derive(Debug, Clone)]
pub struct Ldapconf {
    /// `KEY value` directives.
    pub settings: usize,
    /// `TLS_*`/`SASL_*`/`GSSAPI_*`/`KRB5_*`/`WHEN_*` namespaced keys.
    pub namespaced: usize,
    /// Recognised OpenLDAP option names.
    pub named: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "URI",
    "BASE",
    "BINDDN",
    "HOST",
    "PORT",
    "REFERRALS",
    "SIZELIMIT",
    "TIMELIMIT",
    "DEREF",
    "SCOPE",
    "TIMEOUT",
    "NETWORK_TIMEOUT",
    "BIND_POLICY",
    "VERSION",
    "SASL_MECH",
    "SASL_REALM",
    "SASL_AUTHCID",
    "SASL_AUTHZID",
    "SASL_SECPROPS",
    "SASL_NOCANON",
    "SASL_CBINDING",
    "SASL_MAXSSF",
    "SASL_MINSSF",
    "GSSAPI_SIGN",
    "GSSAPI_ENCRYPT",
    "GSSAPI_ALLOW_REMOTE_PRINCIPAL",
    "TLS_CACERT",
    "TLS_CACERTDIR",
    "TLS_CERT",
    "TLS_KEY",
    "TLS_CIPHER_SUITE",
    "TLS_PROTOCOL_MIN",
    "TLS_PROTOCOL_MAX",
    "TLS_RANDFILE",
    "TLS_REQCERT",
    "TLS_REQSAN",
    "TLS_CRLCHECK",
    "TLS_CRLFILE",
    "TLS_DHFILE",
    "TLS_ECNAME",
    "TLS_SNI",
    "KRB5_KTNAME",
    "WHEN_FIRST",
    "WHEN_LAST",
    "WHEN_ALWAYS",
    "WHEN_NEVER",
    "WHEN_SEARCH",
    "WHEN_FIND",
    "WHEN_ADD",
    "WHEN_DELETE",
    "WHEN_MODRDN",
    "WHEN_MODIFY",
    "WHEN_READ",
    "WHEN_WRITE",
    "BASEDN",
    "KEEPASYNC",
    "SOCKET_KEEPALIVE",
    "USERATTR",
    "ATTROLSONEXACT",
    "ATTRORDERING",
    "DEREF_ALIASES",
    "NO_WILDCARD",
    "SYNTAX",
    "SORTREQUEST",
    "SORTCONSIDER",
    "LIBLDAP",
    "ALIAS",
    "BIND_TIMELIMIT",
    "FIND_BASE",
    "PIDFILE",
    "ARGSFILE",
    "DEREF_NEVER",
    "DEREF_SEARCHING",
    "DEREF_FINDING",
    "DEREF_ALWAYS",
];

/// Whether the buffer looks like ldap.conf.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("TLS_REQCERT")
        || t.contains("SASL_MECH")
        || t.contains("BINDDN")
        || t.contains("dc=") && (t.contains("BASE") || t.contains("URI"))
        || t.contains("ldap://")
        || t.contains("ldaps://")
        || t.contains("ldapi://")
}

impl Ldapconf {
    /// Parse an ldap.conf into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            settings: 0,
            namespaced: 0,
            named: 0,
            comments: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            let mut it = s.split_whitespace();
            let Some(key) = it.next() else {
                continue;
            };
            if it.next().is_none() {
                continue;
            }
            if key.chars().all(|ch| ch.is_ascii_uppercase() || ch == '_') {
                c.settings += 1;
                if key.contains('_') {
                    c.namespaced += 1;
                }
                if KEYS.contains(&key) {
                    c.named += 1;
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
    fn parses_conf() {
        let b = concat!(
            "# ldap.conf\n",
            "URI ldap://ds1.internal ldaps://ds2.internal\n",
            "BASE dc=corp,dc=example,dc=com\n",
            "BINDDN cn=proxy,ou=service,dc=corp,dc=example,dc=com\n",
            "SIZELIMIT 500\n",
            "TIMELIMIT 30\n",
            "TLS_REQCERT demand\n",
            "TLS_CACERT /etc/ssl/ca.pem\n",
            "SASL_MECH GSSAPI\n",
            "REFERRALS off\n",
        );
        let c = Ldapconf::parse(b.as_bytes()).unwrap();
        assert_eq!(c.settings, 9);
        assert_eq!(c.comments, 1);
        assert_eq!(c.namespaced, 3);
        assert_eq!(c.named, 9);
    }

    #[test]
    fn rejects_other() {
        assert!(Ldapconf::parse(b"foo bar").is_none());
    }
}
