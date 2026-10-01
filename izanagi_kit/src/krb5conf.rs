//! MIT Kerberos `krb5.conf` パーサ。
//!
//! `[libdefaults]`/`[realms]`/`[domain_realm]`/`[kdc]`/`[logging]`/`[appdefaults]`/
//! `[dbmodules]`/`[capaths]` セクションと `default_realm`/`kdc`/`admin_server`/
//! `default_ccache_name`/`dns_lookup_kdc` 等既知キーを計数する。
//!
//! ```
//! use izanagi_kit::krb5conf;
//! let conf = b"[libdefaults]\ndefault_realm = EXAMPLE.COM\ndns_lookup_kdc = true\n[realms]\nEXAMPLE.COM = {\n  kdc = kdc.example.com\n}\n";
//! assert!(krb5conf::detect(conf));
//! let c = krb5conf::parse(conf).unwrap();
//! assert_eq!(c.known_sections, 2);
//! assert_eq!(c.known_keys, 3);
//! ```

/// 判定結果の計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `[...]` セクション数。
    pub sections: usize,
    /// 既知セクション数。
    pub known_sections: usize,
    /// `key = value` 行数 (`realm = {` 含む)。
    pub entries: usize,
    /// 既知キー行数。
    pub known_keys: usize,
    /// ネストされた `realm = {` ブロック数。
    pub realm_blocks: usize,
}

const KNOWN_SECTIONS: &[&str] = &[
    "libdefaults",
    "realms",
    "domain_realm",
    "kdc",
    "kdcdefaults",
    "logging",
    "appdefaults",
    "dbmodules",
    "capaths",
    "otp",
    "plugins",
    "dbdefaults",
    "pkinit_anchors",
];

const KNOWN_KEYS: &[&str] = &[
    "default_realm",
    "default_ccache_name",
    "default_keytab_name",
    "default_client_keytab_name",
    "default_tkt_enctypes",
    "default_tgs_enctypes",
    "permitted_enctypes",
    "allow_weak_crypto",
    "kdc_timesync",
    "kdc_req_checksum_type",
    "ap_req_checksum_type",
    "safe_checksum_type",
    "clockskew",
    "dns_lookup_kdc",
    "dns_lookup_realm",
    "dns_canonicalize_hostname",
    "rdns",
    "udp_preference_limit",
    "max_retries",
    "renew_lifetime",
    "ticket_lifetime",
    "forwardable",
    "proxiable",
    "noaddresses",
    "extra_addresses",
    "verify_ap_req_nofail",
    "kdc",
    "admin_server",
    "default_domain",
    "master_kdc",
    "kpasswd_server",
    "database_module",
    "auth_to_local",
    "auth_to_local_names",
    "v4_instance_convert",
    "v4_name_convert",
    "krb4_server",
    "krb4_config",
    "krb4_convert",
    "krb524d",
    "v4_realm",
    "krb524_server",
    "v4_instance_resolve",
    "des_crc_session_supported",
    "allow_enc_tts_in_skey",
    "check_pac",
    "max_life",
    "max_renewable_life",
    "acl_file",
    "dict_file",
    "default_principal_expiration",
    "default_principal_flags",
    "supported_enctypes",
    "reject_bad_transit",
    "kadmind_port",
    "kpasswd_port",
    "iprop_enable",
    "iprop_port",
    "key_stash_file",
    "host_based_services",
    "restrict_anonymous_to_tgt",
    "cert_stash_file",
];

/// `krb5.conf` らしさを簡易判定する。
pub fn detect(b: &[u8]) -> bool {
    let c = parse(b);
    match c {
        Some(c) => (c.known_sections >= 1 && c.known_keys >= 2) || c.known_keys >= 3,
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
        sections: 0,
        known_sections: 0,
        entries: 0,
        known_keys: 0,
        realm_blocks: 0,
    };
    for l in s.lines() {
        let t = l.trim();
        if t.is_empty() || t.starts_with('#') || t.starts_with(';') || t == "}" {
            continue;
        }
        if t.starts_with('[') && t.ends_with(']') {
            c.sections += 1;
            let name = t[1..t.len() - 1].trim().to_ascii_lowercase();
            if KNOWN_SECTIONS.contains(&name.as_str()) {
                c.known_sections += 1;
            }
            continue;
        }
        let Some(eq) = t.find('=') else {
            continue;
        };
        let key = t[..eq].trim().to_ascii_lowercase();
        if key.is_empty()
            || !key
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'-'))
        {
            continue;
        }
        c.entries += 1;
        if KNOWN_KEYS.contains(&key.as_str()) {
            c.known_keys += 1;
        }
        if t.ends_with('{') {
            c.realm_blocks += 1;
        }
    }
    (c.entries > 0 || c.sections > 0).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"[libdefaults]\ndefault_realm = EXAMPLE.COM\ndns_lookup_kdc = true\nticket_lifetime = 24h\n[realms]\nEXAMPLE.COM = {\n  kdc = kdc.example.com\n  admin_server = kdc.example.com\n}\n[domain_realm]\n.example.com = EXAMPLE.COM\n";

    #[test]
    fn detects_krb5() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.known_sections, 3);
        assert_eq!(c.entries, 7);
        assert_eq!(c.known_keys, 5);
        assert_eq!(c.realm_blocks, 1);
    }

    #[test]
    fn rejects_generic_ini() {
        let ini = b"[a]\nx = 1\ny = 2\n[b]\nz = 3\n";
        assert!(!detect(ini));
    }
}
