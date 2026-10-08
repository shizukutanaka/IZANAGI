//! Dovecot `dovecot.conf` census.
//!
//! `key = value` settings (`mail_location`, `mail_privileged_group`,
//! `protocols`, `listen`, `ssl_cert`, `ssl_key`, `ssl_protocols`,
//! `disable_plaintext_auth`, `auth_mechanisms`, `login_greeting`,
//! `log_path`, `info_log_path`, `mail_max_userip_connections`,
//! `first_valid_uid`, `last_valid_uid`, `default_login_user`,
//! `default_internal_user`, `hostname`, `postmaster_address`,
//! `auth_*`, `mail_plugins`, `mail_attribute_dict`, `lda_*`,
//! `imap_*`, `pop3_*`, `lmtp_*`, `managesieve_*`, `dict`,
//! `userdb`, `passdb`, `verbose_proctitle`, `shutdown_clients`,
//! `doveadm_*`, `imapc_*`, `pop3c_*`, `mail_server_*`,
//! `mbox_*`, `maildir_*`, `fs_*`, `obox_*`, `dsync_*`,
//! `replication_*`, `service_*`, `plugin`, `namespace`).
//!
//! Blocks: `protocol <name> {`, `service <name> {`, `mailbox <name> {`,
//! `namespace <name> {`, `auth {`, `passdb {`, `userdb {`, `plugin {`,
//! `dict {`, `ssl {`, `local_name <x> {`, `remote <x> {`,
//! `unix_listeners`/`inet_listeners`. `!include`/`!include_try` pull
//! in extra files.
//!
//! ```rust
//! let d = "protocols = imap lmtp\nmail_location = maildir:~/Maildir\nservice imap-login {\n  inet_listener imap {\n    port = 143\n  }\n}\n";
//! let c = izanagi_kit::dovecot::Dovecot::parse(d.as_bytes()).unwrap();
//! assert_eq!(c.blocks, 2);
//! assert_eq!(c.settings, 3);
//! ```

/// dovecot.conf census.
#[derive(Debug, Clone)]
pub struct Dovecot {
    /// `{ … }` block openings (protocol/service/mailbox/…).
    pub blocks: usize,
    /// `key = value` lines.
    pub settings: usize,
    /// Recognised dovecot setting names.
    pub named: usize,
    /// `!include`/`!include_try` directives.
    pub includes: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const BLOCK_HEADS: &[&str] = &[
    "protocol",
    "service",
    "mailbox",
    "namespace",
    "auth",
    "passdb",
    "userdb",
    "plugin",
    "dict",
    "ssl",
    "local_name",
    "remote",
    "imapc",
    "pop3c",
    "metric",
    "unix_listener",
    "inet_listener",
    "fifo_listener",
    "fields",
    "event",
];

const KEYS: &[&str] = &[
    "protocols",
    "listen",
    "base_dir",
    "instance_name",
    "login_greeting",
    "login_trusted_networks",
    "login_access_sockets",
    "mail_location",
    "mail_privileged_group",
    "mail_uid",
    "mail_gid",
    "mail_plugins",
    "mail_attribute_dict",
    "mail_max_userip_connections",
    "mail_temp_dir",
    "mail_fsync",
    "mail_nfs_index",
    "mail_nfs_storage",
    "mmap_disable",
    "dotlock_use_excl",
    "first_valid_uid",
    "last_valid_uid",
    "first_valid_gid",
    "last_valid_gid",
    "default_login_user",
    "default_internal_user",
    "default_internal_group",
    "hostname",
    "postmaster_address",
    "auth_mechanisms",
    "auth_username_format",
    "auth_username_translation",
    "auth_cache_size",
    "auth_cache_ttl",
    "auth_cache_negative_ttl",
    "auth_realms",
    "auth_default_realm",
    "auth_master_user_separator",
    "auth_anonymous_username",
    "auth_worker_max_count",
    "auth_failure_delay",
    "auth_ssl_require_client_cert",
    "auth_ssl_username_from_cert",
    "auth_verbose",
    "auth_debug",
    "auth_debug_passwords",
    "disable_plaintext_auth",
    "ssl",
    "ssl_cert",
    "ssl_key",
    "ssl_ca",
    "ssl_cert_username_field",
    "ssl_dh",
    "ssl_cipher_list",
    "ssl_cipher_suites",
    "ssl_protocols",
    "ssl_min_protocol",
    "ssl_prefer_server_ciphers",
    "ssl_options",
    "ssl_client_ca_dir",
    "ssl_client_ca_file",
    "ssl_client_cert",
    "ssl_client_key",
    "verbose_proctitle",
    "shutdown_clients",
    "log_path",
    "info_log_path",
    "log_timestamp",
    "syslog_facility",
    "version_ignore",
    "mail_debug",
    "mail_log_prefix",
    "imap_capability",
    "imap_id_send",
    "imap_id_log",
    "imap_client_workarounds",
    "imap_max_line_length",
    "imap_metadata",
    "imap_idle_notify_interval",
    "imap_urlauth_host",
    "imap_urlauth_port",
    "pop3_no_flag_updates",
    "pop3_enable_last",
    "pop3_reuse_xuidl",
    "pop3_client_workarounds",
    "pop3_logout_format",
    "pop3_uidl_format",
    "lmtp_rcpt_check_quota",
    "lmtp_save_to_detail_mailbox",
    "lmtp_hdr_delivery_address",
    "managesieve_max_line_length",
    "managesieve_implementation_string",
    "managesieve_max_compile_size",
    "lda_mailbox_autocreate",
    "lda_mailbox_autosubscribe",
    "lda_original_recipient_header",
    "rejection_reason",
    "rejection_subject",
    "doveadm_worker_count",
    "doveadm_socket_path",
    "doveadm_port",
    "doveadm_password",
    "doveadm_allowed_commands",
    "dict",
    "userdb",
    "passdb",
    "namespace",
    "plugin",
    "protocol",
    "service",
    "mailbox",
    "quota",
    "quota_max_mail_size",
    "quota_warning",
    "imapc_host",
    "imapc_port",
    "imapc_user",
    "imapc_password",
    "imapc_features",
    "pop3c_host",
    "pop3c_port",
    "pop3c_user",
    "pop3c_password",
    "mail_server_admin",
    "mail_server_comment",
    "mail_location_syntax",
    "mbox_read_locks",
    "mbox_write_locks",
    "mbox_lock_timeout",
    "mbox_dotlock_change_timeout",
    "mbox_min_index_size",
    "mbox_dirty_syncs",
    "mbox_very_dirty_syncs",
    "maildir_stat_dirs",
    "maildir_copy_with_hardlinks",
    "maildir_broken_filename_sizes",
    "maildir_empty_new",
    "maildir_filename_prefix",
    "fs_posix_prefix",
    "obox_fs",
    "mailbox_list_index",
    "mailbox_list_index_noreselect",
    "dsync_remote_cmd",
    "dsync_alt_move",
    "replication_dsync_parameters",
    "replica",
    "service_count",
    "inbox",
    "special_use",
    "prefix",
    "separator",
    "type",
    "process_min_avail",
    "process_limit",
    "client_limit",
    "idle_kill",
    "vsz_limit",
    "port",
    "address",
    "ssl",
    "user",
    "group",
    "mode",
];
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// Detect dovecot.conf content.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let t = strip_bom(t);
    let mut named = 0usize;
    let mut blocks = 0usize;
    for line in t.lines() {
        let s = line.trim();
        if s.is_empty() || s.starts_with('#') {
            continue;
        }
        if s.starts_with("!include") {
            named += 1;
            continue;
        }
        if s.ends_with('{') {
            let head = s.trim_end_matches('{').trim();
            let word = head.split_whitespace().next().unwrap_or("");
            if BLOCK_HEADS.contains(&word) {
                blocks += 1;
                continue;
            }
        }
        if let Some(eq) = s.find('=') {
            let key = s[..eq].trim();
            if !key.is_empty()
                && key.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
                && KEYS.contains(&key)
            {
                named += 1;
            }
        }
    }
    named >= 2 || (named >= 1 && blocks >= 1)
}

impl Dovecot {
    /// Census a dovecot.conf buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            blocks: 0,
            settings: 0,
            named: 0,
            includes: 0,
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
            if s.starts_with("!include") {
                c.includes += 1;
                continue;
            }
            if s.ends_with('{') {
                let head = s.trim_end_matches('{').trim();
                let word = head.split_whitespace().next().unwrap_or("");
                if BLOCK_HEADS.contains(&word) {
                    c.blocks += 1;
                    continue;
                }
            }
            if s == "}" || s.starts_with('}') {
                continue;
            }
            if let Some(eq) = s.find('=') {
                let key = s[..eq].trim();
                if !key.is_empty() && key.bytes().all(|x| x.is_ascii_alphanumeric() || x == b'_') {
                    c.settings += 1;
                    if KEYS.contains(&key) {
                        c.named += 1;
                    }
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
    fn detects_conf() {
        let b = b"protocols = imap\nmail_location = maildir:~/Maildir\n";
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
            "## Dovecot conf\n",
            "!include_try /usr/share/dovecot/protocols.d/*.protocol\n",
            "protocols = imap lmtp submission\n",
            "listen = *, ::\n",
            "mail_location = maildir:~/Maildir\n",
            "mail_privileged_group = mail\n",
            "disable_plaintext_auth = yes\n",
            "auth_mechanisms = plain login\n",
            "ssl_cert = </etc/ssl/mail.crt\n",
            "ssl_key = </etc/ssl/mail.key\n",
            "service imap-login {\n",
            "  inet_listener imap {\n",
            "    port = 143\n",
            "  }\n",
            "}\n",
            "namespace inbox {\n",
            "  inbox = yes\n",
            "  mailbox Sent {\n",
            "    special_use = \\Sent\n",
            "  }\n",
            "}\n",
        );
        let c = Dovecot::parse(b.as_bytes()).unwrap();
        assert_eq!(c.blocks, 4);
        assert_eq!(c.settings, 11);
        assert!(c.named >= 10);
        assert_eq!(c.includes, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
