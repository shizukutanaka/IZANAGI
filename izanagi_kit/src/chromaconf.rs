//! Chroma DB environment config parser.
//!
//! Detects Chroma vector-DB configuration by `chroma_*` environment keys
//! (`chroma_server_host`/`chroma_server_http_port`/`chroma_db_impl`/
//! `chroma_server_auth_provider`/`persist_directory`/...), and counts
//! structure. `export ` prefixes are tolerated.
//!
//! ```
//! let b = b"chroma_server_host=0.0.0.0\nchroma_server_http_port=8000\nchroma_db_impl=duckdb+parquet\nis_persistent=true\n";
//! assert!(izanagi_kit::chromaconf::detect(b));
//! let c = izanagi_kit::chromaconf::Chroma::parse(b).unwrap();
//! assert!(c.keys >= 3);
//! ```

/// Parsed config summary.
#[derive(Debug, Clone)]
pub struct Chroma {
    /// Recognized key occurrences.
    pub keys: usize,
    /// `export ` prefixed lines.
    pub env_prefixes: usize,
    /// `KEY=value`/`key: value` lines.
    pub assignments: usize,
    /// `#`/`;` comment lines.
    pub comments: usize,
}

/// Known `chroma_*` keys.
const KEYS: &[&str] = &[
    "chroma_server_host",
    "chroma_server_http_port",
    "chroma_server_grpc_port",
    "chroma_server_ssl_enabled",
    "chroma_server_ssl_cert",
    "chroma_server_ssl_key",
    "chroma_server_api_default_path",
    "chroma_server_cors_allow_origins",
    "chroma_server_auth_provider",
    "chroma_server_auth_credentials",
    "chroma_server_auth_credentials_file",
    "chroma_server_auth_token_transport_header",
    "chroma_server_authn_provider",
    "chroma_server_authz_provider",
    "chroma_server_authz_attribute",
    "chroma_db_impl",
    "chroma_api_impl",
    "chroma_segments_manager_impl",
    "chroma_system_impl",
    "chroma_blockstore_provider",
    "chroma_sysdb_impl",
    "chroma_sysdb_provider",
    "chroma_mutator",
    "chroma_coordinator_host",
    "chroma_collection_member",
    "chroma_collection_segment_manager",
    "chroma_assignment_policy",
    "chroma_product_telemetry_impl",
    "chroma_telemetry_service_impl",
    "chroma_host_port",
    "chroma_otel_collection_endpoint",
    "chroma_otel_service_endpoint",
    "chroma_cache_implementation",
    "chroma_cache_policy",
    "chroma_cache_capacity_bytes",
    "is_persistent",
    "persist_directory",
    "anonymized_telemetry",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(k)
}

/// Detect a Chroma env config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = KEYS.iter().filter(|k| key_present(t, k)).count();
    hits >= 2
}

impl Chroma {
    /// Count categories. Returns `None` when the input does not look like
    /// a Chroma env config.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            env_prefixes: 0,
            assignments: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') || tr.starts_with(';') {
                c.comments += 1;
                continue;
            }
            if tr.starts_with("export ") {
                c.env_prefixes += 1;
            }
            if tr.contains('=') || tr.contains(": ") {
                c.assignments += 1;
            }
        }
        for k in KEYS {
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
        let b = b"export chroma_server_host=0.0.0.0\nchroma_server_http_port=8000\nchroma_db_impl=duckdb+parquet\nis_persistent=true\nanonymized_telemetry=false\n# c\n";
        assert!(detect(b));
        let c = Chroma::parse(b).unwrap();
        assert_eq!(c.keys, 5);
        assert_eq!(c.env_prefixes, 1);
        assert_eq!(c.assignments, 5);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_env() {
        assert!(!detect(b"HOME=/root\nPATH=/bin\n"));
        assert!(Chroma::parse(b"").is_none());
    }
}
