//! Weaviate environment / `.env` config parser.
//!
//! Detects Weaviate vector-DB configuration by its `UPPER_SNAKE_CASE`
//! environment keys (`QUERY_DEFAULTS_LIMIT`, `PERSISTENCE_DATA_PATH`,
//! `AUTHENTICATION_*`, `AUTHORIZATION_*`, `ENABLE_MODULES`,
//! `DEFAULT_VECTORIZER_MODULE`, `CLUSTER_*`, `BACKUP_*`,
//! `PROMETHEUS_MONITORING_*`), and counts structure. `export ` prefixes
//! are tolerated.
//!
//! ```
//! let b = b"PERSISTENCE_DATA_PATH=/var/lib/weaviate\nQUERY_DEFAULTS_LIMIT=20\nENABLE_MODULES=text2vec-transformers\nAUTHENTICATION_ANONYMOUS_ACCESS_ENABLED=true\n";
//! assert!(izanagi_kit::weaviateconf::detect(b));
//! let c = izanagi_kit::weaviateconf::Weaviate::parse(b).unwrap();
//! assert!(c.keys >= 4);
//! ```

/// Parsed config summary.
#[derive(Debug, Clone)]
pub struct Weaviate {
    /// Recognized key occurrences.
    pub keys: usize,
    /// `export ` prefixed lines.
    pub env_prefixes: usize,
    /// `KEY=value` lines.
    pub assignments: usize,
    /// `#`/`;` comment lines.
    pub comments: usize,
}

/// Known env keys.
const KEYS: &[&str] = &[
    "QUERY_DEFAULTS_LIMIT",
    "QUERY_MAXIMUM_RESULTS",
    "QUERY_CROSS_REFERENCE_DEPTH_LIMIT",
    "PERSISTENCE_DATA_PATH",
    "PERSISTENCE_MEMTABLES_FLUSH_IDLE_AFTER_SECONDS",
    "PERSISTENCE_MEMTABLES_MAX_SIZE_MB",
    "PERSISTENCE_LSM_ACCESS_STRATEGY",
    "PERSISTENCE_LSM_MAX_SEGMENT_SIZE_MB",
    "AUTHENTICATION_ANONYMOUS_ACCESS_ENABLED",
    "AUTHENTICATION_OIDC_ENABLED",
    "AUTHENTICATION_OIDC_ISSUER",
    "AUTHENTICATION_OIDC_CLIENT_ID",
    "AUTHENTICATION_OIDC_SKIP_CLAIMS",
    "AUTHENTICATION_APIKEY_ENABLED",
    "AUTHENTICATION_APIKEY_ALLOWED_KEYS",
    "AUTHENTICATION_APIKEY_USERS",
    "AUTHORIZATION_ADMINLIST_ENABLED",
    "AUTHORIZATION_ADMINLIST_USERS",
    "AUTHORIZATION_ADMINLIST_ROLES",
    "AUTHORIZATION_ADMINLIST_READONLY_USERS",
    "AUTHORIZATION_RBAC_ENABLED",
    "AUTHORIZATION_RBAC_ROOT_USERS",
    "AUTHORIZATION_RBAC_ROOT_GROUPS",
    "AUTHORIZATION_RBAC_VIEWER_ROLES",
    "ENABLE_MODULES",
    "DEFAULT_VECTORIZER_MODULE",
    "MODULES_CLIENT_TIMEOUT",
    "BACKUP_FILESYSTEM_PATH",
    "BACKUP_S3_ENDPOINT",
    "BACKUP_S3_BUCKET",
    "BACKUP_S3_PATH",
    "BACKUP_GCS_BUCKET",
    "BACKUP_GCS_PATH",
    "BACKUP_GCS_USE_SNAPSHOTS",
    "BACKUP_AZURE_CONTAINER",
    "BACKUP_AZURE_PATH",
    "PROMETHEUS_MONITORING_ENABLED",
    "PROMETHEUS_MONITORING_PORT",
    "PROMETHEUS_MONITORING_GROUP",
    "CLUSTER_HOSTNAME",
    "CLUSTER_GOSSIP_BIND_PORT",
    "CLUSTER_DATA_BIND_PORT",
    "CLUSTER_JOIN",
    "GRPC_PORT",
    "ORIGIN",
    "LIMIT_RESOURCES",
    "MAXIMUM_CONCURRENT_GET_REQUESTS",
    "MAXIMUM_CONCURRENT_BATCH_REQUESTS",
    "MAXIMUM_CONCURRENT_IMPORT_BATCHES",
    "VECTOR_INDEX_TYPE",
    "LOG_LEVEL",
    "DISABLE_TELEMETRY",
    "FORCE_FULL_REPLICAS_SEARCH",
    "WAIT_FOR_REPLICAS",
    "REPLICAS_MINIMUM_WAIT_TIME",
    "TRUST_INSECURE_PROXY",
    "DISABLE_RECUPERATE",
    "DISABLE_PROFILER",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(k)
}

/// Detect a Weaviate env config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = KEYS.iter().filter(|k| key_present(t, k)).count();
    hits >= 2
}

impl Weaviate {
    /// Count categories. Returns `None` when the input does not look like
    /// a Weaviate env config.
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
            if tr.contains('=') {
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
        let b = b"export PERSISTENCE_DATA_PATH=/var/lib/weaviate\nQUERY_DEFAULTS_LIMIT=20\nENABLE_MODULES=text2vec-transformers\nAUTHENTICATION_ANONYMOUS_ACCESS_ENABLED=true\nCLUSTER_HOSTNAME=node1\n# comment\n";
        assert!(detect(b));
        let c = Weaviate::parse(b).unwrap();
        assert_eq!(c.keys, 5);
        assert_eq!(c.env_prefixes, 1);
        assert_eq!(c.assignments, 5);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_env() {
        assert!(!detect(b"HOME=/root\nPATH=/bin\n"));
        assert!(Weaviate::parse(b"").is_none());
    }
}
