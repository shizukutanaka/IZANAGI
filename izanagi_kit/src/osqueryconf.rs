//! osquery `osquery.conf`/`osquery.flags` の検出と構造カウント。
//!
//! JSON の `options`(daemonize/schedule_splay_percent/events_expiry/
//! logger_path/pidfile/database_path/distributed_plugin/enroll_secret_path/
//! host_identifier/utc/numeric_monitoring...)、`schedule`、`packs`、
//! `decorators`、`auto_table_construction`、`discovery` を識別する。
//! `--flag=value` 形式の flagfile も同一キーで判定する。
//!
//! ```
//! let c = izanagi_kit::osqueryconf::parse(
//!     b"{\n  \"options\": {\n    \"host_identifier\": \"hostname\",\n    \"schedule_splay_percent\": 10\n  },\n  \"schedule\": {\n    \"system_info\": {\"query\": \"SELECT * FROM system_info;\", \"interval\": 3600}\n  }\n}\n").unwrap();
//! assert!(c.options >= 3);
//! assert!(izanagi_kit::osqueryconf::detect(
//!     b"{\"options\": {\"host_identifier\": \"hostname\"}, \"schedule\": {\"a\": {}}}\n"));
//! ```

use crate::textutil::strip_bom;
/// トップレベル既知キー。
const TOP_KEYS: &[&str] = &[
    "auto_table_construction",
    "decorators",
    "discovery",
    "events",
    "fim",
    "yara",
    "options",
    "packs",
    "platform",
    "schedule",
];

/// `options` 内既知キー。
const OPT_KEYS: &[&str] = &[
    "audit_allow_config",
    "audit_allow_sockets",
    "aws_access_key_id",
    "aws_region",
    "buffered_log_max",
    "database_path",
    "disable_audit",
    "disable_caching",
    "disable_distributed",
    "disable_events",
    "disable_logging",
    "disable_tables",
    "disable_watchdog",
    "distributed_interval",
    "distributed_plugin",
    "distributed_tls_read_endpoint",
    "distributed_tls_write_endpoint",
    "enable_file_events",
    "enable_monitor",
    "enable_syslog",
    "enroll_secret_path",
    "events_expiry",
    "events_max",
    "events_optimize",
    "host_identifier",
    "logger_event_type",
    "logger_min_status",
    "logger_min_stderr",
    "logger_path",
    "logger_plugin",
    "logger_snapshot_event_type",
    "numeric_monitoring",
    "pack_refresh_interval",
    "pidfile",
    "read_user_links",
    "schedule_default_interval",
    "schedule_epoch",
    "schedule_splay_percent",
    "schedule_timeout",
    "socket_events",
    "specify_service",
    concat!("ut", "\u{63}"),
    "verbose",
    "watchdog_level",
    "watchdog_memory_limit",
    "watchdog_utilization_limit",
    "watchdog_delay",
    "worker_threads",
];

/// 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知トップ/オプションキー行数。
    pub options: usize,
    /// `//` コメント行数。
    pub comments: usize,
    /// その他の行数。
    pub misc: usize,
}

/// `osquery.conf`/`osquery.flags` らしさを判定する。
pub fn detect(input: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(input) else {
        return false;
    };
    let text = strip_bom(text);
    let mut hits = 0usize;
    let mut top = 0usize;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("//") || t.starts_with('#') {
            continue;
        }
        top += TOP_KEYS
            .iter()
            .filter(|k| t.contains(&format!("\"{}\":", k)))
            .count();
        hits += OPT_KEYS
            .iter()
            .filter(|k| t.contains(&format!("\"{}\":", k)) || t.starts_with(&format!("{k}=")))
            .count();
        hits +=
            t.matches("--").count().min(1) * usize::from(t.starts_with("--") && t.contains('='));
        if top >= 1 && hits >= 1 {
            return true;
        }
        if hits >= 3 {
            return true;
        }
    }
    false
}

/// 構造をカウントする。
pub fn parse(input: &[u8]) -> Option<Counts> {
    if !detect(input) {
        return None;
    }
    let text = std::str::from_utf8(input).ok()?;
    let text = strip_bom(text);
    let mut c = Counts {
        options: 0,
        comments: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with("//") || t.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if TOP_KEYS
            .iter()
            .chain(OPT_KEYS.iter())
            .any(|k| t.contains(&format!("\"{}\":", k)) || t.starts_with(&format!("{k}=")))
        {
            c.options += 1;
        } else {
            c.misc += 1;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"{\n  \"options\": {\n    \"host_identifier\": \"hostname\",\n    \"schedule_splay_percent\": 10,\n    \"events_expiry\": 3600,\n    \"events_max\": 50000,\n    \"logger_path\": \"/var/log/osquery\",\n    \"pidfile\": \"/var/osquery/osquery.pidfile\",\n    \"database_path\": \"/var/osquery/osquery.db\"\n  },\n  \"schedule\": {\n    \"system_info\": {\n      \"query\": \"SELECT * FROM system_info;\",\n      \"interval\": 3600\n    }\n  },\n  \"packs\": {\n    \"internal_stuffs\": \"/usr/share/osquery/packs/internal_stuffs.conf\"\n  },\n  \"decorators\": {\n    \"load\": [\"SELECT uuid AS host_uuid FROM system_info;\"]\n  }\n}\n";

    #[test]
    fn osqueryconf() {
        let c = parse(SAMPLE).unwrap();
        assert!(c.options >= 10);
        assert!(c.misc >= 8);
    }

    #[test]
    fn not_osqueryconf() {
        assert!(!detect(b"{\"a\": 1}\n"));
        assert!(parse(b"text\n").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
