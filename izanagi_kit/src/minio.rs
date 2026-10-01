//! MinIO server config (`config.json` / `config.env`) parser.
//!
//! Detects either the legacy JSON `config.json` (`"version"` + `"credential"`
//! (`accessKey`/`secretKey`) + `"region"`/`"notify"`/`"logger"`/`"cache"`/
//! `"audit"` blocks) or the current environment file (`MINIO_ROOT_USER=`/
//! `MINIO_ROOT_PASSWORD=`/`MINIO_VOLUMES=`/`MINIO_OPTS=`/`MINIO_REGION_NAME=`/
//! `MINIO_DOMAIN=`/`MINIO_SERVER_URL=` …), and counts entries, sections,
//! credentials, and comments.
//!
//! ```
//! let b = br#"MINIO_ROOT_USER=admin
//! MINIO_ROOT_PASSWORD=secret
//! MINIO_VOLUMES="/mnt/data"
//! MINIO_OPTS="--console-address :9001"
//! MINIO_REGION_NAME="us-east-1""#;
//! assert!(izanagi_kit::minio::detect(b));
//! let c = izanagi_kit::minio::Minio::parse(b).unwrap();
//! assert_eq!(c.entries, 5);
//! ```

/// Parsed MinIO config summary.
#[derive(Debug, Clone)]
pub struct Minio {
    /// Config entries (env `KEY=value` lines or JSON `"key":` pairs).
    pub entries: usize,
    /// JSON section keys (`credential`/`region`/`notify`/`logger`/`cache`/`audit`/`policy`/`api`).
    pub sections: usize,
    /// Credential-related entries (`accessKey`/`secretKey`/`MINIO_ROOT_*`).
    pub credentials: usize,
    /// `MINIO_*` env vars.
    pub minio_vars: usize,
    /// `#`/`//` comment lines.
    pub comments: usize,
}

/// MinIO env vars.
const ENV_KEYS: &[&str] = &[
    "MINIO_ROOT_USER",
    "MINIO_ROOT_PASSWORD",
    "MINIO_VOLUMES",
    "MINIO_OPTS",
    "MINIO_REGION_NAME",
    "MINIO_DOMAIN",
    "MINIO_SERVER_URL",
    "MINIO_BROWSER",
    "MINIO_BROWSER_REDIRECT_URL",
    "MINIO_API_ROOT_ACCESS",
    "MINIO_API_REQUESTS_MAX",
    "MINIO_API_REQUESTS_DEADLINE",
    "MINIO_API_REQUESTS_QUEUE_SIZE",
    "MINIO_API_READY_DEADLINE",
    "MINIO_API_CLUSTER_BARRIER",
    "MINIO_API_CORS_ALLOW_ORIGIN",
    "MINIO_API_REMOTE_TRANSPORT_DEADLINE",
    "MINIO_API_LIST_QUORUM",
    "MINIO_API_REPLICATION_WORKERS",
    "MINIO_API_REPLICATION_FAILED_WORKERS",
    "MINIO_API_TRANSITION_WORKERS",
    "MINIO_API_STAGED_UPLOADS_EXPIRY",
    "MINIO_API_DELETE_OBJECTS_MAX",
    "MINIO_PROMETHEUS_AUTH_TYPE",
    "MINIO_PROMETHEUS_JOB_ID",
    "MINIO_PROMETHEUS_URL",
    "MINIO_NOTIFY_AMQP",
    "MINIO_NOTIFY_KAFKA",
    "MINIO_NOTIFY_MQTT",
    "MINIO_NOTIFY_MYSQL",
    "MINIO_NOTIFY_NATS",
    "MINIO_NOTIFY_NSQ",
    "MINIO_NOTIFY_ELASTICSEARCH",
    "MINIO_NOTIFY_REDIS",
    "MINIO_NOTIFY_POSTGRES",
    "MINIO_NOTIFY_WEBHOOK",
    "MINIO_NOTIFY_NATS_USERNAME",
    "MINIO_NOTIFY_NATS_PASSWORD",
    "MINIO_AUDIT_WEBHOOK",
    "MINIO_AUDIT_KAFKA",
    "MINIO_LOGGER_WEBHOOK",
    "MINIO_CACHE",
    "MINIO_CACHE_DRIVES",
    "MINIO_CACHE_EXCLUDE",
    "MINIO_CACHE_EXPIRY",
    "MINIO_CACHE_QUOTA",
    "MINIO_CACHE_AFTER",
    "MINIO_CACHE_WATERMARK_LOW",
    "MINIO_CACHE_WATERMARK_HIGH",
    "MINIO_CACHE_RANGE",
    "MINIO_CACHE_COMMIT",
    "MINIO_KMS_KES",
    "MINIO_KMS_SECRET_KEY",
    "MINIO_KMS_AUTO_ENCRYPTION",
    "MINIO_IDENTITY_LDAP",
    "MINIO_IDENTITY_OPENID",
    "MINIO_POLICY",
    "MINIO_ETCD_ENDPOINTS",
    "MINIO_ETCD_PATH_PREFIX",
    "MINIO_ETCD_COREOS_PATH",
    "MINIO_PUBLIC_IPS",
    "MINIO_HOST_IP",
    "MINIO_DOMAINS",
    "MINIO_INSTANCE_ID",
    "MINIO_ENDPOINT_URL",
    "MINIO_CONSOLE_SUBPATH",
    "MINIO_STORAGE_CLASS_STANDARD",
    "MINIO_STORAGE_CLASS_RRS",
    "MINIO_COMPRESSION",
    "MINIO_COMPRESSION_EXTENSIONS",
    "MINIO_COMPRESSION_MIME_TYPES",
    "MINIO_DRIVE_SYNC",
    "MINIO_HEAL_DRIVES",
    "MINIO_HEAL_BITROT_SCAN",
    "MINIO_HEAL_INTERVAL",
    "MINIO_HEAL_MAX_PARALLEL",
    "MINIO_HEAL_MAX_IO",
    "MINIO_HEAL_BATCH_SIZE",
    "MINIO_HEAL_SLEEP",
    "MINIO_HEAL_TIMEOUT",
    "MINIO_SCANNER_SPEED",
    "MINIO_SCANNER_DELAY",
    "MINIO_SCANNER_MAX_WAIT",
    "MINIO_SCANNER_CYCLE",
    "MINIO_DELETED_OBJECT_RETENTION",
    "MINIO_ILM_EXPIRY_WORKERS",
    "MINIO_ILM_TRANSITION_WORKERS",
    "MINIO_ERASURE_SET_DRIVE_COUNT",
    "MINIO_ERASURE_SET",
    "MINIO_ERASURE_SET_SHARD_SIZE",
    "MINIO_DISTRIBUTED",
    "MINIO_WORM",
    "MINIO_UPDATE",
    "MINIO_UPDATE_MINISIGN_PUBKEY",
    "MINIO_ACTIVE_ACTIVE_REPLICATION",
    "MINIO_SITE_NAME",
    "MINIO_SITE_REGION",
    "MINIO_DEPLOYMENT_ID",
    "MINIO_CALLHOME",
    "MINIO_SUBNET_LICENSE",
    "MINIO_SUBNET_API_KEY",
    "MINIO_SUBNET_PROXY",
    "MINIO_FREE_FLOWING_BYTES",
    "MINIO_CONN_STATS",
    "MINIO_LOG_QUERY_URL",
    "MINIO_LOG_QUERY_AUTH_TOKEN",
    "MINIO_LOG_QUERY_TIMEOUT",
    "MINIO_AUDIT_LOG_PREFIX",
    "MINIO_AUDIT_LOG_KAFKA",
    "MINIO_AUDIT_LOG_WEBHOOK",
];

/// config.json sections.
const JSON_SECTIONS: &[&str] = &[
    "credential",
    "credentials",
    "region",
    "browser",
    "domain",
    "storage-class",
    "storage_class",
    "notify",
    "logger",
    "audit",
    "policy",
    "api",
    "cache",
    "compress",
    "etcd",
    "identity_ldap",
    "identity_openid",
    "heal",
    "scanner",
    "kms",
    "subnet",
    "callhome",
    "lambda",
    "version",
];

fn is_env_key(t: &str) -> bool {
    t.starts_with("MINIO_")
        && t.contains('=')
        && t[..t.find('=').unwrap_or(0)]
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Whether the buffer looks like a MinIO config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let env = t
        .lines()
        .filter(|l| {
            let tr = l.trim();
            is_env_key(tr)
                && ENV_KEYS
                    .iter()
                    .any(|k| &tr[..tr.find('=').unwrap_or(0)] == *k || tr.starts_with(*k))
        })
        .count();
    if env >= 1 {
        return true;
    }
    let json = t.contains("\"credential\"")
        || t.contains("\"accessKey\"")
        || t.contains("\"secretKey\"")
        || (t.contains("\"version\"")
            && JSON_SECTIONS
                .iter()
                .any(|s| t.contains(&format!("\"{s}\""))))
            && t.contains("\"notify\"")
        || t.contains("\"logger\"");
    json
}

impl Minio {
    /// Count entries in a MinIO config. Returns `None` when the input does
    /// not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            entries: 0,
            sections: 0,
            credentials: 0,
            minio_vars: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with('#') || tr.starts_with("//") {
                c.comments += 1;
                continue;
            }
            if let Some(eq) = tr.find('=') {
                let key = &tr[..eq];
                if is_env_key(tr) {
                    c.entries += 1;
                    c.minio_vars += 1;
                    if key.contains("ROOT") || key.contains("SECRET") {
                        c.credentials += 1;
                    }
                    continue;
                }
            }
            // JSON style: count "key": pairs
            let mut rest = tr;
            while let Some(i) = rest.find('"') {
                rest = &rest[i + 1..];
                let Some(end) = rest.find('"') else {
                    break;
                };
                let key = &rest[..end];
                rest = &rest[end + 1..];
                if rest.starts_with(':') {
                    c.entries += 1;
                    if JSON_SECTIONS.contains(&key) {
                        c.sections += 1;
                    }
                    if key == "accessKey" || key == "secretKey" {
                        c.credentials += 1;
                    }
                }
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_env_file() {
        let b = br#"# minio env
MINIO_ROOT_USER=minioadmin
MINIO_ROOT_PASSWORD=minioadmin-secret
MINIO_VOLUMES="/mnt/data{1...4}"
MINIO_OPTS="--console-address :9001"
MINIO_DOMAIN=minio.local
MINIO_REGION_NAME="us-east-1"
MINIO_BROWSER="on"
MINIO_NOTIFY_WEBHOOK_ENABLE_target1=on
MINIO_NOTIFY_WEBHOOK_ENDPOINT_target1=http://hook.local
"#;
        assert!(detect(b));
        let c = Minio::parse(b).unwrap();
        assert_eq!(c.entries, 9);
        assert_eq!(c.minio_vars, 9);
        assert_eq!(c.credentials, 2);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn detects_config_json() {
        let b = br#"{
  "version": "33",
  "credential": { "accessKey": "AK", "secretKey": "SK" },
  "region": "us-east-1",
  "browser": "on",
  "notify": { "webhook": { "1": { "enable": true, "endpoint": "http://x" } } },
  "logger": { "console": { "enable": true } }
}"#;
        assert!(detect(b));
        let c = Minio::parse(b).unwrap();
        assert_eq!(c.sections, 6);
        assert_eq!(c.credentials, 2);
    }

    #[test]
    fn rejects_other_env() {
        assert!(!detect(b"FOO=bar\nBAZ=qux\nHOME=/root\n"));
        assert!(!detect(b"PATH=/usr/bin\nSHELL=/bin/sh\nUSER=root\n"));
    }
}
