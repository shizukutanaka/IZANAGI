//! Squid `squid.conf` configuration census.
//!
//! A squid.conf is `key args` lines: `acl <name> <type> <value>` ACLs,
//! `http_access`/`icp_access`/`http_port`/`https_port`/`icp_port`/
//! `snmp_port`, `cache_mem`/`cache_dir`/`cache_log`/`access_log`/
//! `refresh_pattern`, `coredump_dir`/`pid_filename`/`visible_hostname`,
//! `hierarchy_stoplist`, `acl ... !` negations and `allow`/`deny` verbs.
//! `parse` counts each directive class.
//!
//! ```rust
//! let s = concat!(
//!     "acl localnet src 10.0.0.0/8\n",
//!     "acl SSL_ports port 443\n",
//!     "http_access allow localnet\n",
//!     "http_access deny all\n",
//!     "http_port 3128\n",
//!     "cache_mem 256 MB\n",
//!     "cache_dir ufs /var/spool/squid 100 16 256\n",
//!     "refresh_pattern ^ftp: 1440 20% 10080\n",
//! );
//! let c = izanagi_kit::squid::Squid::parse(s.as_bytes()).unwrap();
//! assert_eq!(c.acls, 2);
//! assert_eq!(c.accesses, 2);
//! ```

const ACCESS: &[&str] = &[
    "http_access",
    "icp_access",
    "htcp_access",
    "snmp_access",
    "url_rewrite_access",
    "adaptation_access",
    "miss_access",
    "ident_lookup_access",
    "cachemgr_passwd",
];

const PORTS: &[&str] = &[
    "http_port",
    "https_port",
    "icp_port",
    "htcp_port",
    "snmp_port",
    "ftp_port",
    "ssl_bump",
];

const CACHEK: &[&str] = &[
    "cache_mem",
    "cache_dir",
    "cache_log",
    "cache_swap_log",
    "cache_effective_user",
    "cache_effective_group",
    "cache_host",
    "cache_peer",
    "cache_peer_access",
    "cachemgr_passwd",
    "cachemgr",
    "cache_store_log",
    "cache_replacement_policy",
    "cache_swap_high",
    "cache_swap_low",
    "cache_vary",
    "collapsed_forwarding",
    "memory_pools",
    "maximum_object_size",
    "minimum_object_size",
    "offline_mode",
    "prefer_direct",
    "always_direct",
    "never_direct",
    "nonhierarchical_direct",
    "digest_generation",
    "digest_bits_per_entry",
    "digest_swapout_chunk_size",
    "logfile_rotate",
    "debug_options",
    "pid_filename",
    "coredump_dir",
    "visible_hostname",
    "unique_hostname",
    "hostname_aliases",
    "error_log_languages",
    "err_page_stylesheet",
    "dns_nameservers",
    "hosts_file",
    "append_domain",
    "connect_timeout",
    "request_timeout",
    "persistent_request_timeout",
    "read_timeout",
    "write_timeout",
    "client_lifetime",
    "half_closed_clients",
    "shutdown_lifetime",
    "negative_ttl",
    "positive_dns_ttl",
    "negative_dns_ttl",
    "range_offset_limit",
    "via",
    "forwarded_for",
    "follow_x_forwarded_for",
    "request_header_access",
    "reply_header_access",
    "header_access",
    "header_replace",
    "uri_whitespace",
    "broken_vary_encoding",
    "uri_whitespace",
    "access_log",
    "cache_log",
    "mime_table",
    "log_mime_hdrs",
    "logfile_daemon",
    "client_netmask",
    "strip_query_terms",
    "buffered_logs",
    "netdb_filename",
    "netdb_high",
    "netdb_low",
    "netdb_ping_period",
    "netdb_ping_rate",
    "query_icmp",
    "minimum_direct_hops",
    "test_reachability",
    "reload_into_ims",
    "pconn_timeout",
    "as_whois_server",
    "log_fqdn",
    "client_db",
    "global_internal_static",
    "memory_cache_shared",
    "memory_cache_mode",
    "warn_default_reject",
];

/// Squid configuration census.
#[derive(Debug, Clone)]
pub struct Squid {
    /// `acl <name> <type> <args>` definitions.
    pub acls: usize,
    /// `*_access allow|deny` rules.
    pub accesses: usize,
    /// `allow`/`deny` verdicts inside access rules.
    pub verdicts: usize,
    /// `http_port`/`https_port`/`icp_port`/`snmp_port`/`ssl_bump` lines.
    pub ports: usize,
    /// `cache_*`/`*_log`/memory/size/`coredump_dir`/`pid_filename`/`visible_hostname`/`dns_*` directives.
    pub caches: usize,
    /// `refresh_pattern`/`refresh_all_ims` lines.
    pub refreshes: usize,
    /// `hierarchy_stoplist`/`request_header_*`/`extension_methods`/`sslproxy_*`/`icap_*`/`adaptation_*`/`ecap_*` lines.
    pub extras: usize,
    /// Other `key args` directive lines.
    pub directives: usize,
    /// `!` negations inside access rules.
    pub negations: usize,
}

/// Whether the buffer looks like a squid.conf.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("acl ") && (t.contains("http_access") || t.contains("http_port")))
        || t.contains("refresh_pattern")
        || (t.contains("cache_dir") && t.contains("http_"))
}

impl Squid {
    /// Parse a squid.conf into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            acls: 0,
            accesses: 0,
            verdicts: 0,
            ports: 0,
            caches: 0,
            refreshes: 0,
            extras: 0,
            directives: 0,
            negations: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() || s.starts_with('#') {
                continue;
            }
            let head = s.split_whitespace().next().unwrap_or("");
            if head == "acl" {
                c.acls += 1;
                continue;
            }
            if ACCESS.contains(&head) {
                c.accesses += 1;
                for part in s.split_whitespace().skip(1) {
                    if part == "allow" || part == "deny" {
                        c.verdicts += 1;
                    } else if part.starts_with('!') {
                        c.negations += 1;
                    }
                }
                continue;
            }
            if PORTS.contains(&head) {
                c.ports += 1;
                continue;
            }
            if s.starts_with("refresh_pattern") || s.starts_with("refresh_all_ims") {
                c.refreshes += 1;
                continue;
            }
            if CACHEK.contains(&head) {
                c.caches += 1;
                continue;
            }
            if s.starts_with("hierarchy_stoplist")
                || s.starts_with("extension_methods")
                || s.starts_with("sslproxy_")
                || s.starts_with("icap_")
                || s.starts_with("adaptation_")
                || s.starts_with("ecap_")
                || s.starts_with("request_header_")
                || s.starts_with("reply_header_")
                || s.starts_with("external_acl_type")
                || s.starts_with("authenticate_")
                || s.starts_with("auth_param")
                || s.starts_with("acl_uses_indirect_client")
                || s.starts_with("delay_")
                || s.starts_with("loadable_modules")
                || s.starts_with("upgrade_http")
                || s.starts_with("tcp_outgoing_")
                || s.starts_with("zph_")
                || s.starts_with("qos_")
                || s.starts_with("wccp")
                || s.starts_with("snmp_")
            {
                c.extras += 1;
                continue;
            }
            c.directives += 1;
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_config() {
        let b = concat!(
            "# squid.conf\n",
            "acl localnet src 10.0.0.0/8\n",
            "acl SSL_ports port 443\n",
            "acl Safe_ports port 80\n",
            "acl CONNECT method CONNECT\n",
            "http_access allow localnet\n",
            "http_access deny !Safe_ports\n",
            "http_access deny CONNECT !SSL_ports\n",
            "icp_access allow localnet\n",
            "http_port 3128\n",
            "icp_port 3130\n",
            "hierarchy_stoplist cgi-bin ?\n",
            "cache_mem 256 MB\n",
            "cache_dir ufs /var/spool/squid 100 16 256\n",
            "access_log /var/log/squid/access.log\n",
            "refresh_pattern ^ftp: 1440 20% 10080\n",
            "coredump_dir /var/spool/squid\n",
        );
        let c = Squid::parse(b.as_bytes()).unwrap();
        assert_eq!(c.acls, 4);
        assert_eq!(c.accesses, 4);
        assert_eq!(c.verdicts, 4);
        assert_eq!(c.negations, 2);
        assert_eq!(c.ports, 2);
        assert_eq!(c.caches, 4);
        assert_eq!(c.refreshes, 1);
        assert_eq!(c.extras, 1);
        assert_eq!(c.directives, 0);
    }

    #[test]
    fn rejects_other() {
        assert!(Squid::parse(b"foo = 1").is_none());
    }
}
