//! Eclipse Mosquitto `mosquitto.conf` parser.
//!
//! Detects the space-separated `option value` configuration style
//! (`listener`, `allow_anonymous`, `persistence`, `log_dest`, `bridge`
//! `connection` …) and counts options by category: listeners, auth/ACL,
//! persistence/logging, and bridging.
//!
//! ```
//! let b = b"per_listener_settings false\nlistener 1883\nallow_anonymous false\npassword_file /etc/mosq/pwfile\npersistence true\npersistence_location /var/lib/mosquitto/\nlog_dest file /var/log/mosq.log\n";
//! assert!(izanagi_kit::mosquitto::detect(b));
//! let c = izanagi_kit::mosquitto::Mosquitto::parse(b).unwrap();
//! assert_eq!(c.keys, 7);
//! ```

/// Parsed mosquitto config summary.
#[derive(Debug, Clone)]
pub struct Mosquitto {
    /// Option lines (non-comment, non-empty).
    pub keys: usize,
    /// Listener keys (`listener`/`port`/`bind_*`/`socket_*`/`protocol`/`mount_point`/`http_dir`/`use_identity_as_username`/`max_*`).
    pub listener_keys: usize,
    /// Auth & ACL keys (`allow_anonymous`/`password_file`/`acl_file`/`plugin`/`auth_plugin`/`per_listener_settings`/`psk_file`).
    pub auth_keys: usize,
    /// Persistence & logging keys (`persistence*`/`autosave*`/`log_*`/`retain_available`/`check_retain_source`/`message_size_limit`).
    pub persist_keys: usize,
    /// Bridge keys (`connection`/`address`/`topic`/`bridge_*`/`remote_*`/`local_*`/`try_private`/`notifications`/`cleansession` — inside `connection` blocks).
    pub bridge_keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Listener & connection options (word match).
const LISTENER_KEYS: &[&str] = &[
    "listener",
    "port",
    "bind_address",
    "bind_interface",
    "socket_domain",
    "protocol",
    "mount_point",
    "http_dir",
    "use_identity_as_username",
    "use_username_as_clientid",
    "max_connections",
    "max_inflight_messages",
    "max_queued_messages",
    "max_queued_bytes",
    "max_packet_size",
    "max_keepalive",
    "websockets_headers_size",
    "certfile",
    "keyfile",
    "cafile",
    "capath",
    "ciphers",
    "crlfile",
    "dhparamfile",
    "tls_engine",
    "tls_version",
    "tls_alpn",
    "require_certificate",
    "use_subject_as_username",
    "allow_zero_length_clientid",
];

/// Auth & ACL options (word match).
const AUTH_KEYS: &[&str] = &[
    "allow_anonymous",
    "password_file",
    "acl_file",
    "plugin",
    "plugin_opt_",
    "auth_plugin",
    "per_listener_settings",
    "psk_file",
    "psk_hint",
];

/// Persistence & logging options (word match).
const PERSIST_KEYS: &[&str] = &[
    "persistence",
    "persistence_file",
    "persistence_location",
    "persistent_client_expiration",
    "autosave_interval",
    "autosave_on_changes",
    "retain_available",
    "check_retain_source",
    "message_size_limit",
    "log_dest",
    "log_type",
    "log_facility",
    "log_timestamp",
    "log_timestamp_format",
    "connection_messages",
    "log_internal_errors",
    "allow_duplicate_messages",
    "queue_qos0_messages",
    "upgrade_outgoing_qos",
    "memory_limit",
    "store_clean_interval",
];

/// Bridge options (word match — only counted inside a `connection` block).
const BRIDGE_KEYS: &[&str] = &[
    "connection",
    "address",
    "addresses",
    "topic",
    "bridge_pattern",
    "bridge_cafile",
    "bridge_capath",
    "bridge_certfile",
    "bridge_keyfile",
    "bridge_insecure",
    "bridge_alpn",
    "bridge_tls_version",
    "remote_address",
    "remote_port",
    "remote_username",
    "remote_password",
    "remote_clientid",
    "local_clientid",
    "local_username",
    "local_password",
    "try_private",
    "notifications",
    "notification_topic",
    "cleansession",
    "start_type",
    "restart_timeout",
    "threshold",
    "idle_timeout",
    "keepalive_interval",
    "max_topic_alias",
    "round_robin",
    "outgoing_retain",
    "bridge_outgoing_retain",
    "bridge_protocol_version",
    "bridge_max_packet_size",
];

fn first_word(tr: &str) -> &str {
    tr.split(char::is_whitespace).next().unwrap_or("")
}

/// Detect a `mosquitto.conf`-style file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let lc = t.to_ascii_lowercase();
    lc.contains("listener ")
        || lc.contains("allow_anonymous ")
        || lc.contains("persistence ")
        || lc.contains("log_dest ")
        || lc.contains("password_file ")
        || lc.contains("per_listener_settings ")
        || (lc.contains("connection ") && lc.contains("topic "))
}

impl Mosquitto {
    /// Count options in a `mosquitto.conf`. Returns `None` when the input
    /// does not look like one.
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
            persist_keys: 0,
            bridge_keys: 0,
            comments: 0,
        };
        let mut in_bridge = false;
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
            if w == "connection" || w == "listener" {
                in_bridge = w == "connection";
                c.bridge_keys += usize::from(in_bridge);
                c.listener_keys += usize::from(!in_bridge);
                continue;
            }
            if in_bridge && BRIDGE_KEYS.contains(&w) {
                c.bridge_keys += 1;
                continue;
            }
            if LISTENER_KEYS.contains(&w) {
                c.listener_keys += 1;
            }
            if AUTH_KEYS.iter().any(|k| w.starts_with(k)) {
                c.auth_keys += 1;
            }
            if PERSIST_KEYS.contains(&w) {
                c.persist_keys += 1;
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
        let b = b"# mosquitto\nper_listener_settings false\nlistener 1883\nprotocol mqtt\nlistener 8080\nprotocol websockets\nallow_anonymous false\npassword_file /etc/mosq/pwfile\nacl_file /etc/mosq/acl\npersistence true\npersistence_location /var/lib/mosquitto/\nautosave_interval 1800\nlog_dest file /var/log/mosq.log\nlog_type error\nconnection cloud\naddress b1:8883\nremote_username u\nremote_password p\ntopic # both 0\n";
        assert!(detect(b));
        let c = Mosquitto::parse(b).unwrap();
        assert_eq!(c.keys, 18);
        assert_eq!(c.comments, 1);
        assert_eq!(c.auth_keys, 4);
        assert_eq!(c.persist_keys, 5);
        assert_eq!(c.bridge_keys, 5);
        assert!(c.listener_keys >= 2);
    }

    #[test]
    fn rejects_yaml() {
        assert!(!detect(b"kind: Pod\nmetadata:\n"));
        assert!(Mosquitto::parse(b"a b\n").is_none());
    }
}
