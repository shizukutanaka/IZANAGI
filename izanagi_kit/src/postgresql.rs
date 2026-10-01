//! PostgreSQL `postgresql.conf` parser.
//!
//! Detects `key = value` directives where the keys belong to the server
//! config namespace (`shared_buffers`, `max_connections`, `wal_level`,
//! `listen_addresses`, `ssl`, `autovacuum`, `log_*`, `checkpoint_*` …) and
//! counts directives, `#` comments, `include*` directives, and `on/true/yes`
//! boolean values.
//!
//! ```
//! let b = br#"# server
//! listen_addresses = 'localhost'
//! port = 5432
//! max_connections = 100
//! shared_buffers = 128MB
//! wal_level = replica
//! ssl = on
//! autovacuum = on"#;
//! assert!(izanagi_kit::postgresql::detect(b));
//! let c = izanagi_kit::postgresql::Postgresql::parse(b).unwrap();
//! assert_eq!(c.directives, 7);
//! assert_eq!(c.bool_values, 2);
//! ```

/// Parsed postgresql.conf summary.
#[derive(Debug, Clone)]
pub struct Postgresql {
    /// `key = value` directive lines.
    pub directives: usize,
    /// Recognized server-config keys seen.
    pub known_keys: usize,
    /// `on`/`true`/`yes`/`1` valued directives.
    pub bool_values: usize,
    /// `include`/`include_if_exists`/`include_dir` directives.
    pub includes: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Strong keys that identify postgresql.conf on their own.
const KEYS: &[&str] = &[
    "shared_buffers",
    "max_connections",
    "listen_addresses",
    "wal_level",
    "autovacuum",
    "work_mem",
    "maintenance_work_mem",
    "effective_cache_size",
    "max_wal_size",
    "min_wal_size",
    "checkpoint_timeout",
    "checkpoint_completion_target",
    "log_statement",
    "log_min_duration_statement",
    "log_directory",
    "log_filename",
    "log_destination",
    "logging_collector",
    "synchronous_commit",
    "fsync",
    "full_page_writes",
    "archive_mode",
    "archive_command",
    "hot_standby",
    "max_wal_senders",
    "max_replication_slots",
    "shared_preload_libraries",
    "dynamic_shared_memory_type",
    "jit",
    "track_activities",
    "track_counts",
    "hba_file",
    "ident_file",
    "external_pid_file",
    "unix_socket_directories",
    "bonjour",
    "port",
    "ssl",
    "ssl_cert_file",
    "ssl_key_file",
    "password_encryption",
    "krb_server_keyfile",
    "db_user_namespace",
    "search_path",
    "default_transaction_isolation",
    "statement_timeout",
    "lock_timeout",
    "idle_in_transaction_session_timeout",
    "vacuum_cost_delay",
    "bgwriter_delay",
    "effective_io_concurrency",
    "max_worker_processes",
    "max_parallel_workers",
    "max_parallel_maintenance_workers",
    "wal_buffers",
    "wal_writer_delay",
    "commit_delay",
    "commit_siblings",
    "recovery_target_timeline",
    "promote_trigger_file",
    "primary_conninfo",
    "restore_command",
    "cluster_name",
    "update_process_title",
    "restart_after_crash",
    "lc_messages",
    "lc_monetary",
    "lc_numeric",
    "lc_time",
    "server_encoding",
    "timezone",
    "log_timezone",
    "datestyle",
    "intervalstyle",
    "temp_buffers",
    "temp_file_limit",
    "max_files_per_process",
    "max_locks_per_transaction",
    "max_pred_locks_per_transaction",
    "row_security",
    "array_nulls",
    "backslash_quote",
    "escape_string_warning",
    "standard_conforming_strings",
    "exit_on_error",
    "data_directory",
    "config_file",
    "include",
    "include_if_exists",
    "include_dir",
];

fn key_of_line(tr: &str) -> Option<(&str, &str)> {
    let eq = tr.find('=')?;
    let key = tr[..eq].trim_end();
    if key.is_empty()
        || key.len() > 60
        || !key
            .chars()
            .all(|c| c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit() || c == '.')
    {
        return None;
    }
    Some((key, tr[eq + 1..].trim_start()))
}

/// Whether the buffer looks like a postgresql.conf.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = t
        .lines()
        .filter(|l| {
            let tr = l.trim();
            !tr.starts_with('#') && key_of_line(tr).is_some_and(|(k, _)| KEYS.contains(&k))
        })
        .count();
    hits >= 2
}

impl Postgresql {
    /// Count directives in a postgresql.conf. Returns `None` when the input
    /// does not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            directives: 0,
            known_keys: 0,
            bool_values: 0,
            includes: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            let Some((key, v)) = key_of_line(tr) else {
                continue;
            };
            c.directives += 1;
            if KEYS.contains(&key) {
                c.known_keys += 1;
            }
            if key.starts_with("include") {
                c.includes += 1;
            }
            let bare = v.trim_end_matches('#').trim_end();
            if matches!(bare, "on" | "true" | "yes" | "1" | "'on'" | "'true'") {
                c.bool_values += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = br#"# postgresql.conf
listen_addresses = 'localhost'   # listen
port = 5432
max_connections = 200
shared_buffers = 256MB
work_mem = 4MB
wal_level = replica
max_wal_senders = 4
archive_mode = off
ssl = on
logging_collector = on
log_statement = 'all'
include_if_exists = 'extra.conf'
"#;
        assert!(detect(b));
        let c = Postgresql::parse(b).unwrap();
        assert_eq!(c.directives, 12);
        assert_eq!(c.known_keys, 12);
        assert_eq!(c.bool_values, 2);
        assert_eq!(c.includes, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_ini_and_env() {
        assert!(!detect(
            b"[mysqld]\nskip-networking\nmax_allowed_packet=16M"
        ));
        assert!(!detect(b"FOO=bar\nBAZ=qux\nQUUX=zed\n"));
    }
}
