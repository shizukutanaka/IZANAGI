//! Qdrant `config.yaml` parser.
//!
//! Detects the Qdrant vector database config by its top-level sections
//! (`storage:`/`service:`/`cluster:`/`telemetry:`/`optimizer:`/`debug:`)
//! plus nested keys (`storage_path:`/`on_disk_payload:`/`grpc_port:`/
//! `shard_number:`/`p2p:`/`pprof:`).
//!
//! ```
//! let b = b"log_level: INFO\nstorage:\n  storage_path: ./storage\n  on_disk_payload: true\nservice:\n  host: 0.0.0.0\n  http_port: 6333\ncluster:\n  enabled: false\n";
//! assert!(izanagi_kit::qdrantconf::detect(b));
//! let c = izanagi_kit::qdrantconf::Qdrant::parse(b).unwrap();
//! assert_eq!(c.sections, 3);
//! ```

/// Parsed config.yaml summary.
#[derive(Debug, Clone)]
pub struct Qdrant {
    /// Recognized key occurrences.
    pub keys: usize,
    /// Top-level known sections.
    pub sections: usize,
    /// Indented `key:` option lines.
    pub options: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Known top-level sections.
const SECTION_KEYS: &[&str] = &[
    "storage:",
    "service:",
    "cluster:",
    "telemetry:",
    "optimizer:",
    "debug:",
    "quantization:",
    "distributed_mode:",
];

/// Known keys anywhere.
const OPTION_KEYS: &[&str] = &[
    "storage_path:",
    "snapshots_path:",
    "on_disk_payload:",
    "max_concurrent_requests:",
    "enable_tls:",
    "host:",
    "http_port:",
    "grpc_port:",
    "max_request_size_mb:",
    "uri:",
    "max_search_threads:",
    "max_optimization_threads:",
    "shard_number:",
    "replication_factor:",
    "default_segment_number:",
    "wal_capacity_mb:",
    "wal_segments_ahead:",
    "deleted_threshold:",
    "optimizers_overlap:",
    "vacuum_min_vector_number:",
    "default_shard_transfer_method:",
    "flush_interval_sec:",
    "shared:",
    "p2p:",
    "bootstrap:",
    "pprof:",
    "log_level:",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(k)
}

/// Detect a Qdrant config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = SECTION_KEYS
        .iter()
        .chain(OPTION_KEYS.iter())
        .filter(|k| key_present(t, k))
        .count();
    hits >= 2
}

impl Qdrant {
    /// Count categories. Returns `None` when the input does not look like
    /// a Qdrant config.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            sections: 0,
            options: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim_end();
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if !l.starts_with(' ') && tr.ends_with(':') && !tr.starts_with('-') {
                c.sections += 1;
            } else if tr.contains(": ") || tr.ends_with(':') {
                c.options += 1;
            }
        }
        for k in SECTION_KEYS {
            c.keys += t.matches(k).count();
        }
        for k in OPTION_KEYS {
            c.keys += t.matches(k).count();
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"log_level: INFO\nstorage:\n  storage_path: ./storage\n  snapshots_path: ./snapshots\n  on_disk_payload: true\n  shard_number: 4\nservice:\n  host: 0.0.0.0\n  http_port: 6333\n  grpc_port: 6334\n  enable_tls: false\ncluster:\n  enabled: true\n  p2p:\n    p2p_port: 6335\n";
        assert!(detect(b));
        let c = Qdrant::parse(b).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.options, 12);
        assert!(c.keys >= 10);
    }

    #[test]
    fn rejects_yaml() {
        assert!(!detect(b"foo: bar\nbaz: qux\n"));
        assert!(Qdrant::parse(b"").is_none());
    }
}
