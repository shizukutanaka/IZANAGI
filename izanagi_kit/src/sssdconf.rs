//! SSSD `sssd.conf` パーサ。
//!
//! `[sssd]`/`[nss]`/`[pam]`/`[sudo]`/`[ssh]`/`[pac]`/`[ifp]`/`[domain/<名>]`/
//! `[secrets]`/`[kcm]` セクションと `services`/`domains`/`id_provider`/
//! `auth_provider`/`ldap_uri`/`cache_credentials`/`enum_cache_timeout` 等既知キーを計数する。
//!
//! ```
//! use izanagi_kit::sssdconf;
//! let conf = b"[sssd]\nservices = nss, pam\ndomains = EXAMPLE\n[domain/EXAMPLE]\nid_provider = ldap\nldap_uri = ldap://dc.example.com\n";
//! assert!(sssdconf::detect(conf));
//! let c = sssdconf::parse(conf).unwrap();
//! assert_eq!(c.known_sections, 2);
//! assert_eq!(c.known_keys, 4);
//! ```

/// 判定結果の計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `[...]` セクション数。
    pub sections: usize,
    /// 既知セクション数 (`domain/*` も含む)。
    pub known_sections: usize,
    /// `key = value` 行数。
    pub entries: usize,
    /// 既知キー行数。
    pub known_keys: usize,
}

const KNOWN_SECTIONS: &[&str] = &[
    "sssd",
    "nss",
    "pam",
    "sudo",
    "ssh",
    "pac",
    "ifp",
    "secrets",
    "kcm",
    "responder",
    "monitor",
    "autofs",
];

const KNOWN_KEYS: &[&str] = &[
    "services",
    "domains",
    "config_file_version",
    "reconnection_retries",
    "re_expression",
    "full_name_format",
    "try_inotify",
    "krb5_rcache_dir",
    "user",
    "default_domain_suffix",
    "override_space",
    "id_provider",
    "auth_provider",
    "access_provider",
    "chpass_provider",
    "sudo_provider",
    "autofs_provider",
    "selinux_provider",
    "hostid_provider",
    "subdomains_provider",
    "session_provider",
    "resolver_provider",
    "ldap_uri",
    "ldap_backup_uri",
    "ldap_search_base",
    "ldap_user_search_base",
    "ldap_group_search_base",
    "ldap_netgroup_search_base",
    "ldap_sudo_search_base",
    "ldap_schema",
    "ldap_default_bind_dn",
    "ldap_default_authtok",
    "ldap_default_authtok_type",
    "ldap_tls_reqcert",
    "ldap_tls_cacert",
    "ldap_tls_cacertdir",
    "ldap_tls_cert",
    "ldap_tls_key",
    "ldap_tls_cipher_suite",
    "ldap_id_use_start_tls",
    "ldap_referrals",
    "ldap_sasl_mech",
    "ldap_sasl_authid",
    "ldap_sasl_realm",
    "ldap_krb5_keytab",
    "ldap_krb5_init_creds",
    "ldap_pwd_policy",
    "ldap_purge_cache_timeout",
    "ldap_network_timeout",
    "ldap_opt_timeout",
    "ldap_search_timeout",
    "ldap_page_size",
    "ldap_disable_paging",
    "ldap_deref",
    "ldap_user_object_class",
    "ldap_group_object_class",
    "ldap_user_name",
    "ldap_user_uid_number",
    "ldap_user_gid_number",
    "ldap_user_gecos",
    "ldap_user_home_directory",
    "ldap_user_shell",
    "ldap_user_uuid",
    "ldap_group_name",
    "ldap_group_gid_number",
    "ldap_group_member",
    "ldap_group_objectsid",
    "ldap_user_objectsid",
    "ldap_nested_groups_recv_timeout",
    "ad_server",
    "ad_backup_server",
    "ad_domain",
    "ad_hostname",
    "ad_enabled_sites",
    "ad_access_filter",
    "ad_gpo_access_control",
    "ad_gpo_ignore_unreadable",
    "dyndns_update",
    "dyndns_ttl",
    "dyndns_iface",
    "dyndns_refresh_interval",
    "dyndns_update_ptr",
    "dyndns_force_tcp",
    "dyndns_auth",
    "dyndns_server",
    "ipa_domain",
    "ipa_server",
    "ipa_backup_server",
    "ipa_hostname",
    "ipa_hbac_search_base",
    "ipa_selinux_search_base",
    "ipa_subdomains_search_base",
    "ipa_views_search_base",
    "ipa_enable_dns_sites",
    "ipa_server_mode",
    "ipa_automount_location",
    "krb5_server",
    "krb5_backup_server",
    "krb5_realm",
    "krb5_kpasswd",
    "krb5_backup_kpasswd",
    "krb5_store_password_if_offline",
    "krb5_renewable_lifetime",
    "krb5_lifetime",
    "krb5_renew_interval",
    "krb5_use_fast",
    "krb5_canonicalize",
    "krb5_validate",
    "krb5_keytab",
    "krb5_ccachedir",
    "krb5_ccname_template",
    "krb5_map_user",
    "krb5_auth_timeout",
    "krb5_use_kdcinfo",
    "krb5_use_enterprise_principal",
    "krb5_use_subdomain_tgs",
    "krb5_confd_path",
    "cache_credentials",
    "cache_credentials_min_ssf",
    "account_cache_expiration",
    "entry_cache_timeout",
    "entry_cache_user_timeout",
    "entry_cache_group_timeout",
    "entry_cache_netgroup_timeout",
    "entry_cache_service_timeout",
    "entry_cache_sudo_timeout",
    "entry_cache_autofs_timeout",
    "entry_cache_ssh_host_timeout",
    "entry_cache_computer_timeout",
    "refresh_expired_interval",
    "cached_auth_timeout",
    "offline_timeout",
    "offline_failed_login_attempts",
    "offline_failed_login_delay",
    "enum_cache_timeout",
    "entry_cache_nowait_percentage",
    "entry_negative_timeout",
    "filter_users",
    "filter_groups",
    "filter_users_in_groups",
    "pwfield",
    "override_homedir",
    "fallback_homedir",
    "override_shell",
    "default_shell",
    "shell_fallback",
    "vetoed_shells",
    "homedir_substring",
    "override_gid",
    "min_id",
    "max_id",
    "enumerate",
    "timeout",
    "force_timeout",
    "offline_timeout_max",
    "subdomain_enumerate",
    "force_negative_lookup_cache",
    "ignore_group_members",
    "use_fully_qualified_names",
    "disable_netgroups",
    "enable_files_domain",
    "id_mapping",
    "ldap_id_mapping",
    "ldap_idmap_range_min",
    "ldap_idmap_range_max",
    "ldap_idmap_range_size",
    "ldap_idmap_default_domain_sid",
    "ldap_idmap_default_domain",
    "ldap_idmap_autorid_compat",
    "ldap_idmap_helper_table_size",
    "case_sensitive",
    "pwd_expiration_warning",
    "memcache_timeout",
    "memcache_size_passwd",
    "memcache_size_group",
    "memcache_size_initgroups",
    "user_attributes",
    "negative_cache_timeout",
    "local_negative_timeout",
    "pam_id_timeout",
    "pam_pwd_expiration_warning",
    "pam_verbosity",
    "pam_response_filter",
    "pam_account_expired_message",
    "pam_account_locked_message",
    "pam_cert_auth",
    "pam_cert_db_path",
    "p11_child_timeout",
    "pam_app_services",
    "pam_initgroups_scheme",
    "pam_trusted_uids",
    "pam_public_domains",
    "pam_ssh_auth_timeout",
    "sudo_timed",
    "sudo_inverse_order",
    "sudo_threshold",
    "ssh_hash_known_hosts",
    "ssh_known_hosts_timeout",
    "ca_db",
    "verify",
    "crl_file",
    "pam_11_listen_timeout",
    "provider",
    "timeout",
    "fd_limit",
    "offline_credentials_expiration",
    "subdomain_inherit",
    "user_clockskew",
    "realmd_tags",
];

/// `sssd.conf` らしさを簡易判定する。
pub fn detect(b: &[u8]) -> bool {
    let c = parse(b);
    match c {
        Some(c) => (c.known_sections >= 1 && c.known_keys >= 2) || c.known_keys >= 4,
        None => false,
    }
}

/// セクション名が既知か (`domain/<名>` 形式含む)。
fn known_section(name: &str) -> bool {
    KNOWN_SECTIONS.contains(&name) || name.starts_with("domain/")
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
    };
    for l in s.lines() {
        let t = l.trim();
        if t.is_empty() || t.starts_with('#') || t.starts_with(';') {
            continue;
        }
        if t.starts_with('[') && t.ends_with(']') {
            c.sections += 1;
            let name = t[1..t.len() - 1].trim().to_ascii_lowercase();
            if known_section(&name) {
                c.known_sections += 1;
            }
            continue;
        }
        let Some(eq) = t.find('=') else {
            continue;
        };
        let key = t[..eq].trim().to_ascii_lowercase();
        if key.is_empty() || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
            continue;
        }
        c.entries += 1;
        if KNOWN_KEYS.contains(&key.as_str()) {
            c.known_keys += 1;
        }
    }
    (c.entries > 0 || c.sections > 0).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"[sssd]\nservices = nss, pam, sudo, autofs\ndomains = EXAMPLE\nconfig_file_version = 2\n[nss]\nfilter_users = root\nmemcache_timeout = 300\n[pam]\noffline_credentials_expiration = 0\n[domain/EXAMPLE]\nid_provider = ldap\nauth_provider = ldap\nldap_uri = ldap://dc.example.com\nldap_search_base = dc=example,dc=com\ncache_credentials = True\n";

    #[test]
    fn detects_sssd() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 4);
        assert_eq!(c.known_sections, 4);
        assert_eq!(c.entries, 11);
        assert_eq!(c.known_keys, 11);
    }

    #[test]
    fn rejects_generic_ini() {
        let ini = b"[a]\nx = 1\ny = 2\n[b]\nz = 3\n";
        assert!(!detect(ini));
    }
}
