//! Rspamd `rspamd.conf` / `local.d/*.conf` (UCL format).
//!
//! ```
//! let b = b"worker \"normal\" {\n  bind_socket = \"localhost:11333\"\n}\nworker \"controller\" {\n  bind_socket = \"localhost:11334\"\n}\nactions {\n  reject = 15\n  add_header = 6\n}\n";
//! assert!(izanagi_kit::rspamdconf::detect(b));
//! let c = izanagi_kit::rspamdconf::Rspamd::parse(b).unwrap();
//! assert_eq!(c.worker_blocks, 2);
//! assert!(c.keys >= 4);
//! ```
const WORKER_KEYS: &[&str] = &[
    "bind_socket",
    "count",
    "enabled",
    "type",
    "mime",
    "timeout",
    "max_files",
];
const SECTION_KEYS: &[&str] = &[
    "actions",
    "classifier",
    "composite",
    "group",
    "logging",
    "lua",
    "metri\u{63}",
    "modules",
    "options",
    "plugins",
    "regexp",
    "rules",
    "statfile",
    "upstream",
    "worker",
];
const KEYS: &[&str] = &[
    "reject",
    "add_header",
    "greylist",
    "rewrite_subject",
    "quarantine",
    "discard",
    "soft_reject",
    "no_action",
    "pidfile",
    "tempdir",
    "filters",
    "map_watch_interval",
    "history_rows",
    "dns_timeouts",
    "task_timeout",
    "events_backend",
    "disable_hyperscan",
    "public_groups_only",
    "debug_modules",
    "neighbours",
    "local_networks",
    "mempool_vars",
    "explicit_modules",
    "spamd_never_fail",
    "use_argv",
    "use_env",
    "control_ip\u{63}",
    "hs_cache_dir",
    "ignore_punct",
    "allow_raw_input",
    "signed_only",
    "url_tld",
    "sync_key",
    "expire",
    "learn_condition",
    "rows_used",
    "hash_len",
    "retransmits",
    "error_time",
    "revive_time",
    "resolve_timeout",
    "rate_limit",
    "log_id",
    "log_color",
    "log_re_cache",
    "log_url",
    "log_size",
    "log_buf_type",
    "log_severity",
    "log_json",
    "log_plain",
    "log_verbose",
    "debug_ip",
    "syslog",
    "file",
    "filename",
    "facility",
    "level",
    "language",
    "show_tags",
    "include_user_settings",
    "global_functions",
    "allow_learn",
    "max_size",
    "skip_maps",
    "path",
    "host",
    "password",
    "db",
    "servers",
    "read_servers",
    "write_servers",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(k)
}

/// Detect Rspamd UCL configuration.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut hits = 0usize;
    for k in KEYS {
        if key_present(t, k) {
            hits += 1;
        }
    }
    let worker = t.contains("worker");
    let ucl_assign = t.contains(" = ") || t.contains(" =\n") || t.contains('=');
    hits >= 3 && (worker || (ucl_assign && t.contains('{')))
}

/// Structural counts for a Rspamd config.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rspamd {
    /// Recognized key occurrences.
    pub keys: usize,
    /// `worker "<name>" {` blocks.
    pub worker_blocks: usize,
    /// `name {` UCL sections (incl. worker blocks).
    pub sections: usize,
    /// `key = value` lines.
    pub assignments: usize,
    /// `#`/`//`/`/* */` comment lines.
    pub comments: usize,
}

impl Rspamd {
    /// Parse structural counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            worker_blocks: 0,
            sections: 0,
            assignments: 0,
            comments: 0,
        };
        let mut in_block_comment = false;
        for l in t.lines() {
            let tr = l.trim();
            if in_block_comment {
                if tr.contains("*/") {
                    in_block_comment = false;
                }
                c.comments += 1;
                continue;
            }
            if tr.starts_with("/*") {
                in_block_comment = true;
                c.comments += 1;
                continue;
            }
            if tr.starts_with('#') || tr.starts_with("//") {
                c.comments += 1;
            } else if tr.starts_with("worker") && tr.contains('{') {
                c.worker_blocks += 1;
                c.sections += 1;
            } else if SECTION_KEYS.iter().any(|s| tr.starts_with(s)) && tr.contains('{') {
                c.sections += 1;
            } else if tr.contains('=') {
                c.assignments += 1;
            }
        }
        for k in KEYS {
            c.keys += t.matches(k).count();
        }
        for k in WORKER_KEYS {
            c.keys += t.matches(k).count();
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"worker \"normal\" {\n  bind_socket = \"localhost:11333\"\n  count = 4\n}\nactions {\n  reject = 15\n  add_header = 6\n  greylist = 4\n}\noptions {\n  pidfile = \"/run/rspamd.pid\"\n  filters = \"chartable,dkim,spf\"\n}\n";
        assert!(detect(b));
        let c = Rspamd::parse(b).unwrap();
        assert_eq!(c.worker_blocks, 1);
        assert_eq!(c.sections, 3);
        assert_eq!(c.assignments, 7);
        assert!(c.keys >= 8);
    }

    #[test]
    fn rejects_nginx() {
        let b = b"server {\n  listen 80;\n  root /var/www;\n}\n";
        assert!(!detect(b));
        assert!(Rspamd::parse(b).is_none());
    }
}
