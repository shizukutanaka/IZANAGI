//! Grafana Tempo `tempo.yaml` の検出と構造カウント。
//!
//! ```
//! let c = izanagi_kit::tempoconf::parse(b"server:\n  http_listen_port: 3200\nstorage:\n  trace:\n    backend: local\n    local:\n      path: /tmp/traces\n").unwrap();
//! assert_eq!(c.sections, 2);
//! assert_eq!(c.backends, 1);
//! ```

/// 既知トップレベルキー。
const SECTIONS: &[&str] = &[
    "server",
    "distributor",
    "ingester",
    "compactor",
    "querier",
    "query_frontend",
    "metrics_generator",
    "storage",
    "memberlist",
    "overrides",
    "usage_report",
    "multitenancy_enabled",
    "internal_server",
    "target",
    "stream_over_http_enabled",
    "ui",
    "cache",
    "vulture",
];
/// storage.trace 配下の既知バックエンド名。
const BACKENDS: &[&str] = &[
    "local",
    "s3",
    "gcs",
    "azure",
    "wal",
    "block",
    "pool",
    "cache",
    "background_cache",
    "memcached",
    "redis",
    "index_downsample_bytes",
    "v2",
    "parquet",
    "version",
    "dedicated_columns",
    "queue",
    "worker",
    "compaction",
    "receivers",
    "forwarders",
    "trace",
];
/// metrics_generator 配下の既知名。
const GENERATOR: &[&str] = &[
    "ring",
    "kvstore",
    "processor",
    "service_graphs",
    "span_metrics",
    "local_blocks",
    "native_histograms",
    "exemplars",
    "storage",
    "remote_write",
    "collection_interval",
    "trace_id_label_name",
    "generate_native_histograms",
    "stale_duration",
    "metrics_ingest_time_slack",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// トップレベルセクション数。
    pub sections: usize,
    /// storage/trace 配下のバックエンド名数。
    pub backends: usize,
    /// metrics_generator 配下の既知名数。
    pub generators: usize,
    /// `- ` リスト項目数。
    pub items: usize,
    /// `key:` / `key: v` 行の総数。
    pub entries: usize,
    /// その他行数。
    pub misc: usize,
}

/// `b` が Tempo 設定かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.sections >= 2 && c.backends >= 1)
}

/// `b` を Tempo 設定として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        backends: 0,
        generators: 0,
        items: 0,
        entries: 0,
        misc: 0,
    };
    let mut section = "";
    let mut sindent = 0_usize;
    let mut sub = "";
    let mut known = 0;
    for line in text.lines() {
        if line.trim().is_empty() || line.trim().starts_with('#') {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        let t = line.trim();
        if t.starts_with('-') {
            c.items += 1;
            c.entries += 1;
            continue;
        }
        let Some(colon) = t.find(':') else {
            c.misc += 1;
            continue;
        };
        c.entries += 1;
        let key = &t[..colon];
        if indent == 0 {
            if SECTIONS.contains(&key) {
                c.sections += 1;
                known += 1;
            } else {
                c.misc += 1;
            }
            section = key;
            sindent = indent;
            sub = "";
        } else if indent == sindent + 2 {
            sub = key;
            if section == "metrics_generator" && GENERATOR.contains(&key) {
                c.generators += 1;
            }
        } else if indent == sindent + 4 {
            if section == "storage" && sub == "trace" && BACKENDS.contains(&key) {
                c.backends += 1;
            } else if section == "metrics_generator" && GENERATOR.contains(&key) {
                c.generators += 1;
            }
        }
    }
    (known >= 1 && c.entries >= 2).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tempo() {
        let cfg = b"server:\n  http_listen_port: 3200\ndistributor:\n  receivers:\n    otlp:\n      protocols:\n        grpc:\nquerier:\n  frontend_worker:\n    frontend_address: x\nstorage:\n  trace:\n    backend: local\n    wal:\n      path: /tmp/wal\n    local:\n      path: /tmp/traces\n    pool:\n      max_workers: 100\nmemberlist:\n  join_members: [a]\noverrides:\n  defaults:\n    metrics_generator:\n      processors: [service-graphs]\n";
        let c = parse(cfg).unwrap();
        assert_eq!(c.sections, 6);
        assert!(c.backends >= 3);
    }

    #[test]
    fn not_tempo() {
        assert!(!detect(b"key: value\n"));
    }
}
