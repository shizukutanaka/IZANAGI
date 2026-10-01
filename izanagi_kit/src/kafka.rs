//! Apache Kafka `server.properties` parser.
//!
//! Detects broker configuration files by the characteristic properties
//! (`broker.id`, `zookeeper.connect`, `log.dirs`, `listeners`, …) and counts
//! `key=value` assignments by category: identity/network, log retention,
//! and security (SSL/SASL/authorizer).
//!
//! ```
//! let b = b"broker.id=1\nlisteners=PLAINTEXT://:9092\nlog.dirs=/var/kafka\nzookeeper.connect=zk:2181\nnum.partitions=3\nlog.retention.hours=168\nssl.truststore.location=/tmp/trust.jks\n";
//! assert!(izanagi_kit::kafka::detect(b));
//! let c = izanagi_kit::kafka::Kafka::parse(b).unwrap();
//! assert_eq!(c.kv_pairs, 7);
//! assert!(c.broker_keys >= 4);
//! ```

/// Parsed Kafka broker config summary.
#[derive(Debug, Clone)]
pub struct Kafka {
    /// `key=value` assignment lines (comments excluded).
    pub kv_pairs: usize,
    /// Broker identity/network keys (`broker.id`/`listeners`/`zookeeper.connect`/…).
    pub broker_keys: usize,
    /// Log retention & segment keys (`log.*`/`message.`/`flush.*`).
    pub log_keys: usize,
    /// Security keys (`ssl.*`/`sasl.*`/`authorizer`/`super.users`/`acl*`).
    pub security_keys: usize,
    /// `#`/`!` comment lines.
    pub comments: usize,
}

/// Identity & network keys (prefix match).
const BROKER_KEYS: &[&str] = &[
    "broker.id",
    "node.id",
    "process.roles",
    "controller.",
    "listeners",
    "advertised.listeners",
    "zookeeper.connect",
    "zookeeper.",
    "num.partitions",
    "default.replication.factor",
    "offsets.topic",
    "num.network.threads",
    "num.io.threads",
    "queued.max.requests",
    "socket.request.max.bytes",
    "connections.max.idle.ms",
    "auto.create.topics.enable",
    "delete.topic.enable",
    "controlled.shutdown",
    "replica.lag",
    "num.replica",
    "min.insync.replicas",
    "unclean.leader",
    "group.initial",
    "max.connections",
    "inter.broker",
];

/// Log & storage keys (prefix match).
const LOG_KEYS: &[&str] = &[
    "log.dirs",
    "log.dir",
    "log.retention",
    "log.segment",
    "log.cleanup",
    "log.roll",
    "log.flush",
    "log.index",
    "log.preallocate",
    "log.message",
    "message.max.bytes",
    "flush.",
];

/// Security keys (prefix match).
const SECURITY_KEYS: &[&str] = &[
    "ssl.",
    "sasl.",
    "authorizer",
    "super.users",
    "acl",
    "principal.builder",
    "listener.security",
    "security.providers",
    "delegation.token",
];

/// Detect a Kafka `server.properties`-style file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    if t.contains("broker.id=") || t.contains("broker.id =") {
        return true;
    }
    let lc = t.to_ascii_lowercase();
    lc.contains("zookeeper.connect=")
        || lc.contains("log.dirs=")
        || (lc.contains("listeners=") && lc.contains("advertised.listeners="))
}

impl Kafka {
    /// Count key categories in a Kafka properties file. Returns `None` when
    /// nothing matches the format.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            kv_pairs: 0,
            broker_keys: 0,
            log_keys: 0,
            security_keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with('#') || tr.starts_with('!') {
                c.comments += 1;
                continue;
            }
            let Some(eq) = tr.find('=') else { continue };
            let key = tr[..eq].trim().to_ascii_lowercase();
            if key.is_empty() {
                continue;
            }
            c.kv_pairs += 1;
            if BROKER_KEYS.iter().any(|k| key.starts_with(k)) {
                c.broker_keys += 1;
            }
            if LOG_KEYS.iter().any(|k| key.starts_with(k)) {
                c.log_keys += 1;
            }
            if SECURITY_KEYS.iter().any(|k| key.starts_with(k)) {
                c.security_keys += 1;
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
        let b = b"broker.id=1\n# identity\nlisteners=PLAINTEXT://:9092,SSL://:9093\nadvertised.listeners=PLAINTEXT://host:9092\nnum.network.threads=8\nnum.io.threads=16\nsocket.request.max.bytes=104857600\nlog.dirs=/var/kafka\nnum.partitions=8\nlog.retention.hours=168\nlog.segment.bytes=1073741824\nlog.cleanup.policy=delete\nzookeeper.connect=zk1:2181,zk2:2181\nzookeeper.connection.timeout.ms=18000\nssl.truststore.location=/tmp/trust.jks\nssl.keystore.location=/tmp/key.jks\nauthorizer.class.name=kafka.security.authorizer.AclAuthorizer\nsuper.users=User:admin\n";
        assert!(detect(b));
        let c = Kafka::parse(b).unwrap();
        assert_eq!(c.kv_pairs, 17);
        assert_eq!(c.comments, 1);
        assert_eq!(c.log_keys, 4);
        assert_eq!(c.security_keys, 4);
        assert!(c.broker_keys >= 8);
    }

    #[test]
    fn kraft_style() {
        let b = b"process.roles=broker,controller\nnode.id=1\ncontroller.quorum.voters=1@h:9093\nlisteners=PLAINTEXT://:9092,CONTROLLER://:9093\nlog.dirs=/kraft\n";
        assert!(detect(b));
        let c = Kafka::parse(b).unwrap();
        assert!(c.broker_keys >= 2);
        assert_eq!(c.log_keys, 1);
    }

    #[test]
    fn rejects_plain_props() {
        assert!(!detect(b"foo=bar\napp.name=x\n"));
        assert!(Kafka::parse(b"version=1\n").is_none());
    }
}
