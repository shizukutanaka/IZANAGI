//! VerneMQ 設定(`vernemq.conf`、cuttlefish `key = value`)の検出と構造カウント。
//!
//! `listener.*`/`allow_*`/`max_*`/`queue_*`/`persistent_*`/`vmq_*`/
//! `plugins.*`/`erlang.*`/`log.*`/`leveldb.*`/`distributed_cookie`/`nodename`/
//! `shared_subscription_*`/`message_size_limit`/`graphite_*` 等の
//! ドット区切り既知キーを識別する。
//!
//! ```
//! let c = izanagi_kit::vernemq::parse(
//!     b"listener.tcp.default = 127.0.0.1:1883\nallow_anonymous = on\nmax_connections = 10000\n").unwrap();
//! assert_eq!(c.options, 3);
//! assert!(izanagi_kit::vernemq::detect(b"listener.tcp.default = 127.0.0.1:1883\nallow_anonymous = on\nvmq_acl.acl_file = /etc/vernemq/vmq.acl\n"));
//! ```

use crate::textutil::strip_bom;
/// 既知キー接頭辞(`<prefix>` または `<prefix><何か>` 完全一致系含む)。
const PREFIXES: &[&str] = &[
    "allow_anonymous",
    "allow_multiple_sessions",
    "allow_publish_during_netsplit",
    "allow_register_during_netsplit",
    "allow_subscribe_during_netsplit",
    "allow_unsubscribe_during_netsplit",
    "coordinate_registrations",
    "default_reg_view",
    "distributed_cookie",
    "erlang.",
    "graphite_",
    "leveldb.",
    "listener.",
    "log.",
    "max_client_id_size",
    "max_connections",
    "max_drain_time",
    "max_inflight_messages",
    "max_msgs_per_drain_session",
    "max_offline_messages",
    "max_online_messages",
    "message_size_limit",
    "metadata_plugin",
    "nodename",
    "out_queue_cluster_node",
    "persistent_client_expiration",
    "plugins.",
    "queue_deliver_mode",
    "queue_type",
    "reg_view",
    "retry_interval",
    "server_keepalive",
    "shared_subscription_policy",
    "shared_subscription_timeout_action",
    "systree",
    "sysmon.",
    "upgrade_outgoing_qos",
    "vmq_acl.",
    "vmq_bridge.",
    "vmq_diversity.",
    "vmq_elastic_search.",
    "vmq_passwd.",
    "vmq_plumtree.",
    "vmq_pulse.",
    "vmq_swl.",
    "vmq_webhooks.",
];

/// vernemq 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知プロパティ行。
    pub options: usize,
    /// `##`/`#`/`%` コメント行。
    pub comments: usize,
    /// 分類不能行(未知キー・その他)。
    pub misc: usize,
}

/// `key =` 前のキー名。
fn kv_key(t: &str) -> Option<&str> {
    let p = t.find('=')?;
    let k = t[..p].trim();
    if k.is_empty()
        || !k
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        None
    } else {
        Some(k)
    }
}

/// 既知キーかどうか(接頭辞 `xxx.` 一致 or 完全一致)。
fn known_key(k: &str) -> bool {
    PREFIXES.iter().any(|p| {
        if p.ends_with('.') {
            k.starts_with(*p)
        } else {
            k == *p || k.starts_with(*p) && k.as_bytes().get(p.len()) == Some(&b'.')
        }
    })
}

/// b が vernemq.conf かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let text = strip_bom(text);
    text.lines()
        .filter(|l| {
            let t = l.trim();
            !t.is_empty()
                && !t.starts_with('#')
                && !t.starts_with('%')
                && kv_key(t).is_some_and(known_key)
        })
        .count()
        >= 2
}

/// 構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let text = strip_bom(text);
    let mut c = Counts {
        options: 0,
        comments: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') || t.starts_with('%') {
            c.comments += 1;
            continue;
        }
        if kv_key(t).is_some_and(known_key) {
            c.options += 1;
        } else {
            c.misc += 1;
        }
    }
    (c.options >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"## vernemq\nlistener.tcp.default = 127.0.0.1:1883\nlistener.ws.default = 127.0.0.1:8080\nlistener.vmq.clustering = 0.0.0.0:44053\nallow_anonymous = on\nmax_connections = 10000\nmax_inflight_messages = 20\npersistent_client_expiration = never\nvmq_acl.acl_file = /etc/vernemq/vmq.acl\nvmq_passwd.password_file = /etc/vernemq/vmq.passwd\nnodename = VerneMQ@127.0.0.1\ndistributed_cookie = vmq\nmessage_size_limit = 0\n";

    #[test]
    fn vernemq() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.options, 12);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn not_vernemq() {
        assert!(!detect(b"key=value\nother=thing\n"));
        assert!(!detect(b"listener.tcp.default = x\n"));
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
