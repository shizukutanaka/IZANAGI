//! pmacct コレクタ設定ファイル (`pmacctd.conf`, `nfacctd.conf`, `sfacctd.conf` 等) パーサ。
//!
//! `key: value` コロン区切り形式。`daemonize`/`syslog`/`interface`/`plugins`/
//! `aggregate`/`nfacctd_port`/`sql_*`/`kafka_*`/`imt_*` 等キーを計数する。
//!
//! ```
//! use izanagi_kit::pmacctconf;
//! let conf = b"daemonize: true\nsyslog: daemon\ninterface: eth0\nplugins: print, imt[imta]\naggregate: src_host,dst_host\n";
//! assert!(pmacctconf::detect(conf));
//! let c = pmacctconf::parse(conf).unwrap();
//! assert_eq!(c.entries, 5);
//! assert_eq!(c.known_keys, 5);
//! ```

/// 判定結果の計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `key: value` 行数。
    pub entries: usize,
    /// 既知キー行数。
    pub known_keys: usize,
    /// `#` コメント行数。
    pub comments: usize,
}

const KNOWN_KEYS: &[&str] = &[
    "daemonize",
    "syslog",
    "interface",
    "pcap_filter",
    "plugins",
    "aggregate",
    "aggregate_primitives",
    "aggregate_methods",
    "snaplen",
    "pmacctd_as",
    "pmacctd_net",
    "pmacctd_port",
    "nfacctd_as",
    "nfacctd_net",
    "nfacctd_port",
    "sfacctd_as",
    "sfacctd_net",
    "sfacctd_port",
    "uacctd_pidfile",
    "uacctd_group",
    "pmtelemetryd_port",
    "pmtelemetry_port",
    "bgp_daemon",
    "bmp_daemon",
    "igp_daemon",
    "use_ip_next_hop",
    "tag",
    "classifiers_path",
    "maps",
    "imt_path",
    "imt_client_pth",
    "imt_daemon",
    "imt_mem_pool",
    "pre_tag_map",
    "preprocess",
    "pcap_interfaces_map",
    "files_umask",
    "files_compression",
    "files_umask",
    "sql_host",
    "sql_db",
    "sql_user",
    "sql_passwd",
    "sql_table",
    "sql_table_version",
    "sql_multi_values",
    "sql_dont_try_update",
    "sql_optimize_clauses",
    "sql_refresh_time",
    "sql_purge_interval",
    "sql_max_writers",
    "sql_delimiters",
    "sql_history",
    "sql_history_roundoff",
    "sql_startup_delay",
    "sql_trigger_exec",
    "sql_trigger_time",
    "sql_recovery_logfile",
    "sql_cache_entries",
    "sql_agent_map",
    "sql_number_of_workers",
    "sql_locking_style",
    "sql_restart_on_error",
    "sql_table_schema",
    "sql_table_type",
    "kafka_brokers",
    "kafka_topic",
    "kafka_config_file",
    "rabbitmq_user",
    "rabbitmq_pwd",
    "rabbitmq_host",
    "rabbitmq_exchange",
    "rabbitmq_routing_key",
    "rabbitmq_broker_port",
    "mongo_host",
    "mongo_user",
    "mongo_passwd",
    "mongo_db",
    "mongo_table",
    "mongo_insert_batch",
    "mongo_refresh_time",
    "tee_receiver",
    "tee_source_ip",
    "tee_port",
    "tee_transparent",
    "tee_max_receivers",
    "tee_heterogeneous",
    "tee_pipe",
    "sflow_port",
    "print_output",
    "print_output_separator",
    "print_output_file",
    "print_markers",
    "print_num_entries",
    "print_refresh_time",
    "print_write_header",
    "nfacctd_time_secs",
    "nfacctd_time_new",
    "nfacctd_rr",
    "nfacctd_as_new",
    "nfacctd_pre_portcheck",
    "nfacctd_sql_backup_logfile",
    "networks_file",
    "networks_mask",
    "networks_cache_entries",
    "ports_file",
    "conntrack_buffer_size",
    "post_vtep_tag_checks",
    "daemon_proc_name",
    "proc_priority",
    "geoip_ipv4_file",
    "geoip_ipv6_file",
    "plugin_pipe_size",
    "plugin_buffer_size",
    "plugin_buffer_zerocopy",
    "tmp_net_own_field",
    "tmp_asa_bi_flow",
];

/// 簡易判定 (`key: value` 行 + 既知キー)。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Some(c) = parse(b) else {
        return false;
    };
    c.entries >= 4 && c.known_keys >= 3
}

/// パースして計数を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let s = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        entries: 0,
        known_keys: 0,
        comments: 0,
    };
    for line in s.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') {
            c.comments += 1;
            continue;
        }
        let Some(colon) = t.find(':') else {
            continue;
        };
        let key = t[..colon].trim();
        if key.is_empty()
            || !key
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
        {
            continue;
        }
        c.entries += 1;
        if KNOWN_KEYS.contains(&key) {
            c.known_keys += 1;
        }
    }
    (c.entries > 0 || c.comments > 0).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::{detect, parse};

    const SAMPLE: &[u8] = b"daemonize: false\nsyslog: daemon\npmacctd_as: bgp\npmacctd_net: bgp\ninterface: eth0\npcap_filter: port 5141 or port 6343\nplugins: print[print], imt[imta]\nimt_path: /tmp/imt.pipe\nimt_client_pth: /tmp/imt_client.pipe\naggregate: src_host,dst_host,src_port,dst_port,proto\nsnaplen: 700\nfiles_umask: 022\n";

    #[test]
    fn detects_pmacctconf() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.entries, 12);
        assert_eq!(c.known_keys, 12);
    }

    #[test]
    fn rejects_other_yaml() {
        assert!(!detect(b"name: x\nport: 8080\nlevel: info\n"));
        assert!(!detect(b"a: 1\nb: 2\nc: 3\nd: 4\n"));
    }
}
