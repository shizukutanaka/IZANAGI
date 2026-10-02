//! Postal メールプラットフォーム `postal.yml` の検出と構造カウント。
//!
//! `web`/`smtp`/`general`/`rails`/`main_db`/`message_db`/`rabbitmq`/
//! `dns`/`smtp_server`/`logging`/`spam`/`clamav`/`smtp_relays` 等の
//! 既知トップレベルセクション + ネストされた `host:`/`port:`/`username:` 等で
//! 識別する。
//!
//! ```
//! let c = izanagi_kit::postalconf::parse(
//!     b"web:\n  host: postal.example.com\n  protocol: https\nsmtp_server:\n  port: 25\n").unwrap();
//! assert_eq!(c.sections, 2);
//! assert!(izanagi_kit::postalconf::detect(b"rabbitmq:\n  host: 127.0.0.1\n  port: 5672\n"));
//! ```

/// 既知トップレベルセクション。
const SECTIONS: &[&str] = &[
    "clamav",
    "dns",
    "general",
    "gelf",
    "grpc",
    "legacy_ip_pool",
    "logging",
    "main_db",
    "message_db",
    "migration_server",
    "monitoring",
    "prometheus",
    "rabbitmq",
    "rails",
    "rspamd",
    "smtp",
    "smtp_client",
    "smtp_relays",
    "smtp_server",
    "spam",
    "web",
    "web_server",
    "worker",
];
/// ネストされた既知リーフキー。
const LEAVES: &[&str] = &[
    "affinity",
    "allowed_config_files",
    "base_url",
    "bezier_curves",
    "cert",
    "client_id",
    "cluster_id",
    "command",
    "config",
    "default_bind_address",
    "default_health_server",
    "default_pool",
    "domain",
    "dsn",
    "environment",
    "from",
    "gelf",
    "greylist",
    "health_server_port",
    "host",
    "hostname",
    "ip_pools",
    "key",
    "log_destination",
    "log_level",
    "max_message_size",
    "mdb_encryption",
    "message_retention_days",
    "network",
    "password",
    "path",
    "port",
    "private_key",
    "proto",
    "proxy",
    "raw_message_retention_days",
    "reconnect_on_database_error",
    "return_path",
    "secret",
    "signing_key_path",
    "spam_checks",
    "ssl",
    "suppression_list_removal_delay",
    "threads",
    "tls_mode",
    "tokens",
    "url",
    "use_ip_pools",
    "username",
    "vhost",
    "web_hostname",
    "workers",
];

/// Postal 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知トップレベルセクション行(`name:`、非インデント)。
    pub sections: usize,
    /// ネスト `key: value` 既知代入。
    pub options: usize,
    /// `#` コメント行。
    pub comments: usize,
    /// `- ` リスト項目。
    pub items: usize,
    /// 分類不能行。
    pub misc: usize,
}

/// 行が YAML の `key:` または `key: value` か(キーを返す)。
fn yaml_key(t: &str) -> Option<&str> {
    let p = t.find(':')?;
    let k = t[..p].trim();
    if k.is_empty()
        || !k
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        None
    } else {
        Some(k)
    }
}

/// b が postal.yml かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let mut hits = 0;
    for line in text.lines() {
        let t = line.trim();
        let indent = line.len() - line.trim_start().len();
        if let Some(k) = yaml_key(t) {
            if indent == 0 && SECTIONS.contains(&k) || indent > 0 && LEAVES.contains(&k) {
                hits += 1;
            }
        }
    }
    hits >= 2
}

/// 構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        options: 0,
        comments: 0,
        items: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if t.starts_with("- ") || t == "-" {
            c.items += 1;
            continue;
        }
        if let Some(k) = yaml_key(t) {
            let indent = line.len() - line.trim_start().len();
            if indent == 0 {
                c.sections += 1;
            } else if LEAVES.contains(&k)
                || !t[t.find(':').map_or(t.len(), |p| p + 1)..]
                    .trim()
                    .is_empty()
            {
                c.options += 1;
            } else {
                c.sections += 1;
            }
            continue;
        }
        c.misc += 1;
    }
    (c.sections >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# postal\nweb:\n  host: postal.example.com\n  protocol: https\n  fast_server: false\nsmtp_server:\n  port: 25\n  tls_mode: STARTTLS\nrabbitmq:\n  host: 127.0.0.1\n  username: postal\n  password: secret\n";

    #[test]
    fn postalconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.options, 8);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn not_postal() {
        assert!(!detect(b"foo:\n  bar: baz\n"));
        assert!(!detect(b"hello\n"));
    }
}
