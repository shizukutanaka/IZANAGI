//! Tarantool `box.cfg{...}`(Lua 設定ブロック)の検出と構造カウント。
//!
//! ```
//! let c = izanagi_kit::tarantool::parse(b"box.cfg{listen = 3301, memtx_memory = 134217728, wal_mode = \"write\"}\n").unwrap();
//! assert_eq!(c.params, 3);
//! assert_eq!(c.storage, 1);
//! assert_eq!(c.wal, 1);
//! assert_eq!(c.net, 1);
//! ```

/// レプリケーション・選出系キー。
const REPL: &[&str] = &[
    "replication",
    "replication_synchro_quorum",
    "replication_synchro_timeout",
    "replication_connect_quorum",
    "replication_connect_timeout",
    "replication_timeout",
    "replication_sync_timeout",
    "replication_sync_lag",
    "replication_skip_conflict",
    "replication_anon",
    "replication_autoexpel",
    "replicaset_uuid",
    "instance_uuid",
    "cluster_uuid",
    "election_mode",
    "election_timeout",
    "election_fencing_mode",
    "node_name",
    "sharding",
    "bootstrap_leader",
    "bootstrap_strategy",
];
/// WAL・スナップショット系キー。
const WAL: &[&str] = &[
    "wal_mode",
    "wal_max_size",
    "wal_dir_rescan_delay",
    "wal_cleanup_delay",
    "wal_queue_max_size",
    "checkpoint_interval",
    "checkpoint_wal_threshold",
    "checkpoint_count",
    "snap_io_rate_limit",
    "force_recovery",
];
/// メモリ・ストレージ系キー。
const STORAGE: &[&str] = &[
    "memtx_memory",
    "memtx_max_tuple_size",
    "memtx_min_tuple_size",
    "memtx_allocator",
    "memtx_use_mvcc_engine",
    "memtx_dir",
    "memtx_sort_threads",
    "vinyl_memory",
    "vinyl_cache",
    "vinyl_max_tuple_size",
    "vinyl_read_threads",
    "vinyl_write_threads",
    "vinyl_dir",
    "vinyl_bloom_fpr",
    "vinyl_page_size",
    "vinyl_range_size",
    "vinyl_run_size_ratio",
    "vinyl_run_count_per_level",
    "vinyl_timeout",
    "vinyl_defer_delete",
    "snap_dir",
    "work_dir",
    "readahead",
    "slab_alloc_factor",
    "slab_alloc_arena",
];
/// ネットワーク系キー。
const NET: &[&str] = &[
    "listen",
    "iproto",
    "read_only",
    "hot_standby",
    "worker_pool_threads",
    "net_msg_max",
    "net_raw_recv_max",
    "io_collect_interval",
    "net_conn_timeout",
    "net_msg_queue_max",
];
/// 実行・ログ・監査系キー。
const SYS: &[&str] = &[
    "pid_file",
    "background",
    "daemonize",
    "work_dir",
    "username",
    "user",
    "coredump",
    "custom_proc_title",
    "log",
    "log_level",
    "log_format",
    "log_nonblock",
    "log_modules",
    "too_long_threshold",
    "memtx_shards",
    "feedback_enabled",
    "feedback_host",
    "feedback_interval",
    "feedback_send_metrics",
    "crash_report_enabled",
    "crash_report_send",
    "crash_report_receiver",
    "metrics",
    "sql_cache_size",
    "sql_full_scan",
    "sql_full_count",
    "sql_defer_foreign_keys",
    "audit_log",
    "audit_format",
    "audit_filter",
    "audit_nonblock",
    "security_audit_auth",
    "startup_strategy",
    "instance_state",
    "config",
    "flyway",
    "lua_opts",
    "lua_pretty_print",
    "lua_output_repl",
    "console_socket",
    "replication_bootstrap",
    "readahead",
    "persistence_slave_id",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `key = value` パラメータの総数(box.cfg{ } 内・外を含む)。
    pub params: usize,
    /// レプリケーション・選出系キー数。
    pub replication: usize,
    /// WAL・スナップショット系キー数。
    pub wal: usize,
    /// メモリ・ストレージ系キー数。
    pub storage: usize,
    /// ネットワーク系キー数。
    pub net: usize,
    /// 実行・ログ・監査系キー数。
    pub sys: usize,
    /// その他行数。
    pub misc: usize,
}
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// `b` が Tarantool box.cfg かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let text = match std::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let text = strip_bom(text);
    // `box.cfg` の直後に `{`/`(` が来る呼び出し形だけを見る
    // — Lua コメント(`--`)や文字列値中の同名語では検出しない。
    text.lines().any(|l| {
        let t = l.trim_start();
        if t.starts_with("--") {
            return false;
        }
        let Some(i) = l.find("box.cfg") else {
            return false;
        };
        l[i + "box.cfg".len()..]
            .trim_start()
            .starts_with(['{', '('])
    })
}

/// `b` を Tarantool 設定として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let text = strip_bom(text);
    if !text.contains("box.cfg") {
        return None;
    }
    let mut c = Counts {
        params: 0,
        replication: 0,
        wal: 0,
        storage: 0,
        net: 0,
        sys: 0,
        misc: 0,
    };
    for raw in text.lines() {
        let t = raw.trim();
        if t.is_empty() || t.starts_with("--") {
            continue;
        }
        let bytes = t.as_bytes();
        for i in 0..bytes.len() {
            if bytes[i] != b'=' || (i + 1 < bytes.len() && bytes[i + 1] == b'=') {
                continue;
            }
            let mut j = i;
            while j > 0 && bytes[j - 1].is_ascii_whitespace() {
                j -= 1;
            }
            let end = j;
            while j > 0 && (bytes[j - 1].is_ascii_alphanumeric() || bytes[j - 1] == b'_') {
                j -= 1;
            }
            let key = &t[j..end];
            if key.is_empty() {
                continue;
            }
            c.params += 1;
            if REPL.contains(&key) {
                c.replication += 1;
            } else if WAL.contains(&key) {
                c.wal += 1;
            } else if STORAGE.contains(&key) {
                c.storage += 1;
            } else if NET.contains(&key) {
                c.net += 1;
            } else if SYS.contains(&key) {
                c.sys += 1;
            } else {
                c.misc += 1;
            }
        }
    }
    (c.params >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tarantool() {
        let cfg = br#"box.cfg{
    listen = 3301,
    memtx_memory = 134217728,
    wal_mode = "write",
    replication = {"tcp://r1:3301"},
    checkpoint_interval = 3600,
    iproto = {
        threads = 2
    },
    pid_file = "tarantool.pid",
    custom_proc_title = "tt",
}
"#;
        let c = parse(cfg).unwrap();
        assert_eq!(c.params, 9);
        assert_eq!(c.replication, 1);
        assert_eq!(c.wal, 2);
        assert_eq!(c.storage, 1);
        assert_eq!(c.net, 2);
        assert_eq!(c.sys, 2);
        assert_eq!(c.misc, 1);
    }

    #[test]
    fn not_tarantool() {
        assert!(parse(b"x = 1\n").is_none());
    }

    #[test]
    fn box_cfg_in_comment_or_string_does_not_detect() {
        // `box.cfg` inside a Lua comment must not trigger.
        assert!(!detect(b"-- box.cfg{listen = 3301}\nx = 1\n"));
        // `box.cfg` as a bare word inside a string must not trigger.
        assert!(!detect(b"msg = \"run box.cfg first\"\nx = 1\n"));
        // `box.cfg(...)` call form is accepted.
        assert!(detect(b"box.cfg({listen = 3301})\n"));
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
