//! ClickHouse server `config.xml`/`users.xml` parser.
//!
//! Detects the `<clickhouse>` (or legacy `<yandex>`) root element and the
//! characteristic child elements (`<http_port>`, `<tcp_port>`,
//! `<listen_host>`, `<max_connections>`, `<logger>`,
//! `<mark_cache_size>`/`<mark_cache>`, `<mlock_executable>`,
//! `<remote_servers>`/`<shard>`/`<replica>`, `<zookeeper>`, `<users>`/
//! `<profiles>`/`<quotas>`, `<interserver_http*>`,
//! `<distributed_ddl>`, `<dictionaries_config>` …), and counts elements,
//! port/host settings, cluster elements, and comments.
//!
//! ```
//! let b = br#"<clickhouse>
//!     <logger>
//!         <level>trace</level>
//!     </logger>
//!     <http_port>8123</http_port>
//!     <tcp_port>9000</tcp_port>
//!     <listen_host>::1</listen_host>
//!     <max_connections>4096</max_connections>
//!     <users_config>users.xml</users_config>
//! </clickhouse>"#;
//! assert!(izanagi_kit::clickhouse::detect(b));
//! let c = izanagi_kit::clickhouse::Clickhouse::parse(b).unwrap();
//! assert_eq!(c.elements, 8);
//! assert_eq!(c.root, "clickhouse");
//! ```

/// Parsed ClickHouse config summary.
#[derive(Debug, Clone)]
pub struct Clickhouse {
    /// Root element name (`clickhouse` or `yandex`).
    pub root: String,
    /// `<elem>` open tags seen.
    pub elements: usize,
    /// Recognized config element names.
    pub known_elements: usize,
    /// `*_port`/`listen_host`/`host`/`path`/`level` settings.
    pub settings: usize,
    /// Clustering elements (`shard`, `replica`, `zookeeper`/`keeper_server`, `remote_servers`).
    pub cluster_elements: usize,
    /// `<!-- -->` comment lines.
    pub comments: usize,
}

/// Well-known element names in config.xml/users.xml.
const ELEMS: &[&str] = &[
    "clickhouse",
    "yandex",
    "logger",
    "level",
    "log",
    "errorlog",
    "size",
    "count",
    "console",
    "send_level",
    "syslog",
    "address",
    "hostname",
    "facility",
    "format",
    "http_port",
    "tcp_port",
    "tcp_port_secure",
    "interserver_http_port",
    "interserver_http_host",
    "interserver_https_port",
    "interserver_https_host",
    "http_options",
    "mysql_port",
    "postgresql_port",
    "grpc_port",
    "prometheus_port",
    "tcp_with_proxy_port",
    "ssl",
    "http",
    "listen_host",
    "listen_try",
    "listen_reuse_port",
    "listen_backlog",
    "max_connections",
    "keep_alive_timeout",
    "max_concurrent_queries",
    "max_server_memory_usage",
    "max_thread_pool_size",
    "mark_cache_size",
    "mark_cache",
    "uncompressed_cache_size",
    "uncompressed_cache",
    "mmap_cache_size",
    "mlock_executable",
    "cache_size_in_cells",
    "path",
    "tmp_path",
    "user_files_path",
    "users_config",
    "default_profile",
    "default_database",
    "timezone",
    "mlock_executable",
    "remap_executable",
    "umask",
    "user_dirs",
    "users_without_passwords_allowed",
    "allow_databases",
    "dictionaries_config",
    "user_defined_executable_functions_config",
    "encryption_codecs",
    "session_log",
    "part_log",
    "trace_log",
    "query_log",
    "query_thread_log",
    "query_views_log",
    "text_log",
    "metric_log",
    "asynchronous_metric_log",
    "opentelemetry_span_log",
    "error_log",
    "crash_log",
    "session_log",
    "zookeeper_log",
    "processors_profile_log",
    "async_insert_log",
    "backup_log",
    "top_level_domains_path",
    "public_hostname",
    "remote_servers",
    "default",
    "shard",
    "replica",
    "host",
    "port",
    "user",
    "password",
    "weight",
    "internal_replication",
    "zookeeper",
    "keeper_server",
    "node",
    "index",
    "session_timeout_ms",
    "operation_timeout_ms",
    "root",
    "identity",
    "optional",
    "distributed_ddl",
    "task",
    "cleanup_delay_period",
    "max_tasks_in_queue",
    "path_in_zookeeper",
    "users",
    "profiles",
    "quotas",
    "default",
    "readonly",
    "access_management",
    "named_collection_control",
    "show_named_collections",
    "show_named_collections_secrets",
    "allow_ddl",
    "allow_introspection_functions",
    "max_memory_usage",
    "max_execution_time",
    "max_result_rows",
    "max_result_bytes",
    "max_rows_to_read",
    "max_bytes_to_read",
    "use_uncompressed_cache",
    "load_balancing",
    "networks",
    "ip",
    "host_regexp",
    "http_methods",
    "grpc",
    "builtin_dictionaries_reload_interval",
    "max_session_timeout",
    "default_session_timeout",
    "send_progress_in_http_headers",
    "http_headers_response_time_ns",
    "disable_internal_dns_cache",
    "dns_cache_update_period",
    "dns_max_consecutive_failures",
    "allow_proxy",
    "total_memory_profiler_step",
    "total_memory_tracker_sample_probability",
    "graphite",
    "graphite_rollup",
    "interval",
    "timeout",
    "patterns",
    "pattern",
    "regexp",
    "function",
    "retention",
    "age",
    "precision",
    "macros",
    "http_options",
    "shutdown_wait_unfinished_queries",
    "shutdown_wait_unfinished",
    "compiled_expression_cache_size",
    "global_profiler_real_time_period_ns",
    "compressed_protocol",
    "temporary_directories_policy",
    "temporary_data_in_cache",
    "user_group_path",
    "allow_feature_tier",
    "asynchronous_metrics",
    "catboost_dynamic_library_path",
    "url_scheme_mappers",
    "send_stacktrace",
    "custom_settings_prefixes",
    "core_dump",
    "size_limit",
    "path_to_merge",
    "merge_tree",
    "replicated",
    "in_memory",
    "allow_remote_fs_zero_copy_replication",
    "storage_policy",
    "disks",
    "s3",
    "endpoint",
    "access_key_id",
    "secret_access_key",
    "volume",
    "volumes",
    "main",
    "disk",
    "encryption",
    "backups",
    "allowed_path",
    "local_database",
    "database_atomic",
    "replica_path",
    "replica_name",
    "secure",
    "openSSL",
    "server",
    "client",
    "verificationMode",
    "caConfig",
    "certificateFile",
    "privateKeyFile",
    "dhParamsFile",
    "keyPath",
    "certificate",
    "passPhrase",
    "include_from",
    "incl",
];

/// Cluster-related element names.
const CLUSTER: &[&str] = &[
    "remote_servers",
    "shard",
    "replica",
    "zookeeper",
    "keeper_server",
    "node",
    "internal_replication",
    "replica_path",
    "replica_name",
    "distributed_ddl",
];

fn elem_name(tr: &str) -> Option<(&str, bool)> {
    // returns (name, is_closing)
    let t = tr.strip_prefix('<')?;
    if t.starts_with('!') || t.starts_with('?') {
        return None;
    }
    if let Some(rest) = t.strip_prefix('/') {
        let end = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.')))
            .unwrap_or(rest.len());
        return Some((&rest[..end], true));
    }
    let end = t
        .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.')))
        .unwrap_or(t.len());
    Some((&t[..end], false))
}

/// Whether the buffer looks like a ClickHouse config file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    if t.contains("<clickhouse>") || t.contains("<clickhouse ") || t.contains("<yandex") {
        return true;
    }
    // users.xml also uses <clickhouse>; fall back on element signature.
    let hits = [
        "http_port",
        "tcp_port",
        "remote_servers",
        "zookeeper",
        "users_config",
    ]
    .iter()
    .filter(|e| t.contains(&format!("<{e}>")) || t.contains(&format!("<{e} ")))
    .count();
    hits >= 2
}

impl Clickhouse {
    /// Count elements in a ClickHouse config. Returns `None` when the input
    /// does not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            root: String::new(),
            elements: 0,
            known_elements: 0,
            settings: 0,
            cluster_elements: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with("<!--") {
                c.comments += 1;
                continue;
            }
            // A line may carry several tags (`<a><b>v</b></a>`) — scan each.
            let mut rest = tr;
            while let Some(lt) = rest.find('<') {
                rest = &rest[lt..];
                let Some((name, closing)) = elem_name(rest) else {
                    rest = &rest[1..];
                    continue;
                };
                rest = &rest[1..];
                if name.is_empty() {
                    continue;
                }
                if c.root.is_empty() && !closing && (name == "clickhouse" || name == "yandex") {
                    c.root = name.to_string();
                }
                if !closing {
                    c.elements += 1;
                    if ELEMS.contains(&name) {
                        c.known_elements += 1;
                    }
                    if name.ends_with("_port")
                        || matches!(
                            name,
                            "listen_host" | "host" | "path" | "level" | "hostname" | "port"
                        )
                    {
                        c.settings += 1;
                    }
                    if CLUSTER.contains(&name) {
                        c.cluster_elements += 1;
                    }
                }
            }
        }
        if c.root.is_empty() {
            c.root = "clickhouse".to_string();
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = br#"<?xml version="1.0"?>
<clickhouse>
    <!-- logging -->
    <logger>
        <level>trace</level>
        <log>/var/log/clickhouse-server/clickhouse-server.log</log>
        <errorlog>/var/log/clickhouse-server/clickhouse-server.err.log</errorlog>
    </logger>
    <http_port>8123</http_port>
    <tcp_port>9000</tcp_port>
    <interserver_http_port>9009</interserver_http_port>
    <listen_host>127.0.0.1</listen_host>
    <max_connections>4096</max_connections>
    <keep_alive_timeout>3</keep_alive_timeout>
    <mark_cache_size>5368709120</mark_cache_size>
    <mlock_executable>false</mlock_executable>
    <path>/var/lib/clickhouse/</path>
    <tmp_path>/var/lib/clickhouse/tmp/</tmp_path>
    <remote_servers>
        <default>
            <shard>
                <replica>
                    <host>localhost</host>
                    <port>9000</port>
                </replica>
            </shard>
        </default>
    </remote_servers>
    <zookeeper>
        <node index="1">
            <host>localhost</host>
            <port>2181</port>
        </node>
    </zookeeper>
    <users_config>users.xml</users_config>
    <default_profile>default</default_profile>
    <default_database>default</default_database>
</clickhouse>"#;
        assert!(detect(b));
        let c = Clickhouse::parse(b).unwrap();
        assert_eq!(c.root, "clickhouse");
        assert_eq!(c.elements, 28);
        assert_eq!(c.known_elements, 28);
        assert_eq!(c.settings, 10);
        assert_eq!(c.cluster_elements, 5);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn detects_users_xml() {
        let b = br#"<clickhouse>
    <profiles>
        <default>
            <max_memory_usage>10000000000</max_memory_usage>
        </default>
    </profiles>
    <users>
        <default>
            <password></password>
            <networks><ip>::/0</ip></networks>
        </default>
    </users>
</clickhouse>"#;
        assert!(detect(b));
        let c = Clickhouse::parse(b).unwrap();
        assert_eq!(c.elements, 9);
    }

    #[test]
    fn rejects_other_xml() {
        assert!(!detect(br#"<html><body><p>hi</p></body></html>"#));
        assert!(!detect(b"not xml at all\nhttp_port 8123\n"));
    }
}
