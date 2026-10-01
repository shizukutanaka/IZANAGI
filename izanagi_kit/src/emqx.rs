//! EMQX `emqx.conf` (HOCON) parser.
//!
//! Detects EMQX's `key = value` / `key { }` HOCON configuration
//! (`listeners.tcp`, `listeners.ssl`, `mqtt`, `dashboard`, `authentication`,
//! `authorization`, `node.name`, `cluster.name`, `zones` …) and counts
//! assignments by category: listeners, node/cluster, and MQTT/session limits.
//!
//! ```
//! let b = b"listeners.tcp.default {\n  bind = 1883\n}\ndashboard {\n  listeners.http {\n    bind = 18083\n  }\n}\nnode.name = \"emqx@127\"\n";
//! assert!(izanagi_kit::emqx::detect(b));
//! let c = izanagi_kit::emqx::Emqx::parse(b).unwrap();
//! assert!(c.listener_keys >= 1);
//! ```

/// Parsed EMQX config summary.
#[derive(Debug, Clone)]
pub struct Emqx {
    /// `key = value`/`key = {` assignment lines.
    pub kv_pairs: usize,
    /// Listener keys (`listeners.*`/`bind`/`max_connections`/`acceptors`/`tcp_options`/`ssl_options`/`ws_options`/`proxy_protocol`/`access_rules`).
    pub listener_keys: usize,
    /// Node & cluster keys (`node.*`/`cluster.*`/`rpc.*`).
    pub node_keys: usize,
    /// MQTT/session keys (`mqtt.*`/`max_packet_size`/`max_qos_allowed`/`retain_available`/`shared_subscription*`/`wildcard_subscription`/`upgrade_qos`/`retry_interval`/`await_rel_timeout`/`session_expiry_interval`/`keepalive*`).
    pub mqtt_keys: usize,
    /// `{`-section blocks (`authentication`/`authorization`/`exhook`/`gateway`/`bridges`/`rule_engine`/`connectors`/`dashboard`/`log`/`alarm`/`monitor`/`telemetry`).
    pub blocks: usize,
    /// `#`/`##`/`//`/`%%` comment lines.
    pub comments: usize,
}

/// Listener-side keys (prefix match on dotted key or last segment).
const LISTENER_KEYS: &[&str] = &[
    "listeners.",
    "bind",
    "max_connections",
    "acceptors",
    "tcp_options",
    "ssl_options",
    "ws_options",
    "wss_options",
    "proxy_protocol",
    "proxy_address",
    "access_rules",
    "limiter",
    "zone",
    "mountpoint",
    "rate_limit",
];

/// Node & cluster keys (prefix match).
const NODE_KEYS: &[&str] = &["node.", "cluster.", "rpc."];

/// MQTT/session keys (prefix match).
const MQTT_KEYS: &[&str] = &[
    "mqtt.",
    "max_packet_size",
    "max_qos_allowed",
    "retain_available",
    "shared_subscription",
    "wildcard_subscription",
    "upgrade_qos",
    "retry_interval",
    "await_rel_timeout",
    "session_expiry_interval",
    "keepalive",
    "use_username_as_clientid",
    "strict_mode",
    "peer_cert_as_username",
    "peer_cert_as_clientid",
];

/// Sections that open a `{`-block.
const BLOCKS: &[&str] = &[
    "authentication",
    "authorization",
    "exhook",
    "gateway",
    "bridges",
    "rule_engine",
    "connectors",
    "dashboard",
    "log",
    "alarm",
    "monitor",
    "telemetry",
    "sysmon",
    "overload_protection",
    "prometheus",
    "opentelemetry",
    "persistent_session_store",
];

fn first_word(tr: &str) -> &str {
    tr.split(|c: char| c.is_ascii_whitespace() || c == '=' || c == '{')
        .next()
        .unwrap_or("")
}

/// Detect an `emqx.conf`-style file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("listeners.tcp")
        || t.contains("listeners.ssl")
        || t.contains("listeners.ws")
        || t.contains("listeners.quic")
        || t.contains("node.name")
        || t.contains("cluster.name")
        || t.contains("dashboard") && (t.contains("bind") || t.contains("listeners.http"))
        || t.contains("mqtt {")
        || t.contains("mqtt=")
        || t.contains("persistent_session_store")
}

impl Emqx {
    /// Count key categories in an `emqx.conf`. Returns `None` when the input
    /// does not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            kv_pairs: 0,
            listener_keys: 0,
            node_keys: 0,
            mqtt_keys: 0,
            blocks: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() || tr == "}" || tr == "{" {
                continue;
            }
            if tr.starts_with('#') || tr.starts_with("//") || tr.starts_with("%%") {
                c.comments += 1;
                continue;
            }
            let w = first_word(tr);
            if w.is_empty() {
                continue;
            }
            let opens = tr.contains('{') && (tr.ends_with('{') || tr.matches('{').count() == 1);
            if BLOCKS.contains(&w) && opens {
                c.blocks += 1;
                continue;
            }
            if tr.contains('=') || tr.contains(':') {
                c.kv_pairs += 1;
                let wl = w.strip_suffix('.').unwrap_or(w);
                if LISTENER_KEYS.iter().any(|k| wl.starts_with(k)) || w.starts_with("listeners.") {
                    c.listener_keys += 1;
                }
                if NODE_KEYS.iter().any(|k| w.starts_with(k)) {
                    c.node_keys += 1;
                }
                if MQTT_KEYS.iter().any(|k| wl.starts_with(k)) {
                    c.mqtt_keys += 1;
                }
            } else if opens {
                c.blocks += 1;
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
        let b = b"## emqx\nlisteners.tcp.default {\n  bind = 1883\n  max_connections = 1024000\n  tcp_options {\n    backlog = 512\n  }\n}\nlisteners.ssl.default {\n  bind = 8883\n  ssl_options {\n    certfile = /cert.pem\n  }\n}\nnode.name = \"emqx@127\"\nnode.cookie = secret\ncluster.name = emqxcl\nmqtt {\n  max_packet_size = 1MB\n  retain_available = true\n}\ndashboard {\n  listeners.http {\n    bind = 18083\n  }\n}\n";
        assert!(detect(b));
        let c = Emqx::parse(b).unwrap();
        assert_eq!(c.comments, 1);
        assert_eq!(c.blocks, 7);
        assert_eq!(c.node_keys, 3);
        assert!(c.listener_keys >= 4);
        assert!(c.mqtt_keys >= 1);
    }

    #[test]
    fn rejects_json() {
        assert!(!detect(br#"{"a":1}"#));
        assert!(Emqx::parse(b"x = y\n").is_none());
    }
}
