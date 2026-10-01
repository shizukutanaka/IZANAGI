//! MySQL/MariaDB option file (`my.cnf`/`my.ini`) parser.
//!
//! Detects INI groups (`[mysqld]`/`[client]`/`[mysql]`/`[mysqldump]`/
//! `[mysqld_safe]`/`[mysqld_multi]`/`[server]`/`[embedded]`/`[galera]`/
//! `[mariadb*]`) and `key=value` or bare `skip-*`/`no-*` options, and counts
//! groups, options, `!include*` directives, and bare flag options.
//!
//! ```
//! let b = br#"[client]
//! port = 3306
//!
//! [mysqld]
//! skip-networking
//! max_connections = 100
//! innodb_buffer_pool_size = 128M
//! log_error = /var/log/mysql.err"#;
//! assert!(izanagi_kit::mysql::detect(b));
//! let c = izanagi_kit::mysql::Mysql::parse(b).unwrap();
//! assert_eq!(c.groups, 2);
//! assert_eq!(c.options, 5);
//! ```

/// Parsed my.cnf summary.
#[derive(Debug, Clone)]
pub struct Mysql {
    /// `[group]` headers.
    pub groups: usize,
    /// Option lines (`key=value` or bare flag).
    pub options: usize,
    /// Recognized server keys.
    pub known_keys: usize,
    /// Bare flag options (`skip-x`, `no-x`, or valueless names).
    pub flags: usize,
    /// `!include`/`!includedir` directives.
    pub includes: usize,
    /// `#`/`;` comment lines.
    pub comments: usize,
}

/// Group names accepted as MySQL option-file sections.
const GROUPS: &[&str] = &[
    "mysqld",
    "client",
    "client-server",
    "client-mariadb",
    "mysql",
    "mysql_upgrade",
    "mysqladmin",
    "mysqlbinlog",
    "mysqlcheck",
    "mysqldump",
    "mysqlimport",
    "mysqlshow",
    "mysqlslap",
    "mysqld_safe",
    "mysqld_multi",
    "myisamchk",
    "myisampack",
    "server",
    "embedded",
    "galera",
    "mariadb",
    "mariadb-client",
    "mariadb-server",
    "xtrabackup",
    "sst",
    "wsrep",
    "innobackupex",
    "isamchk",
    "pack_isam",
];

/// Recognized option keys.
const KEYS: &[&str] = &[
    "bind-address",
    "bind_address",
    "port",
    "socket",
    "datadir",
    "basedir",
    "pid-file",
    "pid_file",
    "user",
    "character-set-server",
    "character_set_server",
    "collation-server",
    "collation_server",
    "default-storage-engine",
    "default_storage_engine",
    "max_connections",
    "max_allowed_packet",
    "table_open_cache",
    "thread_cache_size",
    "sort_buffer_size",
    "join_buffer_size",
    "read_buffer_size",
    "read_rnd_buffer_size",
    "tmp_table_size",
    "max_heap_table_size",
    "innodb_buffer_pool_size",
    "innodb_log_file_size",
    "innodb_log_buffer_size",
    "innodb_flush_log_at_trx_commit",
    "innodb_file_per_table",
    "innodb_flush_method",
    "innodb_io_capacity",
    "innodb_read_io_threads",
    "innodb_write_io_threads",
    "innodb_lock_wait_timeout",
    "query_cache_size",
    "query_cache_type",
    "key_buffer_size",
    "myisam_sort_buffer_size",
    "log_error",
    "log-error",
    "slow_query_log",
    "slow-query-log",
    "long_query_time",
    "general_log",
    "general-log",
    "log_queries_not_using_indexes",
    "server-id",
    "server_id",
    "log_bin",
    "log-bin",
    "binlog_format",
    "expire_logs_days",
    "binlog_expire_logs_seconds",
    "sync_binlog",
    "relay_log",
    "read_only",
    "gtid_mode",
    "enforce_gtid_consistency",
    "wsrep_on",
    "wsrep_provider",
    "wsrep_cluster_address",
    "wsrep_cluster_name",
    "wsrep_node_address",
    "wsrep_sst_method",
    "ssl",
    "ssl-ca",
    "ssl-cert",
    "ssl-key",
    "require_secure_transport",
    "default_password_lifetime",
    "validate_password",
    "secure_file_priv",
    "local_infile",
    "sql_mode",
    "explicit_defaults_for_timestamp",
    "event_scheduler",
    "performance_schema",
    "open_files_limit",
    "back_log",
    "interactive_timeout",
    "wait_timeout",
    "net_read_timeout",
    "net_write_timeout",
    "connect_timeout",
    "delayed_insert_timeout",
    "lower_case_table_names",
    "skip-name-resolve",
    "skip_name_resolve",
    "skip-networking",
    "skip_networking",
];

fn is_group(t: &str) -> bool {
    t.starts_with('[')
        && t.ends_with(']')
        && GROUPS
            .iter()
            .any(|g| t[1..t.len() - 1].eq_ignore_ascii_case(g))
}

fn key_of_line(tr: &str) -> Option<(&str, Option<&str>)> {
    if let Some(eq) = tr.find('=') {
        let key = tr[..eq].trim_end();
        if !key.is_empty()
            && key.len() <= 60
            && key.chars().all(|c| {
                c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '-' | '.')
            })
        {
            return Some((key, Some(tr[eq + 1..].trim_start())));
        }
        return None;
    }
    // bare flag option
    if !tr.is_empty()
        && tr.len() <= 60
        && !tr.contains(char::is_whitespace)
        && tr
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '-' | '.'))
    {
        return Some((tr, None));
    }
    None
}

/// Whether the buffer looks like a MySQL/MariaDB option file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let groups = t.lines().filter(|l| is_group(l.trim())).count();
    if groups > 0 {
        return true;
    }
    let mut mysql_keys = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.starts_with('!') || tr.starts_with('#') || tr.starts_with(';') {
            continue;
        }
        if let Some((k, _)) = key_of_line(tr) {
            if KEYS.contains(&k) {
                mysql_keys += 1;
            }
        }
    }
    mysql_keys >= 3
}

impl Mysql {
    /// Count groups/options in a MySQL option file. Returns `None` when the
    /// input does not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            groups: 0,
            options: 0,
            known_keys: 0,
            flags: 0,
            includes: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with('#') || tr.starts_with(';') {
                c.comments += 1;
                continue;
            }
            if tr.starts_with("!include") {
                c.includes += 1;
                continue;
            }
            if is_group(tr) {
                c.groups += 1;
                continue;
            }
            let Some((key, v)) = key_of_line(tr) else {
                continue;
            };
            c.options += 1;
            if v.is_none() {
                c.flags += 1;
            }
            let norm = key.replace('-', "_");
            if KEYS.contains(&key) || KEYS.contains(&norm.as_str()) {
                c.known_keys += 1;
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
        let b = br#"# my.cnf
[client]
port = 3306
socket = /var/run/mysql.sock

[mysqld_safe]
log_error = /var/log/mysql.err

[mysqld]
user = mysql
skip-networking
bind-address = 127.0.0.1
max_connections = 250
innodb_buffer_pool_size = 512M
innodb_file_per_table = ON
slow_query_log = 1
server_id = 1
log_bin = mysql-bin
binlog_format = ROW
sql_mode = STRICT_TRANS_TABLES
secure_file_priv = /var/lib/mysql-files
!includedir /etc/mysql/conf.d/
"#;
        assert!(detect(b));
        let c = Mysql::parse(b).unwrap();
        assert_eq!(c.groups, 3);
        assert_eq!(c.options, 15);
        assert_eq!(c.flags, 1);
        assert_eq!(c.known_keys, 15);
        assert_eq!(c.includes, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other_ini() {
        assert!(!detect(b"[section]\nkey = value\n[other]\nx = 1"));
        assert!(!detect(b"no mysql keys here\njust some text\n"));
    }
}
