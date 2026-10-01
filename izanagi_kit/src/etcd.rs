//! etcd `etcd.conf.yml`/etcd.conf parser.
//!
//! Detects the etcd member/cluster key namespace (`name`, `data-dir`,
//! `listen-peer-urls`, `listen-client-urls`, `initial-advertise-peer-urls`,
//! `advertise-client-urls`, `initial-cluster`, `initial-cluster-token`,
//! `initial-cluster-state`, `discovery`, `heartbeat-interval`,
//! `election-timeout`, `snapshot-count`, `quota-backend-bytes`,
//! `auto-compaction-*`, `client-transport-security`/`peer-transport-security`
//! blocks …) and counts keys, list items, bool values, and `#` comments.
//!
//! ```
//! let b = br#"name: infra1
//! data-dir: /var/lib/etcd
//! listen-peer-urls: http://localhost:2380
//! listen-client-urls: http://localhost:2379
//! initial-cluster: infra1=http://localhost:2380
//! initial-cluster-state: new
//! initial-cluster-token: etcd-cluster"#;
//! assert!(izanagi_kit::etcd::detect(b));
//! let c = izanagi_kit::etcd::Etcd::parse(b).unwrap();
//! assert_eq!(c.known_keys, 7);
//! ```

/// Parsed etcd.conf summary.
#[derive(Debug, Clone)]
pub struct Etcd {
    /// `key:`/`key: value` lines.
    pub keys: usize,
    /// Recognized etcd config keys.
    pub known_keys: usize,
    /// TLS/security sub-block keys.
    pub tls_keys: usize,
    /// `- ` list items (peer/cluster urls).
    pub list_items: usize,
    /// `true`/`false`/`enabled`/`disabled`/`on`/`off` values.
    pub bool_values: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Well-known etcd config keys.
const KEYS: &[&str] = &[
    "name",
    "data-dir",
    "wal-dir",
    "snapshot-count",
    "heartbeat-interval",
    "election-timeout",
    "listen-peer-urls",
    "listen-client-urls",
    "listen-client-http-urls",
    "max-snapshots",
    "max-wals",
    "quota-backend-bytes",
    "backend-batch-limit",
    "backend-bbolt-freelist-type",
    "backend-batch-interval",
    "max-txn-ops",
    "max-request-bytes",
    "grpc-keepalive-min-time",
    "grpc-keepalive-interval",
    "grpc-keepalive-timeout",
    "socket-reuse-port",
    "socket-reuse-address",
    "initial-advertise-peer-urls",
    "initial-cluster",
    "initial-cluster-state",
    "initial-cluster-token",
    "advertise-client-urls",
    "discovery",
    "discovery-fallback",
    "discovery-proxy",
    "discovery-srv",
    "discovery-srv-name",
    "discovery-endpoints",
    "dial-timeout",
    "dial-retries",
    "dial-backoff",
    "discovery-token",
    "strict-reconfig-check",
    "pre-vote",
    "auto-compaction-mode",
    "auto-compaction-retention",
    "client-transport-security",
    "peer-transport-security",
    "cert-file",
    "key-file",
    "client-cert-auth",
    "trusted-ca-file",
    "auto-tls",
    "peer-cert-file",
    "peer-key-file",
    "peer-client-cert-auth",
    "peer-trusted-ca-file",
    "peer-auto-tls",
    "peer-cert-allowed-cn",
    "peer-cert-allowed-hostname",
    "self-signed-cert-validity",
    "cipher-suites",
    "cors",
    "host-whitelist",
    "auth-token",
    "auth-token-ttl",
    "bcrypt-cost",
    "proxy",
    "proxy-failure-wait",
    "proxy-refresh-interval",
    "proxy-dial-timeout",
    "proxy-write-timeout",
    "proxy-read-timeout",
    "fallback",
    "logger",
    "log-level",
    "log-format",
    "log-outputs",
    "log-file",
    "log-rotation-config-json",
    "enable-log-rotation",
    "enable-pprof",
    "debug",
    "metrics",
    "listen-metrics-urls",
    "initial-election-tick-advance",
    "force-new-cluster",
    "wal-sync-duration",
    "wal-fsync-duration",
    "unsafe-no-fsync",
    "unsafe-warning",
    "snapshot-catchup-entries",
    "snapshot-send-timeout",
    "snapshot-recv-timeout",
    "lease-checkpoint-interval",
    "lease-checkpoint-persist",
    "compactor-batch-limit",
    "compactor-checkpoint-interval",
    "watch-progress-notify-interval",
    "warning-check-interval",
    "experimental-compaction-batch-limit",
    "experimental-backend-bbolt-freelist-type",
    "experimental-bootstrap-defrag-threshold-megabytes",
    "experimental-compaction-sleep-interval",
    "experimental-downgrade-checks-enabled",
    "experimental-memory-mlock",
    "experimental-txn-mode-write-with-shared-buffer",
    "experimental-watch-progress-notify-interval",
    "experimental-warning-apply-duration",
    "experimental-corrupt-check-time",
    "experimental-peer-skip-client-san-verification",
    "experimental-enable-lease-checkpoint",
    "experimental-enable-lease-checkpoint-persist",
    "experimental-max-learners",
    "experimental-initial-corrupt-check",
    "experimental-compact-hash-check-enabled",
    "experimental-compact-hash-check-time",
    "v2-deprecation",
    "feature-gates",
    "warning-unary-request-duration",
    "enable-distributed-tracing",
    "distributed-tracing-address",
    "distributed-tracing-service-name",
    "distributed-tracing-service-instance-id",
    "distributed-tracing-sampling-rate",
    "defrag-threshold",
];

/// Sub-block keys that hold TLS settings.
const TLS_BLOCKS: &[&str] = &["client-transport-security", "peer-transport-security"];

fn key_of_line(tr: &str) -> Option<(&str, &str)> {
    let colon = tr.find(':')?;
    if colon == 0 {
        return None;
    }
    let key = tr[..colon].trim_end();
    if !key
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '-' | '.'))
    {
        return None;
    }
    Some((key, tr[colon + 1..].trim_start()))
}

/// Whether the buffer looks like an etcd config file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = t
        .lines()
        .filter(|l| {
            let tr = l.trim();
            !tr.starts_with('#')
                && !tr.starts_with('-')
                && key_of_line(tr).is_some_and(|(k, _)| KEYS.contains(&k))
        })
        .count();
    hits >= 2 || t.contains("initial-cluster")
}

impl Etcd {
    /// Count keys in an etcd config. Returns `None` when the input does not
    /// look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            known_keys: 0,
            tls_keys: 0,
            list_items: 0,
            bool_values: 0,
            comments: 0,
        };
        let mut tls_indent: Option<usize> = None;
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            let indent = l.len() - tr.len();
            if tr.starts_with('-') {
                c.list_items += 1;
                continue;
            }
            let Some((key, v)) = key_of_line(tr) else {
                continue;
            };
            if let Some(ti) = tls_indent {
                if indent > ti {
                    c.tls_keys += 1;
                } else {
                    tls_indent = None;
                }
            }
            if TLS_BLOCKS.contains(&key) && v.is_empty() {
                tls_indent = Some(indent);
            }
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
        let b = br#"# etcd.conf.yml
name: infra1
data-dir: /var/lib/etcd
wal-dir: /var/lib/etcd/wal
snapshot-count: 10000
heartbeat-interval: 100
election-timeout: 1000
listen-peer-urls: http://localhost:2380
listen-client-urls: http://localhost:2379
max-snapshots: 5
max-wals: 5
quota-backend-bytes: 8589934592
initial-advertise-peer-urls: http://localhost:2380
initial-cluster: infra1=http://localhost:2380,infra2=http://localhost:2382
initial-cluster-state: new
initial-cluster-token: etcd-cluster-1
advertise-client-urls: http://localhost:2379
client-transport-security:
  cert-file: /etc/etcd/server.pem
  key-file: /etc/etcd/server-key.pem
  client-cert-auth: true
  trusted-ca-file: /etc/etcd/ca.pem
logger: zap
log-level: info
auto-compaction-mode: periodic
auto-compaction-retention: "1"
"#;
        assert!(detect(b));
        let c = Etcd::parse(b).unwrap();
        assert_eq!(c.keys, 25);
        assert_eq!(c.known_keys, 25);
        assert_eq!(c.tls_keys, 4);
        assert_eq!(c.bool_values, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_generic_yaml() {
        assert!(!detect(b"foo: bar\nname: x\nkind: z\n"));
    }
}
