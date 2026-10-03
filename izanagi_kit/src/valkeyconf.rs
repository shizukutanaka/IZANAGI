//! valkey.conf(Redis 系 `key value` 行)の検出と構造カウント。
//!
//! ```
//! let c = izanagi_kit::valkeyconf::parse(b"bind 127.0.0.1\nport 6379\nsave 900 1\nappendonly yes\nmaxmemory 1gb\nrequirepass x\n").unwrap();
//! assert_eq!(c.options, 6);
//! assert_eq!(c.net, 2);
//! assert_eq!(c.persistence, 2);
//! ```

/// 接続・ネットワーク系キー。
const NET: &[&str] = &[
    "bind",
    "port",
    "protected-mode",
    "tcp-backlog",
    "tcp-keepalive",
    "timeout",
    "unixsocket",
    "unixsocketperm",
    "io-threads",
    "io-threads-do-reads",
    "listen",
    "replica-announce-ip",
    "announce-ip",
    "maxclients",
];
/// 永続化系キー。
const PERSIST: &[&str] = &[
    "save",
    "appendonly",
    concat!("appendfsyn", 'c'),
    "appendfilename",
    "appenddirname",
    "rdbcompression",
    "rdbchecksum",
    "rdb-del-sync-files",
    "dbfilename",
    "dir",
    "stop-writes-on-bgsave-error",
    "no-appendfsync-on-rewrite",
    "auto-aof-rewrite-percentage",
    "auto-aof-rewrite-min-size",
    "aof-load-truncated",
    "aof-use-rdb-preamble",
    "aof-timestamp-enabled",
    "aof_rewrite_cpulist",
    "bgsave_cpulist",
];
/// レプリケーション・クラスタ系キー。
const REPL: &[&str] = &[
    "replicaof",
    "slaveof",
    "masterauth",
    "masteruser",
    "replica-serve-stale-data",
    "replica-read-only",
    concat!("repl-diskless-syn", 'c'),
    "repl-diskless-sync-delay",
    "repl-diskless-load",
    "repl-disable-tcp-nodelay",
    "repl-backlog-size",
    "repl-backlog-ttl",
    "replica-priority",
    "min-replicas-to-write",
    "min-replicas-max-lag",
    "replica-announce-port",
    "cluster-enabled",
    "cluster-config-file",
    "cluster-node-timeout",
    "cluster-migration-barrier",
    "cluster-require-full-coverage",
    "cluster-allow-reads-when-down",
];
/// 認証・コマンド・TLS 系キー。
const SECURITY: &[&str] = &[
    "requirepass",
    "aclfile",
    "acluser",
    "rename-command",
    "acllog-max-len",
    "tls-port",
    "tls-cert-file",
    "tls-key-file",
    "tls-ca-cert-file",
    "tls-auth-clients",
    "tls-replication",
    "tls-cluster",
];
/// メモリ・制限・ログ系キー。
const LIMITS: &[&str] = &[
    "maxmemory",
    "maxmemory-policy",
    "maxmemory-samples",
    "maxmemory-eviction-tenacity",
    "lazyfree-lazy-eviction",
    "lazyfree-lazy-expire",
    "lazyfree-lazy-server-del",
    "lazyfree-lazy-user-del",
    "lazyfree-lazy-user-flush",
    "latency-monitor-threshold",
    "slowlog-log-slower-than",
    "slowlog-max-len",
    "daemonize",
    "supervised",
    "pidfile",
    "loglevel",
    "logfile",
    "databases",
    "hz",
    "dynamic-hz",
    "jemalloc-bg-thread",
    "enable-debug-command",
    "enable-module-command",
    "list-max-listpack-size",
    "set-max-listpack-entries",
    "set-max-intset-entries",
    "zset-max-listpack-entries",
    "zset-max-listpack-value",
    "hash-max-listpack-entries",
    "hash-max-listpack-value",
    "stream-node-max-entries",
    "activerehashing",
    "always-show-logo",
    "server-threads",
    "replica-ignore-maxmemory",
    "tracking-table-max-keys",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `key value` オプション行の総数。
    pub options: usize,
    /// 接続・ネットワーク系オプション数。
    pub net: usize,
    /// 永続化系オプション数。
    pub persistence: usize,
    /// レプリケーション・クラスタ系オプション数。
    pub replication: usize,
    /// 認証・コマンド・TLS 系オプション数。
    pub security: usize,
    /// メモリ・制限・ログ系オプション数。
    pub limits: usize,
    /// その他行数。
    pub misc: usize,
}

fn key_of(line: &str) -> &str {
    line.split_whitespace().next().unwrap_or("")
}

/// `b` が valkey.conf かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.options >= 3 && c.misc == 0)
}

/// `b` を valkey.conf として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        options: 0,
        net: 0,
        persistence: 0,
        replication: 0,
        security: 0,
        limits: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let k = key_of(t);
        if t.split_whitespace().nth(1).is_none() {
            c.misc += 1;
            continue;
        }
        c.options += 1;
        if NET.contains(&k) {
            c.net += 1;
        } else if PERSIST.contains(&k) {
            c.persistence += 1;
        } else if REPL.contains(&k) {
            c.replication += 1;
        } else if SECURITY.contains(&k) {
            c.security += 1;
        } else if LIMITS.contains(&k) {
            c.limits += 1;
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
    fn valkey() {
        let cfg = b"bind 127.0.0.1\nport 6379\nprotected-mode yes\nsave 900 1\nappendonly yes\ndir /var/lib/valkey\nreplicaof 10.0.0.1 6379\nrequirepass s3cret\nmaxmemory 1gb\nmaxmemory-policy allkeys-lru\ndaemonize no\n# comment\n";
        let c = parse(cfg).unwrap();
        assert_eq!(c.options, 11);
        assert_eq!(c.net, 3);
        assert_eq!(c.persistence, 3);
        assert_eq!(c.replication, 1);
        assert_eq!(c.security, 1);
        assert_eq!(c.limits, 3);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn not_valkey() {
        assert!(!detect(b"random line\nfoo bar baz\n"));
    }
}
