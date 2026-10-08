//! Exim `configure` file census.
//!
//! Sections are introduced by `begin <name>` (`acl`, `routers`,
//! `transports`, `authenticators`, `retry`, `rewrite`, `main`,
//! `lookup`, `local_scan`, `log`, `smtp`). Top-level main options
//! and per-driver options use `key = value` (`driver = dnslookup`,
//! `domains = +local_domains`, `transport = remote_smtp`,
//! `router_home_directory`, `accept`, `deny`, `warn`, `require`,
//! `verify`, `hosts`, `condition`, `data`, `message`, `log_message`,
//! `hosts_try_dkim`, `dkim_*`, `tls_*`, `acl_smtp_*`, `local_domains`,
//! `relay_to_domains`, `system_aliases`, `message_body_visible`,
//! `errors_reply_to`, `print_topbit_chars`, `tls_certificate`,
//! `tls_privatekey`, `primary_hostname`, `domainlist`,
//! `hostlist`, `addresslist`, `check_log_space`,
//! `check_spool_space`, `smtp_accept_max`,
//! `smtp_accept_queue_per_connection`, `smtp_banner`,
//! `smtp_check_spool_space`, `smtp_connect_backlog`,
//! `smtp_enforce_sync`, `smtp_receive_timeout`,
//! `smtp_reserve_hosts`, `smtp_return_error_details`,
//! `log_selector`, `log_timezone`, `split_spool_directory`,
//! `spool_directory`, `exim_user`, `exim_group`,
//! `gecos_pattern`, `gecos_name`, `local_from_prefix`,
//! `local_from_suffix`, `untrusted_set_sender`,
//! `never_users`, `trusted_users`, `trusted_groups`,
//! `admin_groups`, `qualify_domain`, `qualify_recipient`,
//! `recipients_max`, `ignore_bounce_errors_after`,
//! `timeout_frozen_after`, `auto_thaw`, `freeze_tell`,
//! `deliver_queue_load_max`, `queue_only_load`,
//! `queue_run_max`, `queue_smtp_domains`,
//! `remote_max_parallel`, `smtp_max_outgoing`,
//! `daemon_smtp_ports`, `local_interfaces`,
//! `extra_local_interfaces`, `pid_file_path`,
//! `keep_environment`, `add_environment`,
//! `openssl_options`, `openssl_cipherlist`,
//! `mysql_servers`, `pgsql_servers`, `oracle_servers`,
//! `ldap_default_servers`, `ldap_version`,
//! `spamd_address`, `av_scanner`, `prdr_enable`,
//! `chunking_advertise_hosts`, `pipelining_advertise_hosts`,
//! `auth_advertise_hosts`, `tls_advertise_hosts`,
//! `received_header_text`, `received_headers_max`,
//! `smtp_receive_timeout`, `rfc1413_*`).
//!
//! Option lines may use `${…}` expansions and `:` domain lists.
//!
//! ```rust
//! let e = "primary_hostname = mail.example.org\nbegin acl\nacl_smtp_rcpt:\n  accept hosts = :\nbegin routers\ndnslookup:\n  driver = dnslookup\n  transport = remote_smtp\n";
//! let c = izanagi_kit::exim::Exim::parse(e.as_bytes()).unwrap();
//! assert_eq!(c.sections, 2);
//! ```

/// exim configure census.
#[derive(Debug, Clone)]
pub struct Exim {
    /// `begin <name>` section headers.
    pub sections: usize,
    /// `key = value` option lines.
    pub settings: usize,
    /// Driver/option keywords recognised.
    pub named: usize,
    /// `${` string-expansion occurrences.
    pub expansions: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const SECTIONS: &[&str] = &[
    "acl",
    "routers",
    "transports",
    "authenticators",
    "retry",
    "rewrite",
    "main",
    "lookup",
    "local_scan",
    "log",
    "smtp",
];

const KEYS: &[&str] = &[
    "primary_hostname",
    "domainlist",
    "hostlist",
    "addresslist",
    "local_domains",
    "relay_to_domains",
    "qualify_domain",
    "qualify_recipient",
    "acl_smtp_rcpt",
    "acl_smtp_data",
    "acl_smtp_mail",
    "acl_smtp_connect",
    "acl_smtp_helo",
    "acl_smtp_vrfy",
    "acl_smtp_quit",
    "acl_smtp_predata",
    "acl_smtp_mime",
    "acl_smtp_notquit",
    "acl_not_smtp",
    "acl_not_smtp_start",
    "exim_user",
    "exim_group",
    "spool_directory",
    "log_selector",
    "log_timezone",
    "split_spool_directory",
    "daemon_smtp_ports",
    "local_interfaces",
    "tls_certificate",
    "tls_privatekey",
    "tls_verify_certificates",
    "tls_dhparam",
    "tls_on_connect_ports",
    "message_body_visible",
    "errors_reply_to",
    "recipients_max",
    "ignore_bounce_errors_after",
    "timeout_frozen_after",
    "auto_thaw",
    "queue_run_max",
    "remote_max_parallel",
    "never_users",
    "trusted_users",
    "trusted_groups",
    "admin_groups",
    "smtp_accept_max",
    "smtp_banner",
    "smtp_receive_timeout",
    "smtp_return_error_details",
    "av_scanner",
    "spamd_address",
    "driver",
    "domains",
    "local_parts",
    "senders",
    "hosts",
    "transport",
    "router",
    "condition",
    "data",
    "file",
    "directory",
    "user",
    "group",
    "mode",
    "headers_add",
    "headers_remove",
    "return_path_add",
    "envelope_to_add",
    "delivery_date_add",
    "check_local_user",
    "require_files",
    "verify",
    "accept",
    "deny",
    "warn",
    "require",
    "defer",
    "discard",
    "drop",
    "message",
    "log_message",
    "logwrite",
    "set",
    "continue",
    "endpass",
    "no_more",
    "unseen",
    "cannot_route_message",
    "dns_again_means_nonexist",
    "ignore_target_hosts",
    "same_domain_copy_routing",
    "self",
    "allow_fail",
    "allow_defer",
    "fail_verify",
    "fail_verify_recipient",
    "translate_ip_address",
    "host_find_failed",
    "host_lookup",
    "dkim_domain",
    "dkim_selector",
    "dkim_private_key",
    "dkim_canon",
    "dkim_strict",
    "dkim_sign_headers",
    "hosts_try_dkim",
    "hosts_try_auth",
    "hosts_try_prdr",
    "hosts_try_chunking",
    "hosts_avoid_tls",
    "hosts_require_tls",
    "hosts_require_auth",
    "hosts_max_avoid_tls",
    "interface",
    "helo_data",
    "port",
    "connect_timeout",
    "command_timeout",
    "data_timeout",
    "final_timeout",
    "timeout",
    "retry_use_local_part",
    "batch_max",
    "max_rcpt",
    "serialize_hosts",
    "rcpt_include_affixes",
    "ignore_bounce_errors",
    "delivery_driver",
    "errors_to",
    "from",
    "to",
    "cc",
    "bcc",
    "subject",
    "once",
    "once_repeat",
    "once_file_size",
    "reply_transport",
    "return_message",
    "maildir_format",
    "maildir_tag",
    "maildirfolder_create_regex",
    "maildir_use_size_file",
    "quota",
    "quota_size_regex",
    "quota_warn_threshold",
    "quota_warn_message",
    "current_directory",
    "home_directory",
    "message_prefix",
    "message_suffix",
    "delivery_time_add",
    "local_delivery_batch",
    "local_delivery_batch_max",
    "rcpt_include_affixes",
    "client_secret",
    "client_name",
    "public_name",
    "server_secret",
    "server_set_id",
    "server_condition",
    "server_advertise_condition",
    "server_mail_auth_condition",
    "server_debug_print",
    "server_hostname",
    "server_scram_salt",
    "server_scram_iterations",
    "server_prompts",
    "server_xoauth2_server",
    "wildcard",
];
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// Detect exim configure content.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let t = strip_bom(t);
    let mut begins = 0usize;
    let mut named = 0usize;
    for line in t.lines() {
        let s = line.trim();
        if s.is_empty() || s.starts_with('#') {
            continue;
        }
        if let Some(rest) = s.strip_prefix("begin") {
            let word = rest.split_whitespace().next().unwrap_or("");
            if SECTIONS.contains(&word) {
                begins += 1;
                continue;
            }
        }
        if let Some(eq) = s.find('=') {
            let key = s[..eq].split_whitespace().next().unwrap_or("");
            if !key.is_empty()
                && key.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
                && KEYS.contains(&key)
            {
                named += 1;
            }
        } else {
            let word = s.split_whitespace().next().unwrap_or("");
            if KEYS.contains(&word) {
                named += 1;
            }
        }
    }
    begins >= 1 && (begins + named) >= 2
}

impl Exim {
    /// Census an exim configure buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            sections: 0,
            settings: 0,
            named: 0,
            expansions: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            c.expansions += s.matches("${").count();
            if let Some(rest) = s.strip_prefix("begin") {
                let word = rest.split_whitespace().next().unwrap_or("");
                if SECTIONS.contains(&word) {
                    c.sections += 1;
                    continue;
                }
            }
            if let Some(eq) = s.find('=') {
                let key = s[..eq].split_whitespace().next().unwrap_or("");
                if !key.is_empty() && key.bytes().all(|x| x.is_ascii_alphanumeric() || x == b'_') {
                    c.settings += 1;
                    if KEYS.contains(&key) {
                        c.named += 1;
                    }
                    continue;
                }
            }
            let word = s.split_whitespace().next().unwrap_or("");
            if KEYS.contains(&word) {
                c.named += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_conf() {
        let b = b"begin acl\n  accept hosts = :\nbegin routers\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[x]\ny=1\n"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses_conf() {
        let b = concat!(
            "# exim4\n",
            "primary_hostname = mail.example.org\n",
            "domainlist local_domains = @ : localhost\n",
            "acl_smtp_rcpt = acl_check_rcpt\n",
            "begin acl\n",
            "acl_check_rcpt:\n",
            "  accept hosts = :\n",
            "  deny message = Relay denied\n",
            "begin routers\n",
            "dnslookup:\n",
            "  driver = dnslookup\n",
            "  domains = ! +local_domains\n",
            "  transport = remote_smtp\n",
            "  ignore_target_hosts = ${lookup dnsdb{}}\n",
            "  no_more\n",
            "begin transports\n",
            "remote_smtp:\n",
            "  driver = smtp\n",
        );
        let c = Exim::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.settings, 10);
        assert!(c.named >= 10);
        assert_eq!(c.expansions, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
