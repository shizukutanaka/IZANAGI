//! Prosody `prosody.cfg.lua` — `key = value`/`key = { list; }` Lua 風代入 +
//! `VirtualHost "x"`/`Component "x" "type"` 宣言の XMPP サーバ設定。
//!
//! ```
//! let cfg = b"admins = { \"admin@example.net\" }\nmodules_enabled = {\n    \"roster\";\n    \"tls\";\n}\nallow_registration = false\ndaemonize = true\nVirtualHost \"example.net\"\n    enabled = true\nComponent \"conference.example.net\" \"muc\"\n    restrict_room_creation = true\n";
//! assert!(izanagi_kit::prosody::detect(cfg));
//! let c = izanagi_kit::prosody::parse(cfg).unwrap();
//! assert_eq!(c.virtualhosts, 1);
//! assert_eq!(c.components, 1);
//! ```

/// Prosody の既知設定キー (完全一致の代表)。
const KNOWN_KEYS: &[&str] = &[
    "admins",
    "modules_enabled",
    "modules_disabled",
    "modules",
    "allow_registration",
    "daemonize",
    "c2s_require_encryption",
    "s2s_require_encryption",
    "s2s_secure_auth",
    "s2s_secure_domains",
    "s2s_insecure_domains",
    "authentication",
    "storage",
    "sql",
    "log",
    "pidfile",
    "ports",
    "ssl_ports",
    "interfaces",
    "http_ports",
    "http_interfaces",
    "https_ports",
    "data_path",
    "plugin_paths",
    "certificates",
    "statistics",
    "stats",
    "limits",
    "c2s_tcp_keepalives",
    "s2s_tcp_keepalives",
    "registration_whitelist",
    "whitelist",
    "blacklist",
    "trusted_proxies",
    "proxy65_address",
    "trusted_networks",
    "allow_anonymous_s2s",
    "core_http_running",
    "plugin_installer",
    "trusted_paths",
];

/// 走査結果の集計。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 物理行数(空行除く)。
    pub lines: usize,
    /// `key = value`/`key = {` 代入行数。
    pub assignments: usize,
    /// `key = {` リスト代入数。
    pub list_assignments: usize,
    /// 既知キーの代入数。
    pub known_assignments: usize,
    /// `VirtualHost "x"` 宣言数。
    pub virtualhosts: usize,
    /// `Component "x" "type"` 宣言数。
    pub components: usize,
    /// `--` コメント行数。
    pub comments: usize,
}

/// `VirtualHost "…"` または `Component "…" ["type"]` 行か。
fn declaration(t: &str, head: &str) -> bool {
    let Some(rest) = t.strip_prefix(head) else {
        return false;
    };
    rest.trim_start().starts_with('"')
}

/// prosody.cfg.lua らしい構造かを判定する。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Some(c) = parse(b) else {
        return false;
    };
    (c.virtualhosts + c.components >= 1 && c.assignments >= 2) || c.known_assignments >= 4
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
        assignments: 0,
        list_assignments: 0,
        known_assignments: 0,
        virtualhosts: 0,
        components: 0,
        comments: 0,
    };
    for raw in s.lines() {
        let t = raw.trim();
        if t.is_empty() {
            continue;
        }
        c.lines += 1;
        if t.starts_with("--") {
            c.comments += 1;
            continue;
        }
        if declaration(t, "VirtualHost") {
            c.virtualhosts += 1;
            continue;
        }
        if declaration(t, "Component") {
            c.components += 1;
            continue;
        }
        if let Some((key, rhs)) = t.split_once('=') {
            let key = key.trim();
            let rhs = rhs.trim_start();
            if key.is_empty()
                || !key
                    .bytes()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == b'_' || ch == b'.')
            {
                continue;
            }
            // `==` 比較演算子は除外。
            if rhs.starts_with('=') {
                continue;
            }
            c.assignments += 1;
            if rhs.starts_with('{') {
                c.list_assignments += 1;
            }
            if KNOWN_KEYS.contains(&key) {
                c.known_assignments += 1;
            }
        }
    }
    if c.assignments == 0 && c.virtualhosts + c.components == 0 {
        return None;
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"-- prosody\nadmins = { \"admin@example.net\" }\nmodules_enabled = {\n    \"roster\";\n    \"saslauth\";\n    \"tls\";\n}\nmodules_disabled = {\n    -- \"offline\";\n}\nallow_registration = false\ndaemonize = true\nc2s_require_encryption = true\nauthentication = \"internal_hashed\"\nVirtualHost \"example.net\"\n    enabled = true\n    ssl = {\n        key = \"/etc/prosody/k.pem\";\n        certificate = \"/etc/prosody/c.pem\";\n    }\nComponent \"conference.example.net\" \"muc\"\n    restrict_room_creation = true\n";

    #[test]
    fn detects_prosody() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.virtualhosts, 1);
        assert_eq!(c.components, 1);
        assert_eq!(c.list_assignments, 4);
        assert_eq!(c.known_assignments, 7);
    }

    #[test]
    fn rejects_lua() {
        assert!(!detect(b"print(\"hello\")\nif a == b then end\n"));
        assert!(!detect(b"x = 1\ny = 2\n"));
    }
}
