//! aerospike.conf(コンテキスト `{ }` + `key value`)の検出と構造カウント。
//!
//! ```
//! let c = izanagi_kit::aerospike::parse(b"service {\n\tuser root\n}\nnetwork {\n\tservice {\n\t\tport 3000\n\t}\n}\nnamespace test {\n\tmemory-size 1G\n}\n").unwrap();
//! assert_eq!(c.contexts, 4);
//! assert_eq!(c.options, 3);
//! assert_eq!(c.namespaces, 1);
//! ```

/// トップレベル既知コンテキスト名。
const CONTEXTS: &[&str] = &[
    "service",
    "network",
    "logging",
    "xdr",
    "namespace",
    "security",
    "mod-lua",
    "set",
    "storage-engine",
];
/// ネストされる既知サブコンテキスト名。
const SUBCTX: &[&str] = &[
    "service",
    "heartbeat",
    concat!("fabri", 'c'),
    "info",
    "tls",
    "channel",
    "console",
    "file",
    "syslog",
    concat!('d', 'c'),
    "dc-security-config-file",
    "device",
    "memory",
    "index-type",
    "set-index",
    "sindex",
    "user",
    "log",
    "report-data-op",
    "report-violation",
    "feature-key-file",
    "seeds-address-port",
    "mesh",
];
/// `key value` オプションの既知先頭語。
const KEYS: &[&str] = &[
    "user",
    "group",
    "pidfile",
    "service-threads",
    "work-directory",
    "run-as-daemon",
    "feature-key-file",
    "address",
    "port",
    "access-address",
    "access-port",
    "alternate-access-address",
    "alternate-access-port",
    "tls-address",
    "tls-port",
    "tls-authenticate-client",
    "tls-cert-file",
    "tls-key-file",
    "tls-ca-file",
    "tls-protocols",
    "tls-cipher-suite",
    "mesh-seed-address-port",
    "interval",
    "timeout",
    "mode",
    "multicast-group",
    "multicast-address-port",
    "mtu",
    "protocol",
    "mtu",
    "connect-timeout-ms",
    "memory-size",
    "replication-factor",
    "default-ttl",
    "nsup-period",
    "high-water-disk-pct",
    "high-water-memory-pct",
    "stop-writes-pct",
    "mount",
    "file",
    "filesize",
    "data-in-memory",
    "read-page-cache",
    "sindex-type",
    "index-type",
    "storage-engine",
    "com",
    "device",
    "scheduler-mode",
    "write-block-size",
    "cold-start-empty",
    "migrate-order",
    "migrate-retransmit-ms",
    "max-record-size",
    "single-bin",
    "commit-to-device",
    "compression",
    "compression-level",
    "encryption",
    "encryption-key-file",
    "encryption-key-secret",
    "enable-authentication",
    "enable-quotas",
    "role",
    "privilege",
    "log-directory",
    "allow-builtin-users",
    "enable-ldap",
    "ldap-user-base-dn",
    "xdr-digestlog-size",
    "xdr-max-ship-throughput",
    "period-ms",
    "node-address-port",
    "local-address-port",
    "rev-ship-threshold-ms",
    "src-namespace",
    "src-set",
    "user-registered",
    "user-password",
    "syslog",
    "facility",
    "tag",
    "severity",
    "context",
    "any",
    "critical",
    "warning",
    "info",
    "debug",
    "detail",
    "interval-ms",
    "batch",
    "cluster-state",
    "connections",
    "info-port",
    "ticker-interval",
    "node-id",
    "transaction-max-ms",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `xxx {` コンテキスト開始行の総数。
    pub contexts: usize,
    /// `namespace` コンテキスト数。
    pub namespaces: usize,
    /// `logging`/`network`/`xdr`/`security`/`service` コンテキスト数。
    pub services: usize,
    /// 既知サブコンテキスト数(storage-engine/device/dc/console 等)。
    pub subcontexts: usize,
    /// `key value` オプション行の総数。
    pub options: usize,
    /// その他行数。
    pub misc: usize,
}

/// `b` が aerospike.conf かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.contexts >= 2 && c.namespaces >= 1)
}

/// `b` を aerospike.conf として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        contexts: 0,
        namespaces: 0,
        services: 0,
        subcontexts: 0,
        options: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if t.ends_with('{') {
            let name = t
                .trim_end_matches('{')
                .split_whitespace()
                .next()
                .unwrap_or("");
            c.contexts += 1;
            if name == "namespace" {
                c.namespaces += 1;
            } else if CONTEXTS.contains(&name) {
                c.services += 1;
            } else if SUBCTX.contains(&name) {
                c.subcontexts += 1;
            } else {
                c.misc += 1;
            }
            continue;
        }
        if t == "}" {
            continue;
        }
        let key = t.split_whitespace().next().unwrap_or("");
        if KEYS.contains(&key) && t.split_whitespace().nth(1).is_some() {
            c.options += 1;
        } else {
            c.misc += 1;
        }
    }
    (c.contexts >= 1 && c.options >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aerospike() {
        let cfg = b"service {\n\tuser root\n\tgroup root\n\tpidfile /run/aerospike.pid\n}\nnetwork {\n\tservice {\n\t\taddress any\n\t\tport 3000\n\t}\n\theartbeat {\n\t\tmode mesh\n\t\tport 3002\n\t\tinterval 150\n\t\ttimeout 10\n\t}\n}\nlogging {\n\tconsole {\n\t\tcontext any info\n\t}\n}\nnamespace test {\n\treplication-factor 2\n\tmemory-size 1G\n\tdefault-ttl 0\n\tstorage-engine memory\n}\n";
        let c = parse(cfg).unwrap();
        assert_eq!(c.contexts, 7);
        assert_eq!(c.namespaces, 1);
        assert_eq!(c.options, 14);
    }

    #[test]
    fn not_aerospike() {
        assert!(!detect(b"service { x }\n"));
    }
}
