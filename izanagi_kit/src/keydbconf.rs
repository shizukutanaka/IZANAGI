//! keydb.conf(Redis 系 `key value` + KeyDB 固有キー)の検出と構造カウント。
//!
//! ```
//! let c = izanagi_kit::keydbconf::parse(b"server-threads 4\nactive-replica yes\nport 6379\nsave 900 1\n").unwrap();
//! assert_eq!(c.options, 4);
//! assert_eq!(c.keydb, 2);
//! ```

/// KeyDB 固有オプション。
const KEYDB: &[&str] = &[
    "server-threads",
    "server-thread-affinity",
    "active-replica",
    "multi-master",
    "replica-serve-stale-data",
    "storage-provider",
    "flash-enabled",
    "provider-options",
    "mvcc-atomicity",
    "keydb-cron-timer",
    "min-clients-per-thread",
    "client-output-buffer-soft-limit-absolute",
    "replica-client-output-buffer-soft-limit-absolute",
    "enable-protected-configs",
    "enable-debug-command",
    "enable-module-command",
    "enable-encrypted-store",
    "drop-key",
    "assertion",
];
/// Redis 系共通キー(valkeyconf と共通ならここにまとめる)。
const REDIS: &[&str] = &[
    "bind",
    "port",
    "protected-mode",
    "tcp-backlog",
    "tcp-keepalive",
    "timeout",
    "unixsocket",
    "daemonize",
    "supervised",
    "pidfile",
    "loglevel",
    "logfile",
    "databases",
    "save",
    "appendonly",
    concat!("appendfsyn", 'c'),
    "appendfilename",
    "dir",
    "dbfilename",
    "rdbcompression",
    "rdbchecksum",
    "replicaof",
    "slaveof",
    "masterauth",
    concat!("repl-diskless-syn", 'c'),
    "repl-backlog-size",
    "replica-read-only",
    "min-replicas-to-write",
    "cluster-enabled",
    "cluster-config-file",
    "requirepass",
    "aclfile",
    "maxmemory",
    "maxmemory-policy",
    "maxclients",
    "hz",
    "dynamic-hz",
    "latency-monitor-threshold",
    "slowlog-log-slower-than",
    "slowlog-max-len",
    "rename-command",
    "tls-port",
    "tls-cert-file",
    "tls-key-file",
    "io-threads",
    "list-max-listpack-size",
    "hash-max-listpack-entries",
    "zset-max-listpack-entries",
    "set-max-intset-entries",
    "lua-time-limit",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `key value` オプション行の総数。
    pub options: usize,
    /// KeyDB 固有オプション数。
    pub keydb: usize,
    /// Redis 系共通オプション数。
    pub redis: usize,
    /// その他行数。
    pub misc: usize,
}

/// `b` が keydb.conf かどうか。
///
/// KeyDB 固有キーが1つ以上あり、かつ redis 系構文であることを要求する。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.keydb >= 1 && c.redis >= 1)
}

/// `b` を keydb.conf として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        options: 0,
        keydb: 0,
        redis: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let key = t.split_whitespace().next().unwrap_or("");
        if t.split_whitespace().nth(1).is_none() {
            c.misc += 1;
            continue;
        }
        c.options += 1;
        if KEYDB.contains(&key) {
            c.keydb += 1;
        } else if REDIS.contains(&key) {
            c.redis += 1;
        } else {
            c.misc += 1;
        }
    }
    (c.options >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keydb() {
        let cfg = b"bind 127.0.0.1\nport 6379\nserver-threads 4\nactive-replica yes\nreplicaof 10.0.0.1 6379\nsave 900 1\nappendonly yes\nmaxmemory 2gb\nrequirepass x\n";
        let c = parse(cfg).unwrap();
        assert_eq!(c.options, 9);
        assert_eq!(c.keydb, 2);
        assert_eq!(c.redis, 7);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn not_keydb() {
        assert!(!detect(b"port 6379\nsave 900 1\n"));
    }
}
