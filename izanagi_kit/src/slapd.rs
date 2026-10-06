//! OpenLDAP `slapd.conf` (and `slapd.d`-style `olc*` attribute) census.
//!
//! Whitespace `directive value` lines: global (`include`, `moduleload`,
//! `loglevel`, `TLSCertificateFile` …), database (`database`, `suffix`,
//! `rootdn`, `rootpw`, `directory`, `index`, `access`, `overlay`,
//! `syncrepl` …) and `olc*` attributes for config-database dumps.
//! `#` comments. Companion to `crate::ldapconf` (client `ldap.conf`).
//!
//! ```rust
//! let s = b"include /etc/ldap/schema/core.schema\nloglevel 256\ndatabase mdb\nsuffix dc=example,dc=com\nrootdn cn=admin,dc=example,dc=com\ndirectory /var/lib/ldap\n";
//! assert!(izanagi_kit::slapd::detect(s));
//! let c = izanagi_kit::slapd::Slapd::parse(s).unwrap();
//! assert_eq!(c.settings, 6);
//! ```

/// slapd.conf census.
#[derive(Debug, Clone)]
pub struct Slapd {
    /// `directive value` lines matching a known slapd directive.
    pub settings: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// slapd directives (first word; covers `slapd.conf` and `olc*` config
/// attributes).
const KEYS: &[&str] = &[
    "access",
    "allow",
    "argsfile",
    "attribute",
    "attributetype",
    "authz-regexp",
    "authz-policy",
    "backend",
    "bindconf",
    "cachesize",
    "checkpoint",
    "concurrency",
    "conn_max_pending",
    "conn_max_pending_auth",
    "database",
    "defaultsearchbase",
    "deref",
    "directory",
    "dirtyread",
    "disallow",
    "dn",
    "dncachesize",
    "gentlehup",
    "idletimeout",
    "idlcachesize",
    "include",
    "index",
    "index_substr_*",
    "lastmod",
    "limits",
    "listener-threads",
    "localSSF",
    "loglevel",
    "maxderefdepth",
    "maxsize",
    "mode",
    "moduleload",
    "modulepath",
    "monitoring",
    "objectclass",
    "olcAccess",
    "olcAddContentAcl",
    "olcAllows",
    "olcArgsFile",
    "olcAttributeOptions",
    "olcAttributeTypes",
    "olcAuthIDRewrite",
    "olcAuthzPolicy",
    "olcAuthzRegexp",
    "olcBackend",
    "olcConcurrency",
    "olcConnMaxPending",
    "olcConnMaxPendingAuth",
    "olcDatabase",
    "olcDbCacheSize",
    "olcDbCheckpoint",
    "olcDbConfig",
    "olcDbDirectory",
    "olcDbDNcacheSize",
    "olcDbIDLcacheSize",
    "olcDbIndex",
    "olcDbMaxSize",
    "olcDbMode",
    "olcDbNoSync",
    "olcDbSearchStack",
    "olcDbShmKey",
    "olcDbURI",
    "olcDisallows",
    "olcGentleHUP",
    "olcIdleTimeout",
    "olcInclude",
    "olcLastMod",
    "olcLimits",
    "olcLocalSSF",
    "olcLogFile",
    "olcLogLevel",
    "olcModuleLoad",
    "olcModulePath",
    "olcObjectClasses",
    "olcOverlay",
    "olcPasswordCryptSaltFormat",
    "olcPasswordHash",
    "olcPidFile",
    "olcPluginLogFile",
    "olcReadOnly",
    "olcReferral",
    "olcReplicaArgsFile",
    "olcReplicaPidFile",
    "olcReplicationInterval",
    "olcRequires",
    "olcRestrict",
    "olcReverseLookup",
    "olcRootDN",
    "olcRootDSE",
    "olcRootPW",
    "olcSaslHost",
    "olcSaslRealm",
    "olcSaslSecProps",
    "olcSchemaDN",
    "olcSecurity",
    "olcServerID",
    "olcSizeLimit",
    "olcSockbufMaxIncoming",
    "olcSockbufMaxIncomingAuth",
    "olcSortVals",
    "olcSubordinate",
    "olcSuffix",
    "olcSyncProviderConfig",
    "olcSyncrepl",
    "olcTCPBuffer",
    "olcThreads",
    "olcTimeLimit",
    "olcTLSCACertificateFile",
    "olcTLSCACertificatePath",
    "olcTLSCertificateFile",
    "olcTLSCertificateKeyFile",
    "olcTLSCipherSuite",
    "olcTLSDHParamFile",
    "olcTLSProtocolMin",
    "olcTLSRandFile",
    "olcTLSVerifyClient",
    "olcToolThreads",
    "olcUpdateRef",
    "overlay",
    "password-crypt-salt-format",
    "password-hash",
    "pidfile",
    "plugin",
    "readonly",
    "referral",
    "replica",
    "replogfile",
    "require",
    "restrict",
    "reverse-lookup",
    "reversedn",
    "rootdn",
    "rootdse",
    "rootpw",
    "sasl-host",
    "sasl-realm",
    "sasl-regexp",
    "sasl-secprops",
    "schemacheck",
    "security",
    "serverid",
    "sizelimit",
    "sockbuf_max_incoming",
    "sockbuf_max_incoming_auth",
    "sortvals",
    "subordinate",
    "suffix",
    "suffixmassage",
    "syncdata",
    "syncrepl",
    "threads",
    "timelimit",
    "TLSCACertificateFile",
    "TLSCACertificatePath",
    "TLSCertificateFile",
    "TLSCertificateKeyFile",
    "TLSCipherSuite",
    "TLSDHParamFile",
    "TLSProtocolMin",
    "TLSRandFile",
    "TLSVerifyClient",
    "tool-threads",
    "ucdata-path",
    "updatedn",
    "updateref",
];

fn first_word(t: &str) -> &str {
    t.split(char::is_whitespace)
        .next()
        .unwrap_or("")
        .trim_end_matches(':')
}

/// Detect a `slapd.conf`-style file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut n = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') {
            continue;
        }
        if KEYS.contains(&first_word(tr)) {
            n += 1;
        }
    }
    n >= 3
}

impl Slapd {
    /// Count directives. Returns `None` when the input does not look like
    /// a `slapd.conf`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            settings: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if KEYS.contains(&first_word(tr)) {
                c.settings += 1;
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
        let b = b"# slapd\ninclude /etc/ldap/schema/core.schema\ninclude /etc/ldap/schema/cosine.schema\npidfile /var/run/slapd.pid\nargsfile /var/run/slapd.args\nloglevel 256\nmoduleload back_mdb\nTLSCertificateFile /etc/ldap/cert.pem\ndatabase mdb\nmaxsize 1073741824\nsuffix dc=example,dc=com\nrootdn cn=admin,dc=example,dc=com\nrootpw {SSHA}x\ndirectory /var/lib/ldap\nindex objectClass eq\nindex cn,uid eq\naccess to attrs=userPassword by self write by anonymous auth\n";
        assert!(detect(b));
        let c = Slapd::parse(b).unwrap();
        assert_eq!(c.settings, 16);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"URI ldap://x\nBASE dc=a\nTLS_REQCERT demand\n"));
        assert!(!detect(b"foo bar\n"));
        assert!(Slapd::parse(b"").is_none());
    }
}
