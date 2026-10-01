//! nslcd `nslcd.conf` パーサ (nss-pam-ldapd)。
//!
//! 平坦な `key value` 形式と `uid`/`gid`/`uri`/`base`/`binddn`/`tls_cacertfile`/
//! `map`/`filter`/`scope`/`pagesize`/`sasl_*`/`krb5_*` 等既知キーを計数する。
//!
//! ```
//! use izanagi_kit::nslcdconf;
//! let conf = b"uid nslcd\ngid nslcd\nuri ldap://localhost/\nbase dc=example,dc=com\n";
//! assert!(nslcdconf::detect(conf));
//! let c = nslcdconf::parse(conf).unwrap();
//! assert_eq!(c.entries, 4);
//! assert_eq!(c.known_keys, 4);
//! ```

/// 判定結果の計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `key value` 行数。
    pub entries: usize,
    /// 既知キー行数。
    pub known_keys: usize,
}

const KNOWN_KEYS: &[&str] = &[
    "uid",
    "gid",
    "log",
    "uri",
    "ldap_version",
    "binddn",
    "bindpw",
    "rootpwmoddn",
    "rootpwmodpw",
    "sasl_mech",
    "sasl_realm",
    "sasl_authcid",
    "sasl_authzid",
    "sasl_secprops",
    "sasl_canonicalize",
    "krb5_ccname",
    "base",
    "scope",
    "deref",
    "referrals",
    "maps",
    "map",
    "filter",
    "filters",
    "bind_timelimit",
    "timelimit",
    "idle_timelimit",
    "reconnect_sleeptime",
    "reconnect_retrytime",
    "ssl",
    "tls_reqcert",
    "tls_cacertdir",
    "tls_cacertfile",
    "tls_randfile",
    "tls_ciphers",
    "tls_cert",
    "tls_key",
    "pagesize",
    "nss_initgroups_ignoreusers",
    "nss_min_uid",
    "nss_max_uid",
    "nss_nested_groups",
    "nss_getgrent_skipmembers",
    "nss_disable_enumeration",
    "validnames",
    "ignorecase",
    "pam_authz_search",
    "pam_authc_search",
    "pam_authc_ppolicy",
    "pam_password_prohibit_message",
    "pam_password_expiry_warning",
    "pam_min_uid",
    "pam_max_uid",
    "cache",
    "reconnect_invalidate",
    "reconnect_maxsleeptime",
];

/// `nslcd.conf` らしさを簡易判定する。
pub fn detect(b: &[u8]) -> bool {
    let c = parse(b);
    match c {
        Some(c) => c.entries >= 3 && c.known_keys >= 3,
        None => false,
    }
}

/// 行種別を計数する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    if !b.is_ascii() {
        return None;
    }
    let s = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        entries: 0,
        known_keys: 0,
    };
    for l in s.lines() {
        let t = l.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let key = t.split_whitespace().next().unwrap_or("");
        if key.is_empty() || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
            continue;
        }
        c.entries += 1;
        if KNOWN_KEYS.contains(&key) {
            c.known_keys += 1;
        }
    }
    (c.entries > 0).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"uid nslcd\ngid nslcd\nuri ldap://localhost/\nbase dc=example,dc=com\nbinddn cn=admin,dc=example,dc=com\nbindpw secret\nssl start_tls\ntls_reqcert demand\n";

    #[test]
    fn detects_nslcd() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.entries, 8);
        assert_eq!(c.known_keys, 8);
    }

    #[test]
    fn rejects_other() {
        let t = b"foo bar\nbaz qux\nhello world\n";
        assert!(!detect(t));
    }
}
