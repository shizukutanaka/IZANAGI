//! Apache Cassandra `cassandra.yaml` parser.
//!
//! Detects the characteristic flat key namespace (`cluster_name`,
//! `listen_address`, `seed_provider`, `commitlog_*`, `num_tokens`,
//! `endpoint_snitch`, `rpc_address`, `native_transport_port`, `data_file_directories`,
//! `saved_caches_directory`, `hinted_handoff_*`, `authorizer`, `authenticator`,
//! `partitioner`, `concurrent_*`, `memtable_*`, `row_cache_*`, `key_cache_*`,
//! `internode_*`) plus `seed_provider:` `- class_name` / `- seeds` list items,
//! and counts keys, snake_case keys, bool values, and `#` comments.
//!
//! ```
//! let b = br#"cluster_name: 'Test Cluster'
//! num_tokens: 16
//! seed_provider:
//!   - class_name: org.apache.cassandra.locator.SimpleSeedProvider
//!     parameters:
//!       - seeds: "127.0.0.1"
//! listen_address: localhost
//! endpoint_snitch: SimpleSnitch"#;
//! assert!(izanagi_kit::cassandra::detect(b));
//! let c = izanagi_kit::cassandra::Cassandra::parse(b).unwrap();
//! assert_eq!(c.known_keys, 8);
//! ```

/// Parsed cassandra.yaml summary.
#[derive(Debug, Clone)]
pub struct Cassandra {
    /// `key:`/`key: value` lines.
    pub keys: usize,
    /// Recognized cassandra.yaml keys.
    pub known_keys: usize,
    /// `- ` list items (seed parameters, data directories).
    pub list_items: usize,
    /// `true`/`false`/`enabled`/`disabled` values.
    pub bool_values: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Well-known cassandra.yaml keys.
const KEYS: &[&str] = &[
    "cluster_name",
    "listen_address",
    "broadcast_address",
    "listen_interface",
    "listen_on_broadcast_address",
    "rpc_address",
    "broadcast_rpc_address",
    "rpc_interface",
    "native_transport_port",
    "native_transport_port_ssl",
    "storage_port",
    "ssl_storage_port",
    "seed_provider",
    "class_name",
    "parameters",
    "seeds",
    "data_file_directories",
    "saved_caches_directory",
    "commitlog_directory",
    "cdc_raw_directory",
    "hints_directory",
    "commitlog_sync",
    "commitlog_sync_period_in_ms",
    "commitlog_sync_batch_window_in_ms",
    "commitlog_segment_size_in_mb",
    "commitlog_compression",
    "commitlog_max_compression_buffers_in_pool",
    "num_tokens",
    "allocate_tokens_for_keyspace",
    "allocate_tokens_for_local_replication_factor",
    "initial_token",
    "endpoint_snitch",
    "dynamic_snitch",
    "dynamic_snitch_update_interval_in_ms",
    "dynamic_snitch_reset_interval_in_ms",
    "dynamic_snitch_badness_threshold",
    "hinted_handoff_enabled",
    "hinted_handoff_disabled_datacenters",
    "hinted_handoff_throttle_in_kb",
    "max_hint_window_in_ms",
    "hints_flush_period_in_ms",
    "max_hints_file_size_in_mb",
    "hints_compression",
    "authorizer",
    "authenticator",
    "role_manager",
    "roles_validity_in_ms",
    "permissions_validity_in_ms",
    "credentials_validity_in_ms",
    "partitioner",
    "disk_failure_policy",
    "commit_failure_policy",
    "key_cache_size_in_mb",
    "key_cache_save_period",
    "key_cache_keys_to_save",
    "row_cache_class_name",
    "row_cache_size_in_mb",
    "row_cache_save_period",
    "row_cache_keys_to_save",
    "counter_cache_size_in_mb",
    "counter_cache_save_period",
    "counter_cache_keys_to_save",
    "memtable_heap_space_in_mb",
    "memtable_offheap_space_in_mb",
    "memtable_flush_writers",
    "memtable_cleanup_threshold",
    "memtable_allocation_type",
    "column_index_size_in_kb",
    "index_summary_capacity_in_mb",
    "index_summary_resize_interval_in_minutes",
    "concurrent_reads",
    "concurrent_writes",
    "concurrent_counter_writes",
    "concurrent_materialized_view_writes",
    "concurrent_compactors",
    "compaction_throughput_mb_per_sec",
    "compaction_large_partition_warning_threshold_mb",
    "sstable_preemptive_open_interval_in_mb",
    "stream_throughput_outbound_megabits_per_sec",
    "inter_dc_stream_throughput_outbound_megabits_per_sec",
    "trickle_fsync",
    "trickle_fsync_interval_in_kb",
    "batchlog_replay_throttle_in_kb",
    "read_request_timeout_in_ms",
    "range_request_timeout_in_ms",
    "write_request_timeout_in_ms",
    "counter_write_request_timeout_in_ms",
    "cas_contention_timeout_in_ms",
    "truncate_request_timeout_in_ms",
    "request_timeout_in_ms",
    "slow_query_log_timeout_in_ms",
    "cross_node_timeout",
    "streaming_keep_alive_period_in_secs",
    "phi_convict_threshold",
    "server_encryption_options",
    "client_encryption_options",
    "internode_encryption",
    "internode_compression",
    "inter_dc_tcp_nodelay",
    "tracing",
    "native_transport_max_threads",
    "native_transport_max_frame_size_in_mb",
    "native_transport_max_concurrent_connections",
    "native_transport_max_concurrent_connections_per_ip",
    "native_transport_flush_in_batches_legacy",
    "native_transport_max_negotiable_protocol_version",
    "native_transport_rate_limiting",
    "incremental_backups",
    "snapshot_before_compaction",
    "auto_snapshot",
    "tombstone_warn_threshold",
    "tombstone_failure_threshold",
    "column_index_cache_size_in_kb",
    "batch_size_warn_threshold_in_kb",
    "batch_size_fail_threshold_in_kb",
    "unlogged_batch_across_partitions_warn_threshold",
    "cdc_enabled",
    "cdc_total_space_in_mb",
    "cdc_free_space_check_interval_ms",
    "back_pressure_enabled",
    "back_pressure_strategy",
    "repair_session_max_tree_depth",
    "consistent_rangemovement",
    "enable_user_defined_functions",
    "enable_scripted_user_defined_functions",
    "enable_materialized_views",
    "enable_sasi_indexes",
    "enable_transient_replication",
    "enable_drop_compact_storage",
    "max_value_size_in_mb",
    "otc_coalescing_strategy",
    "otc_coalescing_window_us",
    "otc_coalescing_sufficient_coalesced_messages",
    "windows_timer_interval",
    "prepared_statements_cache_size_mb",
    "thrift_prepared_statements_cache_size_mb",
    "file_cache_size_in_mb",
    "buffer_pool_use_heap_if_exhausted",
    "disk_optimization_strategy",
    "disk_optimization_estimate_percentile",
    "disk_optimization_page_cross_chance",
    "gc_warn_threshold_in_ms",
    "gc_log_threshold_in_ms",
    "internode_application_send_queue_capacity_in_bytes",
    "internode_application_send_queue_reserve_endpoint_capacity_in_bytes",
    "internode_application_send_queue_reserve_global_capacity_in_bytes",
    "internode_application_receive_queue_capacity_in_bytes",
    "internode_application_receive_queue_reserve_endpoint_capacity_in_bytes",
    "internode_application_receive_queue_reserve_global_capacity_in_bytes",
    "full_query_logging_options",
    "corrupted_tombstone_strategy",
    "diagnostic_events_enabled",
    "repaired_data_tracking_for_range_reads_enabled",
    "repaired_data_tracking_for_partition_reads_enabled",
    "report_unconfirmed_repaired_data_mismatches",
    "audit_logging_options",
    "startup_checks",
    "ideal_consistency_level",
    "automatic_sstable_upgrade",
    "max_concurrent_automatic_sstable_upgrades",
    "streaming_entire_sstables",
    "stream_state_read",
    "netty_streaming_request_timeout",
    "native_transport_idle_timeout",
    "transparent_data_encryption_options",
    "table_metric_histogram_initial_size",
    "user_defined_functions_enabled",
    "scripted_user_defined_functions_enabled",
];

fn key_of_line(tr: &str) -> Option<(&str, &str)> {
    let colon = tr.find(':')?;
    if colon == 0 {
        return None;
    }
    let key = tr[..colon].trim_end();
    if !key
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '.'))
    {
        return None;
    }
    Some((key, tr[colon + 1..].trim_start()))
}

/// Whether the buffer looks like cassandra.yaml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = t
        .lines()
        .filter(|l| {
            let tr = l.trim();
            if tr.starts_with('#') {
                return false;
            }
            if let Some(st) = tr.strip_prefix('-') {
                let tr2 = st.trim_start();
                if let Some((k, _)) = key_of_line(tr2) {
                    return KEYS.contains(&k);
                }
                return false;
            }
            key_of_line(tr).is_some_and(|(k, _)| KEYS.contains(&k))
        })
        .count();
    hits >= 3 || t.contains("seed_provider") || t.contains("endpoint_snitch")
}

impl Cassandra {
    /// Count keys/list items in cassandra.yaml. Returns `None` when the
    /// input does not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            known_keys: 0,
            list_items: 0,
            bool_values: 0,
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
            if let Some(st) = tr.strip_prefix('-') {
                c.list_items += 1;
                let inner = st.trim_start();
                if let Some((k, v)) = key_of_line(inner) {
                    c.keys += 1;
                    if KEYS.contains(&k) {
                        c.known_keys += 1;
                    }
                    if matches!(v, "true" | "false" | "enabled" | "disabled") {
                        c.bool_values += 1;
                    }
                }
                continue;
            }
            let Some((key, v)) = key_of_line(tr) else {
                continue;
            };
            c.keys += 1;
            if KEYS.contains(&key) {
                c.known_keys += 1;
            }
            if matches!(v, "true" | "false" | "enabled" | "disabled" | "on" | "off") {
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
        let b = br#"# Cassandra storage config YAML
cluster_name: 'My Cluster'
num_tokens: 256
commitlog_sync: periodic
commitlog_sync_period_in_ms: 10000
commitlog_segment_size_in_mb: 32
seed_provider:
  - class_name: org.apache.cassandra.locator.SimpleSeedProvider
    parameters:
      - seeds: "127.0.0.1,127.0.0.2"
listen_address: localhost
rpc_address: localhost
endpoint_snitch: SimpleSnitch
partitioner: org.apache.cassandra.dht.Murmur3Partitioner
auto_snapshot: true
key_cache_size_in_mb: 100
concurrent_reads: 32
concurrent_writes: 32
hinted_handoff_enabled: false
"#;
        assert!(detect(b));
        let c = Cassandra::parse(b).unwrap();
        assert_eq!(c.keys, 18);
        assert_eq!(c.known_keys, 18);
        assert_eq!(c.list_items, 2);
        assert_eq!(c.bool_values, 2);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_generic_yaml() {
        assert!(!detect(b"foo: bar\nbaz: qux\nquux:\n  - a\n"));
    }
}
