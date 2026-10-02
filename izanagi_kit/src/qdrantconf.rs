//! Qdrant `config.yaml` の検出と構造カウント。
//!
//! `storage`/`service`/`cluster`/`telemetry_disabled`/`log_level` 等の
//! 既知トップキーとセクション内既知キーをインデント走査で分類する。
//!
//! ```
//! let c = izanagi_kit::qdrantconf::parse(
//!     b"log_level: INFO\nstorage:\n  storage_path: ./storage\nservice:\n  http_port: 6333\n").unwrap();
//! assert_eq!(c.sections, 3);
//! assert!(izanagi_kit::qdrantconf::detect(b"storage:\n  storage_path: ./s\nservice:\n  http_port: 6333\n"));
//! ```

/// 既知トップレベルキー。
const SECTIONS: &[&str] = &[
    "cluster",
    "consensus",
    "distributed",
    "gpu",
    "hnsw",
    "log_level",
    "optimizer_cpu_budget",
    "service",
    "storage",
    "telemetry_disabled",
    "tls",
    "tracing",
];
/// セクション配下の既知キー(先頭セグメント一致)。
const SUBKEYS: &[&str] = &[
    "affinity",
    "auto_flush_interval",
    "bootstrap",
    "bootstrap_uri",
    "collection",
    "compression",
    "confirm_timeout",
    "content",
    "default_segment_number",
    "default_write_consistency_factor",
    "disabled",
    "enable_cors",
    "enable_tls",
    "enable_toc",
    "enabled",
    "flush_interval_sec",
    "grpc_port",
    "host",
    "http_port",
    "indexing_threshold",
    "jitter_ms",
    "lookup_retry_timeout",
    "max_search_threads",
    "max_request_size_mb",
    "max_workers",
    "memmap_threshold",
    "message_send_tries",
    "metadata",
    "on_disk",
    "on_disk_payload",
    "optimizers",
    "p2p",
    "path",
    "peer",
    "performance",
    "period_ms",
    "port",
    "quantization",
    "read_consistency",
    "recovery",
    "shard_transfer_method",
    "snapshot",
    "snapshots_path",
    "storage_path",
    "tick_period_ms",
    "timeout",
    "uri",
    "update_queue_size",
    "update_rate_limit",
    "uri_port",
    "verify",
    "wal",
    "wal_capacity_mb",
    "wal_segments_ahead",
    "write_consistency_factor",
];

/// Qdrant 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知トップキー行。
    pub sections: usize,
    /// セクション配下の既知キー。
    pub options: usize,
    /// `- ` リスト項目。
    pub items: usize,
    /// `#` コメント行。
    pub comments: usize,
    /// 分類不能行。
    pub misc: usize,
}

/// b が Qdrant config.yaml かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    text.lines()
        .filter(|l| {
            let t = l.trim();
            t.find(':')
                .is_some_and(|p| SECTIONS.contains(&t[..p].trim()))
        })
        .count()
        >= 2
}

/// config.yaml の構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        options: 0,
        items: 0,
        comments: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if t.starts_with('-') {
            c.items += 1;
            continue;
        }
        let Some(colon) = t.find(':') else {
            c.misc += 1;
            continue;
        };
        let key = t[..colon].trim();
        let indent = line.len() - line.trim_start().len();
        if indent == 0 && SECTIONS.contains(&key) {
            c.sections += 1;
        } else if SUBKEYS.contains(&key) || SECTIONS.contains(&key) {
            c.options += 1;
        } else {
            c.misc += 1;
        }
    }
    (c.options >= 2).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# qdrant\nlog_level: INFO\ntelemetry_disabled: true\n\nstorage:\n  storage_path: ./storage\n  snapshots_path: ./snapshots\n  on_disk_payload: true\n  wal:\n    wal_capacity_mb: 32\n\nservice:\n  host: 0.0.0.0\n  http_port: 6333\n  grpc_port: 6334\n  enable_tls: false\n\ncluster:\n  enabled: true\n  p2p:\n    port: 6335\n";

    #[test]
    fn qdrantconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 5);
        assert_eq!(c.options, 12);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn not_qdrant() {
        assert!(!detect(b"key: value\nother: 1\n"));
        assert!(!detect(b"hello\n"));
    }
}
