//! RabbitMQ `rabbitmq.conf` (sysctl-style) parser.
//!
//! Detects the `key = value` INI-style broker configuration (dotted key
//! namespaces like `listeners.tcp.default`, `management.tcp.port`,
//! `cluster_formation.*`) and counts keys by category: listeners, auth/users,
//! memory/disk watermarks, and clustering.
//!
//! ```
//! let b = b"listeners.tcp.default = 5672\nmanagement.tcp.port = 15672\ndefault_user = guest\ndefault_pass = guest\nvm_memory_high_watermark.relative = 0\n";
//! assert!(izanagi_kit::rabbitmq::detect(b));
//! let c = izanagi_kit::rabbitmq::Rabbitmq::parse(b).unwrap();
//! assert_eq!(c.keys, 5);
//! ```

/// Parsed RabbitMQ config summary.
#[derive(Debug, Clone)]
pub struct Rabbitmq {
    /// `key = value` / `key=value` assignment lines.
    pub keys: usize,
    /// Listener keys (`listeners.*`/`ssl_options`/`mqtt.listeners`/`stomp.listeners`/`web_mqtt`).
    pub listener_keys: usize,
    /// User & auth keys (`default_*`/`auth_*`/`loopback_users`/`password_hashing`/`auth_backends`).
    pub auth_keys: usize,
    /// Memory, disk & flow keys (`vm_memory*`/`disk_free_limit`/`flow`/`raft.wal_max_size_bytes`).
    pub memory_keys: usize,
    /// Cluster keys (`cluster_*`/`cluster.name`/`cluster.nodes`/`classic_mirrored`/`queue_master_locator`).
    pub cluster_keys: usize,
    /// `#`/`%%`/`!` comment lines.
    pub comments: usize,
}

/// Listener & protocol keys (prefix match).
const LISTENER_KEYS: &[&str] = &[
    "listeners.",
    "ssl_options",
    "mqtt.listeners",
    "stomp.listeners",
    "web_mqtt",
    "web_stomp",
    "amqp_options",
    "num_acceptors",
    "handshake_timeout",
    "frame_max",
    "channel_max",
    "heartbeat",
    "tcp_listen_options",
    "proxy_protocol",
];

/// Auth & user keys (prefix match).
const AUTH_KEYS: &[&str] = &[
    "default_user",
    "default_pass",
    "default_vhost",
    "default_permissions",
    "default_user_tags",
    "auth_",
    "loopback_users",
    "password_hashing",
    "auth_backends",
    "ldap.",
    "credential_validator",
];

/// Memory & disk keys (prefix match).
const MEMORY_KEYS: &[&str] = &[
    "vm_memory",
    "disk_free_limit",
    "total_memory",
    "raft.",
    "mnesia_table_loading",
    "max_message_size",
];

/// Cluster keys (prefix match).
const CLUSTER_KEYS: &[&str] = &[
    "cluster_",
    "cluster.",
    "classic_mirrored",
    "queue_master_locator",
    "queue_leader_locator",
    "mirroring_sync",
];

/// Detect a `rabbitmq.conf`-style file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let lc = t.to_ascii_lowercase();
    (lc.contains("listeners.tcp") || lc.contains("listeners.ssl"))
        || lc.contains("management.tcp")
        || (lc.contains("default_user") && lc.contains("default_pass"))
        || lc.contains("cluster_formation")
        || (lc.contains("vm_memory") && lc.contains("disk_free_limit"))
}

impl Rabbitmq {
    /// Count key categories in a `rabbitmq.conf`. Returns `None` when the
    /// input does not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            listener_keys: 0,
            auth_keys: 0,
            memory_keys: 0,
            cluster_keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with('#') || tr.starts_with("%%") || tr.starts_with('!') {
                c.comments += 1;
                continue;
            }
            let Some(eq) = tr.find('=') else { continue };
            let key = tr[..eq].trim().to_ascii_lowercase();
            if key.is_empty() {
                continue;
            }
            c.keys += 1;
            if LISTENER_KEYS.iter().any(|k| key.starts_with(k)) {
                c.listener_keys += 1;
            }
            if AUTH_KEYS.iter().any(|k| key.starts_with(k)) {
                c.auth_keys += 1;
            }
            if MEMORY_KEYS.iter().any(|k| key.starts_with(k)) {
                c.memory_keys += 1;
            }
            if CLUSTER_KEYS.iter().any(|k| key.starts_with(k)) {
                c.cluster_keys += 1;
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
        let b = b"# rabbit\nlisteners.tcp.default = 5672\nlisteners.ssl.default = 5671\nmanagement.tcp.port = 15672\ndefault_user = admin\ndefault_pass = secret\nloopback_users.guest = false\nauth_backends.1 = rabbit_auth_backend_internal\nvm_memory_high_watermark.relative = 0\ndisk_free_limit.absolute = 50MB\ncluster_formation.peer_discovery_backend = rabbit_peer_discovery_classic_config\ncluster_formation.classic_config.nodes.1 = rabbit@n1\ncluster.name = prod\n";
        assert!(detect(b));
        let c = Rabbitmq::parse(b).unwrap();
        assert_eq!(c.keys, 12);
        assert_eq!(c.comments, 1);
        assert_eq!(c.listener_keys, 2);
        assert_eq!(c.auth_keys, 4);
        assert_eq!(c.memory_keys, 2);
        assert_eq!(c.cluster_keys, 3);
    }

    #[test]
    fn rejects_random_conf() {
        assert!(!detect(b"name = thing\nport = 9000\n"));
        assert!(Rabbitmq::parse(b"a = b\n").is_none());
    }
}
