//! Postfix `main.cf`/`master.cf` census.
//!
//! `main.cf` holds `key = value` parameters (`myhostname`,
//! `inet_interfaces`, `mydestination`, `alias_maps`, `smtpd_*`,
//! `smtpd_recipient_restrictions`, `mynetworks`, `relayhost`,
//! `transport_maps`, `virtual_alias_maps`, `mailbox_size_limit`,
//! `message_size_limit`, `smtp_tls_*`, `compatibility_level`,
//! `home_mailbox`, `mail_spool_directory`, `command_directory`,
//! `daemon_directory`, `data_directory`, `mail_owner`, `queue_directory`,
//! `sendmail_path`, `newaliases_path`, `mailq_path`, `setgid_group`,
//! `html_directory`, `manpage_directory`, `sample_directory`,
//! `readme_directory`, `meta_directory`, `shlib_directory`,
//! `inet_protocols`, `mynetworks_style`, `myorigin`, `mydomain`,
//! `masquerade_domains`, `canonical_maps`, `sender_canonical_maps`,
//! `recipient_canonical_maps`, `relocated_maps`, `header_checks`,
//! `body_checks`, `mime_header_checks`, `smtpd_banner`,
//! `smtpd_helo_required`, `smtpd_helo_restrictions`,
//! `smtpd_sender_restrictions`, `smtpd_relay_restrictions`,
//! `smtpd_client_restrictions`, `smtpd_milters`,
//! `non_smtpd_milters`, `disable_vrfy_command`,
//! `smtputf8_enable`, `strict_rfc821_envelopes`,
//! `address_verify_map`, `unverified_recipient_reject_code`,
//! `postscreen_*`, `dnsblog_*`, `tlsproxy_*`, `smtp_dns_support_level`,
//! `smtp_host_lookup`, `lmtp_host_lookup`,
//! `disable_dns_lookups`, `smtp_address_preference`,
//! `always_add_missing_headers`, `local_recipient_maps`,
//! `local_transport`, `default_transport`, `relay_transport`,
//! `virtual_transport`, `default_privs`, `default_process_limit`,
//! `max_idle`, `max_use`, `bounce_queue_lifetime`,
//! `maximal_queue_lifetime`, `minimal_backoff_time`,
//! `maximal_backoff_time`, `queue_run_delay`,
//! `bounce_notice_recipient`, `delay_notice_recipient`,
//! `error_notice_recipient`, `notify_classes`,
//! `smtpd_error_sleep_time`, `smtpd_hard_error_limit`,
//! `smtpd_soft_error_limit`, `smtpd_client_connection_count_limit`,
//! `smtpd_client_connection_rate_limit`, `anvil_rate_time_unit`,
//! `smtpd_peername_lookup`, `smtpd_discard_ehlo_keywords`,
//! `smtpd_expansion_filter`, `smtpd_forbidden_commands`,
//! `smtpd_junk_command_limit`, `smtpd_noop_commands`,
//! `smtpd_null_access_lookup_key`, `smtpd_proxy_filter`,
//! `smtpd_proxy_timeout`, `smtpd_proxy_ehlo`,
//! `smtpd_sasl_*`, `smtpd_tls_*`, `smtps_*`, `tls_*`,
//! `debugger_command`, `debug_peer_level`, `debug_peer_list`,
//! `daemon_timeout`, `ipc_timeout`, `command_execution_directory`,
//! `import_environment`, `mail_name`, `mail_version`,
//! `authorized_submit_users`, `authorized_flush_users`,
//! `propagate_unmatched_extensions`, `recipient_delimiter`,
//! `owner_request_special`, `twin_vendor_id`,
//! `alternative_config_directories`, `multi_instance_*`.
//!
//! `master.cf` rows are `service type private unpriv chroot wakeup
//! maxproc command` (`smtp inet n - y - - smtpd`, `-o` option lines).
//!
//! ```rust
//! let m = "myhostname = mail.example.org\ninet_interfaces = all\nmydestination = localhost, example.org\nsmtpd_banner = $myhostname ESMTP\n";
//! let c = izanagi_kit::postfix::Postfix::parse(m.as_bytes()).unwrap();
//! assert_eq!(c.settings, 4);
//! ```

/// main.cf/master.cf census.
#[derive(Debug, Clone)]
pub struct Postfix {
    /// `key = value` parameter lines.
    pub settings: usize,
    /// Recognised main.cf parameter names.
    pub named: usize,
    /// Continuation lines (leading whitespace).
    pub continuations: usize,
    /// `master.cf` service rows (`name type … command`).
    pub master_rows: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "myhostname",
    "mydomain",
    "myorigin",
    "inet_interfaces",
    "inet_protocols",
    "mydestination",
    "mynetworks",
    "mynetworks_style",
    "relayhost",
    "alias_maps",
    "alias_database",
    "transport_maps",
    "virtual_alias_maps",
    "virtual_alias_domains",
    "mailbox_size_limit",
    "message_size_limit",
    "compatibility_level",
    "home_mailbox",
    "mail_spool_directory",
    "command_directory",
    "daemon_directory",
    "data_directory",
    "mail_owner",
    "queue_directory",
    "sendmail_path",
    "newaliases_path",
    "mailq_path",
    "setgid_group",
    "html_directory",
    "manpage_directory",
    "sample_directory",
    "readme_directory",
    "meta_directory",
    "shlib_directory",
    "smtpd_banner",
    "smtpd_helo_required",
    "smtpd_helo_restrictions",
    "smtpd_sender_restrictions",
    "smtpd_recipient_restrictions",
    "smtpd_relay_restrictions",
    "smtpd_client_restrictions",
    "smtpd_milters",
    "non_smtpd_milters",
    "smtpd_sasl_auth_enable",
    "smtpd_sasl_type",
    "smtpd_sasl_path",
    "smtpd_sasl_security_options",
    "smtpd_tls_cert_file",
    "smtpd_tls_key_file",
    "smtpd_tls_security_level",
    "smtp_tls_security_level",
    "disable_vrfy_command",
    "local_recipient_maps",
    "local_transport",
    "default_transport",
    "relay_transport",
    "virtual_transport",
    "default_process_limit",
    "maximal_queue_lifetime",
    "bounce_queue_lifetime",
    "minimal_backoff_time",
    "maximal_backoff_time",
    "queue_run_delay",
    "notify_classes",
    "bounce_notice_recipient",
    "delay_notice_recipient",
    "error_notice_recipient",
    "header_checks",
    "body_checks",
    "mime_header_checks",
    "canonical_maps",
    "sender_canonical_maps",
    "recipient_canonical_maps",
    "relocated_maps",
    "recipient_delimiter",
    "debugger_command",
    "debug_peer_level",
    "debug_peer_list",
    "daemon_timeout",
    "import_environment",
    "postscreen_access_list",
    "postscreen_dnsbl_sites",
    "postscreen_greet_wait",
    "smtputf8_enable",
    "always_add_missing_headers",
    "masquerade_domains",
    "parent_domain_matches_subdomains",
    "smtp_host_lookup",
    "disable_dns_lookups",
];

const MASTER_TYPES: &[&str] = &["inet", "unix", "unix-dgram", "fifo", "pass"];
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// Detect Postfix `main.cf`/`master.cf` content.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let t = strip_bom(t);
    let mut kv_named = 0usize;
    let mut master = 0usize;
    for line in t.lines() {
        let s = line.trim();
        if s.is_empty() || s.starts_with('#') {
            continue;
        }
        if let Some(eq) = s.find('=') {
            let key = s[..eq].trim();
            if !key.is_empty()
                && key.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
                && KEYS.contains(&key)
            {
                kv_named += 1;
            }
            continue;
        }
        let mut it = s.split_whitespace();
        let svc = it.next().unwrap_or("");
        let ty = it.next().unwrap_or("");
        if !svc.is_empty()
            && svc
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
            && MASTER_TYPES.contains(&ty)
        {
            master += 1;
        }
    }
    kv_named >= 2 || master >= 2
}

impl Postfix {
    /// Census a `main.cf`/`master.cf` buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            settings: 0,
            named: 0,
            continuations: 0,
            master_rows: 0,
            comments: 0,
        };
        let mut prev_continues = false;
        for line in t.lines() {
            let raw = line;
            let s = raw.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                prev_continues = false;
                continue;
            }
            if (raw.starts_with(' ') || raw.starts_with('\t')) && prev_continues {
                c.continuations += 1;
                continue;
            }
            if s.starts_with('-') {
                prev_continues = false;
                continue;
            }
            if let Some(eq) = s.find('=') {
                let key = s[..eq].trim();
                if !key.is_empty() && key.bytes().all(|x| x.is_ascii_alphanumeric() || x == b'_') {
                    c.settings += 1;
                    if KEYS.contains(&key) {
                        c.named += 1;
                    }
                    prev_continues = true;
                    continue;
                }
            }
            let mut it = s.split_whitespace();
            let _svc = it.next().unwrap_or("");
            let ty = it.next().unwrap_or("");
            if MASTER_TYPES.contains(&ty) {
                c.master_rows += 1;
            }
            prev_continues = false;
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_main_cf() {
        let b = b"myhostname = mail.example.org\ninet_interfaces = all\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[x]\ny=1\n"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses_cfg() {
        let b = concat!(
            "# postfix main\n",
            "myhostname = mail.example.org\n",
            "mydomain = example.org\n",
            "inet_interfaces = all\n",
            "inet_protocols = ipv4\n",
            "mydestination = localhost,\n",
            "    example.org, mail.example.org\n",
            "alias_maps = hash:/etc/aliases\n",
            "smtpd_recipient_restrictions = permit_mynetworks,\n",
            "    permit_sasl_authenticated\n",
            "smtp inet n - y - - smtpd\n",
            "  -o syslog_name=postfix/smtp\n",
            "submission inet n - y - - smtpd\n",
        );
        let c = Postfix::parse(b.as_bytes()).unwrap();
        assert_eq!(c.settings, 7);
        assert_eq!(c.named, 7);
        assert_eq!(c.continuations, 2);
        assert_eq!(c.master_rows, 2);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
