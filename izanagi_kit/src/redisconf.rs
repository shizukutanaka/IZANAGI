//! Redis `redis.conf` parser.
//!
//! Detects the space-separated `directive value` configuration style
//! (`bind`, `port`, `save`, `appendonly`, `requirepass`, `replicaof`,
//! `maxmemory`, `daemonize` …) and counts directives by category: network,
//! persistence, security/ACL, and replication.
//!
//! ```
//! let b = b"bind 127 -::1\nport 6379\ndaemonize no\nsave 900 1\nappendonly yes\nrequirepass secret\nmaxmemory 256mb\n";
//! assert!(izanagi_kit::redisconf::detect(b));
//! let c = izanagi_kit::redisconf::Redisconf::parse(b).unwrap();
//! assert_eq!(c.keys, 7);
//! ```

/// Parsed redis.conf summary.
#[derive(Debug, Clone)]
pub struct Redisconf {
    /// Directive lines (non-comment, non-empty).
    pub keys: usize,
    /// Network keys (`bind`/`port`/`tcp-*`/`timeout`/`unixsocket*`/`tls-*`/`io-threads*`).
    pub net_keys: usize,
    /// Persistence keys (`save`/`append*`/`dir`/`dbfilename`/`rdb*`/`aof-*`/`auto-aof-*`/`stop-writes-on-bgsave-error`/`repl-diskless-*`).
    pub persist_keys: usize,
    /// Security/ACL keys (`requirepass`/`masterauth`/`aclfile`/`acllog*`/`acl-*`/`rename-command`/`protected-mode`/`enable-debug-command`/`enable-protected-configs`).
    pub secure_keys: usize,
    /// Replication keys (`replicaof`/`slaveof`/`repl-*`/`replica-*`/`min-replicas-*`/`min-slaves-*`/`masteruser`/`replication*`).
    pub replica_keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Network directives (prefix match).
const NET_KEYS: &[&str] = &[
    "bind",
    "port",
    "tcp-keepalive",
    "tcp-backlog",
    "timeout",
    "unixsocket",
    "tls-",
    "io-threads",
    "socket-mark-id",
    "listen",
    "daemonize",
    "pidfile",
    "loglevel",
    "logfile",
    "syslog",
    "databases",
];

/// Persistence directives (prefix match).
const PERSIST_KEYS: &[&str] = &[
    "save",
    "appendonly",
    "appendfsync",
    "appendfilename",
    "appenddirname",
    "dir",
    "dbfilename",
    "rdbcompression",
    "rdbchecksum",
    "rdb-del-sync-files",
    "sanitize-dump-payload",
    "aof-",
    "auto-aof-",
    "stop-writes-on-bgsave-error",
    "no-appendfsync-on-rewrite",
    "aof-use-rdb-preamble",
    "rdb-save-incremental-fsync",
];

/// Security & ACL directives (prefix match).
const SECURE_KEYS: &[&str] = &[
    "requirepass",
    "masterauth",
    "aclfile",
    "acllog",
    "acl-",
    "accpubsub-default",
    "rename-command",
    "protected-mode",
    "enable-debug-command",
    "enable-protected-configs",
    "enable-module-command",
    "user",
    "sumeuser",
];

/// Replication directives (prefix match).
const REPLICA_KEYS: &[&str] = &[
    "replicaof",
    "slaveof",
    "repl-",
    "replica-",
    "min-replicas",
    "min-slaves",
    "masteruser",
    "replication",
];

fn first_word(tr: &str) -> &str {
    tr.split(char::is_whitespace).next().unwrap_or("")
}

/// Detect a `redis.conf`-style file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let lc = t.to_ascii_lowercase();
    lc.contains("bind ")
        || lc.contains("appendonly ")
        || lc.contains("requirepass ")
        || lc.contains("replicaof ")
        || lc.contains("slaveof ")
        || lc.contains("maxmemory ")
        || lc.contains("aclfile ")
        || lc.contains("aof-")
        || (lc.contains("save ") && (lc.contains("dbfilename ") || lc.contains("dir ")))
}

impl Redisconf {
    /// Count directives in a `redis.conf`. Returns `None` when the input does
    /// not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            net_keys: 0,
            persist_keys: 0,
            secure_keys: 0,
            replica_keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            let w = first_word(tr);
            if w.is_empty() {
                continue;
            }
            c.keys += 1;
            if NET_KEYS.iter().any(|k| w.starts_with(k)) {
                c.net_keys += 1;
            }
            if PERSIST_KEYS.iter().any(|k| w.starts_with(k)) {
                c.persist_keys += 1;
            }
            if SECURE_KEYS.iter().any(|k| w.starts_with(k)) {
                c.secure_keys += 1;
            }
            if REPLICA_KEYS.iter().any(|k| w.starts_with(k)) {
                c.replica_keys += 1;
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
        let b = b"# redis\nbind 127 -::1\nprotected-mode yes\nport 6379\ntcp-backlog 511\ntimeout 0\ntcp-keepalive 300\ndaemonize no\ndatabases 16\nsave 900 1\nsave 300 10\nappendonly yes\nappendfsync everysec\ndir /var/lib/redis\ndbfilename dump.rdb\nrequirepass secret\naclfile /etc/redis/users.acl\nreplicaof master 6379\nreplica-read-only yes\nrepl-diskless-sync yes\nmaxmemory 256mb\nmaxmemory-policy allkeys-lru\n";
        assert!(detect(b));
        let c = Redisconf::parse(b).unwrap();
        assert_eq!(c.keys, 21);
        assert_eq!(c.comments, 1);
        assert_eq!(c.net_keys, 7);
        assert_eq!(c.persist_keys, 6);
        assert_eq!(c.secure_keys, 3);
        assert_eq!(c.replica_keys, 3);
    }

    #[test]
    fn rejects_nginx_conf() {
        assert!(!detect(b"events {}\nhttp {}\n"));
        assert!(Redisconf::parse(b"foo bar\n").is_none());
    }
}
