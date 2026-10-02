//! Weaviate 環境変数型設定(`.env`/`weaviate.conf`)の検出と構造カウント。
//!
//! `PERSISTENCE_DATA_PATH`/`DEFAULT_VECTORIZER_MODULE`/`ENABLE_MODULES`/
//! `AUTHENTICATION_*`/`QUERY_DEFAULT_LIMIT`/`CLUSTER_*` 等の大文字キーを
//! `KEY=value` で分類する。
//!
//! ```
//! let c = izanagi_kit::weaviateconf::parse(
//!     b"PERSISTENCE_DATA_PATH=/data\nDEFAULT_VECTORIZER_MODULE=text2vec-transformers\n").unwrap();
//! assert_eq!(c.options, 2);
//! assert!(izanagi_kit::weaviateconf::detect(b"ENABLE_MODULES=text2vec-openai\nQUERY_DEFAULT_LIMIT=20\n"));
//! ```

/// 既知 Weaviate env キーのプレフィックス。
const PREFIXES: &[&str] = &[
    "AUTHENTICATION_",
    "AUTHORIZATION_",
    "BACKUP_",
    "CLUSTER_",
    "CONTEXTIONARY_",
    "DEFAULT_",
    "DISABLE_",
    "ENABLE_",
    "GOFLAGS_",
    "GRPC_",
    "INFERENCE_",
    "LIMIT_",
    "LOG_",
    "MODULE_",
    "MONITORING_",
    "ORIGIN_",
    "PERSISTENCE_",
    "PROMETHEUS_",
    "QUERY_",
    "RAFT_",
    "REPLICATION_",
    "TRACING_",
    "WEAVIATE_",
];
/// 単体既知キー。
const KEYS: &[&str] = &[
    "AUTHENTICATION_ANONYMOUS_ACCESS_ENABLED",
    "AUTHENTICATION_APIKEY_ALLOWED_KEYS",
    "AUTHENTICATION_APIKEY_ENABLED",
    "AUTHENTICATION_APIKEY_USERS",
    "AUTHENTICATION_OIDC_CLIENT_ID",
    "AUTHENTICATION_OIDC_ENABLED",
    "AUTHENTICATION_OIDC_ISSUER",
    "AUTHENTICATION_OIDC_USERNAME_CLAIM",
    "CLUSTER_API_PORT",
    "CLUSTER_DATA_BIND_PORT",
    "CLUSTER_GOSSIP_BIND_PORT",
    "CLUSTER_HOSTNAME",
    "CLUSTER_JOIN",
    "CONTEXTIONARY_URL",
    "DEFAULT_VECTOR_DISTANCE_METRIC",
    "DEFAULT_VECTORIZER_MODULE",
    "DISABLE_RECOVERY_ON_PANIC",
    "DISK_USE_READONLY_PERCENTAGE",
    "DISK_USE_WARNING_PERCENTAGE",
    "ENABLE_MODULES",
    "ENABLE_TOKENIZER_GSE",
    "ENABLE_TOKENIZER_JIEBA",
    "ENABLE_TOKENIZER_KAGOME_JA",
    "ENABLE_TOKENIZER_KAGOME_KR",
    "GRPC_PORT",
    "LIMIT_RESOURCES",
    "LOG_LEVEL",
    "MAXIMUM_BACKUP_LAYERS",
    "MEMORY_READONLY_PERCENTAGE",
    "MEMORY_WARNING_PERCENTAGE",
    "MODULES_CLIENT_TIMEOUT",
    "ORIGIN",
    "PERSISTENCE_DATA_PATH",
    "PERSISTENCE_HNSW_MAX_LOG_SIZE",
    "PERSISTENCE_LSM_ACCESS_STRATEGY",
    "PROMETHEUS_MONITORING_ENABLED",
    "PROMETHEUS_MONITORING_PORT",
    "QUERY_DEFAULT_LIMIT",
    "QUERY_MAXIMUM_RESULTS",
    "QUERY_NESTED_CROSS_REFERENCE_LIMIT",
    "QUERY_SLOW_LOG_ENABLED",
    "RAFT_BOOTSTRAP_EXPECT",
    "RAFT_BOOTSTRAP_TIMEOUT",
    "RAFT_ELECTION_TIMEOUT",
    "RAFT_ENABLE",
    "RAFT_HEARTBEAT_TIMEOUT",
    "RAFT_INTERNAL_PORT",
    "RAFT_JOIN",
    "RAFT_RPC_PORT",
    "REINDEX_VECTOR_DIMENSIONS_AT_BOOT",
    "REPLICATION_MINIMUM_FACTOR",
    "TRACK_VECTOR_DIMENSIONS",
    "USE_GOMEMLIMIT",
    "WEAVIATE_HOSTNAME",
];

/// Weaviate 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知環境変数行。
    pub options: usize,
    /// `#` コメント行。
    pub comments: usize,
    /// 分類不能行。
    pub misc: usize,
}

/// キーが既知かどうか。
fn known(key: &str) -> bool {
    KEYS.contains(&key)
        || PREFIXES.iter().any(|p| key.starts_with(p))
        || key.ends_with("_ENDPOINT")
        || key.ends_with("_PORT")
}

/// b が Weaviate 設定かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    text.lines()
        .filter(|l| {
            let t = l.trim();
            t.find('=').is_some_and(|p| known(t[..p].trim()))
        })
        .count()
        >= 2
}

/// Weaviate 設定の構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
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
        if t.starts_with('#') {
            c.comments += 1;
            continue;
        }
        let Some(pos) = t.find('=') else {
            c.misc += 1;
            continue;
        };
        if known(t[..pos].trim()) {
            c.options += 1;
        } else {
            c.misc += 1;
        }
    }
    (c.options >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# weaviate env\nPERSISTENCE_DATA_PATH=/var/lib/weaviate\nPERSISTENCE_HNSW_MAX_LOG_SIZE=500MB\nDEFAULT_VECTORIZER_MODULE=text2vec-transformers\nENABLE_MODULES=text2vec-openai,backup-s3\nQUERY_DEFAULT_LIMIT=20\nQUERY_MAXIMUM_RESULTS=10000\nAUTHENTICATION_ANONYMOUS_ACCESS_ENABLED=true\nAUTHENTICATION_APIKEY_ENABLED=false\nCLUSTER_HOSTNAME=node1\nLOG_LEVEL=info\nPROMETHEUS_MONITORING_ENABLED=true\nOTHER_VAR=1\n";

    #[test]
    fn weaviateconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.options, 11);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 1);
    }

    #[test]
    fn not_weaviate() {
        assert!(!detect(b"FOO=bar\nBAZ=1\n"));
        assert!(!detect(b"hello\n"));
    }
}
