//! Matrix Synapse `homeserver.yaml` — `server_name:`/`listeners:`/`database:`/
//! `modules:` 等トップキーを持つ YAML 設定。
//!
//! ```
//! let cfg = b"server_name: \"example.net\"\npid_file: /var/run/synapse.pid\nlisteners:\n  - port: 8448\n    type: http\n    tls: true\ndatabase:\n  name: psycopg2\n  args:\n    user: synapse\nlog_config: \"/etc/synapse/log.yaml\"\nenable_registration: false\n";
//! assert!(izanagi_kit::synapse::detect(cfg));
//! let c = izanagi_kit::synapse::parse(cfg).unwrap();
//! assert_eq!(c.known_tops, 6);
//! ```

/// Synapse homeserver.yaml の既知トップレベルキー。
const KNOWN_TOPS: &[&str] = &[
    "server_name",
    "pid_file",
    "listeners",
    "database",
    "log_config",
    "media_store_path",
    "modules",
    "enable_registration",
    "registration_shared_secret",
    "report_stats",
    "serve_server_wellknown",
    "public_baseurl",
    "signing_key_path",
    "key_refresh_interval",
    "macaroon_secret_key",
    "form_secret",
    "trusted_key_servers",
    "allow_public_rooms_over_federation",
    "allow_guest_access",
    "federation_verify_certificates",
    "turn_uris",
    "turn_shared_secret",
    "email",
    "sentry",
    "opentracing",
    "metrics_flags",
    "redis",
    "user_directory",
    "account_validity",
    "admin_contact",
    "retention",
    "auto_join_rooms",
    "oidc_providers",
    "cas_config",
    "sso",
    "password_config",
    "push",
    "spam_checker",
    "room_prejoin_state",
    "supplemental_key_storage",
    "extra_templates",
    "enable_notifs",
    "enable_metrics",
    "web_client_location",
    "resource_limits",
    "caches",
    "daemonize",
    "print_pidfile",
];

/// 走査結果の集計。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 物理行数(空行除く)。
    pub lines: usize,
    /// 列0 の `key:` トップキー数。
    pub top_keys: usize,
    /// 既知名のトップキー数。
    pub known_tops: usize,
    /// `-` リスト項目数。
    pub list_items: usize,
    /// `key:`/`key: value` 行数。
    pub entries: usize,
    /// `#` コメント行数。
    pub comments: usize,
}

/// homeserver.yaml らしい構造かを判定する。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Some(c) = parse(b) else {
        return false;
    };
    c.known_tops >= 3 && c.entries >= 6
}

/// 行を走査して集計する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    if !b.is_ascii() {
        return None;
    }
    let s = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        lines: 0,
        top_keys: 0,
        known_tops: 0,
        list_items: 0,
        entries: 0,
        comments: 0,
    };
    for raw in s.lines() {
        let tt = raw.trim();
        if tt.is_empty() {
            continue;
        }
        c.lines += 1;
        if tt.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if tt.starts_with('-') {
            c.list_items += 1;
        }
        let col0 = raw.len() == raw.trim_start().len();
        if let Some((key, _)) = tt.split_once(':') {
            let key = key.trim().trim_matches('"');
            let key_ok = !key.is_empty()
                && key
                    .bytes()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == b'_' || ch == b'-');
            if !key_ok {
                continue;
            }
            c.entries += 1;
            if col0 {
                c.top_keys += 1;
                if KNOWN_TOPS.contains(&key) {
                    c.known_tops += 1;
                }
            }
        }
    }
    if c.entries == 0 {
        return None;
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"server_name: \"example.net\"\npid_file: /var/run/synapse.pid\nweb_client_location: https://riot.example.net/\nserve_server_wellknown: true\nlisteners:\n  - port: 8448\n    type: http\n    tls: true\n    resources:\n      - names: [client, federation]\n        compress: false\n  - port: 8008\n    type: http\n    resources:\n      - names: [client]\ndatabase:\n  name: psycopg2\n  args:\n    user: synapse\n    database: synapse\nlog_config: \"/etc/synapse/log.yaml\"\nmedia_store_path: /var/lib/synapse/media\nregistration_shared_secret: \"secret\"\n";

    #[test]
    fn detects_synapse() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.known_tops, 9);
        assert_eq!(c.top_keys, 9);
        assert_eq!(c.list_items, 4);
    }

    #[test]
    fn rejects_other_yaml() {
        assert!(!detect(b"services:\n  web:\n    image: nginx\n"));
        assert!(!detect(b"server_name: x\nfoo: y\n"));
    }
}
