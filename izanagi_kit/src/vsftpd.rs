//! `vsftpd.conf` (Very Secure FTP daemon) census.
//!
//! `key=value` assignments (no whitespace around `=`; comments `#`).
//! Keys are vsftpd-specific (`anonymous_enable`, `local_umask`,
//! `pasv_min_port`, `force_local_data_ssl`, `allow_writeable_chroot` …).
//!
//! ```rust
//! let v = b"anonymous_enable=NO\nlocal_enable=YES\nwrite_enable=YES\nlocal_umask=022\npasv_min_port=40000\npasv_max_port=40010\n";
//! assert!(izanagi_kit::vsftpd::detect(v));
//! let c = izanagi_kit::vsftpd::Vsftpd::parse(v).unwrap();
//! assert_eq!(c.settings, 6);
//! ```

/// vsftpd.conf census.
#[derive(Debug, Clone)]
pub struct Vsftpd {
    /// `key=value` lines matching a known vsftpd directive.
    pub settings: usize,
    /// Other `key=value` lines (unknown names).
    pub other: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// vsftpd directives (`man vsftpd.conf`).
const KEYS: &[&str] = &[
    "accept_timeout",
    "allow_anon_ssl",
    "allow_writeable_chroot",
    "anon_umask",
    "anon_world_readable_only",
    "anonymous_enable",
    "ascii_download_enable",
    "ascii_upload_enable",
    "async_abor_enable",
    "background",
    "check_shell",
    "chmod_enable",
    "chown_uploads",
    "chroot_list_enable",
    "chroot_local_user",
    "cmds_allowed",
    "connect_from_port_20",
    "deny_email_enable",
    "dirlist_enable",
    "dirmessage_enable",
    "download_enable",
    "dual_log_enable",
    "force_anon_data_ssl",
    "force_anon_logins_ssl",
    "force_local_data_ssl",
    "force_local_logins_ssl",
    "guest_enable",
    "guest_username",
    "hide_ids",
    "implicit_ssl",
    "isolate",
    "listen",
    "listen_ipv6",
    "local_enable",
    "local_root",
    "local_umask",
    "log_ftp_protocol",
    "ls_recurse_enable",
    "max_clients",
    "max_login_fails",
    "max_per_ip",
    "message_file",
    "nopriv_user",
    "one_process_model",
    "pam_service_name",
    "pasv_addr_resolve",
    "pasv_enable",
    "pasv_max_port",
    "pasv_min_port",
    "pasv_promiscuous",
    "port_enable",
    "port_promiscuous",
    "require_cert",
    "require_ssl_reuse",
    "reverse_lookup_enable",
    "rsa_cert_file",
    "rsa_private_key_file",
    "secure_chroot_dir",
    "secure_email_list_enable",
    "session_support",
    "setproctitle_enable",
    "ssl_enable",
    "ssl_sslv2",
    "ssl_sslv3",
    "ssl_tlsv1",
    "syslog_enable",
    "tcp_wrappers",
    "text_userdb_names",
    "tildes_enable",
    "use_localtime",
    "use_sendfile",
    "user_config_dir",
    "user_sub_token",
    "userlist_deny",
    "userlist_enable",
    "virtual_use_local_privs",
    "write_enable",
    "xferlog_enable",
    "xferlog_std_format",
];

fn assign_key(l: &str) -> Option<&str> {
    let t = l.trim();
    if t.is_empty() || t.starts_with('#') || t.starts_with('[') {
        return None;
    }
    let (k, _) = t.split_once('=')?;
    let k = k.trim();
    if k.is_empty() || k.contains(' ') {
        return None;
    }
    Some(k)
}

/// Detect a `vsftpd.conf`-style file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut n = 0usize;
    for l in t.lines() {
        if let Some(k) = assign_key(l) {
            if KEYS.contains(&k) {
                n += 1;
            }
        }
    }
    n >= 3
}

impl Vsftpd {
    /// Count directives. Returns `None` when the input does not look like
    /// a `vsftpd.conf`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            settings: 0,
            other: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if let Some(k) = assign_key(l) {
                if KEYS.contains(&k) {
                    c.settings += 1;
                } else {
                    c.other += 1;
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
    fn detects() {
        let b = b"# vsftpd\nanonymous_enable=NO\nlocal_enable=YES\nwrite_enable=YES\nlocal_umask=022\ndirmessage_enable=YES\nxferlog_enable=YES\nconnect_from_port_20=YES\nchroot_local_user=YES\nlisten=YES\npam_service_name=vsftpd\npasv_min_port=40000\npasv_max_port=40010\n";
        assert!(detect(b));
        let c = Vsftpd::parse(b).unwrap();
        assert_eq!(c.settings, 12);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"[Unit]\nDescription=x\n"));
        assert!(!detect(b"foo=1\nbar=2\n"));
        assert!(Vsftpd::parse(b"").is_none());
    }
}
