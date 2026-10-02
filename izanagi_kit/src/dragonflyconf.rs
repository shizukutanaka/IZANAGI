//! Dragonfly flagfile(`--flag value` / `--flag=value` 行)の検出と構造カウント。
//!
//! ```
//! let c = izanagi_kit::dragonflyconf::parse(b"--port 6379\n--dir /data\n--maxmemory 4gb\n--cluster_mode=yes\n--proactor_threads 4\n").unwrap();
//! assert_eq!(c.flags, 5);
//! assert_eq!(c.core, 3);
//! assert_eq!(c.cluster, 1);
//! ```

/// 基本・永続化系フラグ。
const CORE: &[&str] = &[
    "port",
    "bind",
    "dir",
    "dbfilename",
    "dbnum",
    "maxmemory",
    "hz",
    "save_schedule",
    "snapshot_cron",
    "cache_mode",
    "memcached_port",
    "keys_output_limit",
    "object_spill_threshold",
    "eviction_budget",
    "spill_file_prefix",
    "tiered_prefix",
    "tiered_offload_threshold",
    "tiered_storage_write_depth",
    "serialization_max_chunk_size",
];
/// クラスタ・レプリケーション系フラグ。
const CLUSTER: &[&str] = &[
    "cluster_mode",
    "cluster_announce_ip",
    "cluster_announce_port",
    "announce_ip",
    "announce_port",
    "admin_port",
    "admin_bind",
    "replicaof",
    "masterauth",
    "replica_priority",
    "replication_timeout",
    "migration_lock_timeout_ms",
    "cluster_config",
    "enable_multi_slot_update",
    "no_tls_on_admin_port",
    "emulated_cluster",
];
/// TLS・認証系フラグ。
const TLS: &[&str] = &[
    "tls",
    "tls_key_file",
    "tls_cert_file",
    "tls_ca_cert_file",
    "tls_verify_client_cert",
    "tls_sni",
    "requirepass",
    "aclfile",
    "keys_output_limit",
    "tls_replication",
];
/// 実行・ログ系フラグ。
const RUN: &[&str] = &[
    "proactor_threads",
    "conn_use_incoming_cpu",
    "logtostderr",
    "alsologtostderr",
    "stderrthreshold",
    "vmodule",
    "log_dir",
    "minloglevel",
    "version_check",
    "pidfile",
    "daemonize",
    "logfile",
    "epoll_file_threads",
    "channel_size",
    "hz",
    "warn_limit",
    "force_epoll",
    "managed_service",
    "s3_endpoint",
    "s3_ec2_metadata",
    "s3_use_https",
    "s3_sign_payload",
    "skip_locked_nodes_on_flush",
    "lock_on_hashtags",
    "multi_exec_mode",
    "version_string",
    "file_contains",
    "hz",
    "proactor_pool_size",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// フラグ行の総数(`--name` / `--name=v` / `name v`)。
    pub flags: usize,
    /// 基本・永続化系フラグ数。
    pub core: usize,
    /// クラスタ・レプリケーション系フラグ数。
    pub cluster: usize,
    /// TLS・認証系フラグ数。
    pub tls: usize,
    /// 実行・ログ系フラグ数。
    pub runtime: usize,
    /// その他行数。
    pub misc: usize,
}

/// `b` が Dragonfly flagfile かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.flags >= 2 && c.misc == 0)
}

/// `b` を Dragonfly flagfile として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        flags: 0,
        core: 0,
        cluster: 0,
        tls: 0,
        runtime: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let body = t.strip_prefix("--").unwrap_or(t);
        let name = body.split([' ', '\t', '=']).next().unwrap_or("");
        if name.is_empty() || !name.chars().next().is_some_and(|x| x.is_ascii_alphabetic()) {
            c.misc += 1;
            continue;
        }
        c.flags += 1;
        if CORE.contains(&name) {
            c.core += 1;
        } else if CLUSTER.contains(&name) {
            c.cluster += 1;
        } else if TLS.contains(&name) {
            c.tls += 1;
        } else if RUN.contains(&name) {
            c.runtime += 1;
        } else {
            c.misc += 1;
        }
    }
    (c.flags >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dragonfly() {
        let cfg = b"--port 6379\n--bind localhost\n--dir /data\n--dbfilename dump.rdb\n--maxmemory 4gb\n--hz 100\n--cluster_mode=yes\n--admin_port 9999\n--tls_key_file server.key\n--proactor_threads 4\n--logtostderr\n# comment\n";
        let c = parse(cfg).unwrap();
        assert_eq!(c.flags, 11);
        assert_eq!(c.core, 6);
        assert_eq!(c.cluster, 2);
        assert_eq!(c.tls, 1);
        assert_eq!(c.runtime, 2);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn not_dragonfly() {
        assert!(!detect(b"key = value\n"));
    }
}
