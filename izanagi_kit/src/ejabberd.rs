//! ejabberd `ejabberd.yml` — `hosts:`/`listen:`/`modules:`/`acl:` 等トップキー
//! と `mod_*` モジュール項目を持つ YAML 設定。
//!
//! ```
//! let cfg = b"hosts:\n  - example.net\nlisten:\n  -\n    port: 5222\n    module: ejabberd_c2s\nacl:\n  admin:\n    user:\n      - \"admin@example.net\"\nmodules:\n  mod_muc:\n    host: \"conference.@HOST@\"\n  mod_last: {}\n";
//! assert!(izanagi_kit::ejabberd::detect(cfg));
//! let c = izanagi_kit::ejabberd::parse(cfg).unwrap();
//! assert_eq!(c.known_tops, 4);
//! assert_eq!(c.mod_items, 2);
//! ```

/// ejabberd の既知トップレベルキー。
const KNOWN_TOPS: &[&str] = &[
    "hosts",
    "listen",
    "acl",
    "access_rules",
    "shaper",
    "shaper_rules",
    "modules",
    "certfiles",
    "loglevel",
    "registration_timeout",
    "captcha_limit",
    "auth_method",
    "auth_password_format",
    "default_db",
    "odbc_type",
    "odbc_server",
    "s2s_use_starttls",
    "s2s_certfile",
    "tls_protocol",
    "ciphers",
    "api_permissions",
    "oauth_expire",
    "ext_api_url",
    "ext_mod",
    "sql_type",
    "sql_server",
    "sql_database",
    "sql_username",
    "sql_ssl",
    "sm_db_type",
    "sm_host",
    "host_config",
    "append_host_config",
    "define_macro",
    "acme",
    "disable_local_options",
    "define_keyword",
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
    /// `mod_*:` モジュール項目数。
    pub mod_items: usize,
    /// `-` リスト項目数。
    pub list_items: usize,
    /// `key:`/`key: value` 行数。
    pub entries: usize,
    /// `#` コメント行数。
    pub comments: usize,
}

/// ejabberd.yml らしい構造かを判定する。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Some(c) = parse(b) else {
        return false;
    };
    c.known_tops >= 2 && c.entries >= 5
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
        mod_items: 0,
        list_items: 0,
        entries: 0,
        comments: 0,
    };
    for raw in s.lines() {
        let t = raw.trim_end();
        let tt = t.trim_start();
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
        let key_part = tt.strip_prefix('-').map_or(tt, |rest| rest.trim_start());
        if let Some((key, _)) = key_part.split_once(':') {
            let key = key.trim().trim_matches('"');
            let key_ok = !key.is_empty()
                && key
                    .bytes()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == b'_' || ch == b'-' || ch == b'.');
            if key_ok {
                c.entries += 1;
                if col0 {
                    c.top_keys += 1;
                    if KNOWN_TOPS.contains(&key) {
                        c.known_tops += 1;
                    }
                }
                if key.starts_with("mod_") {
                    c.mod_items += 1;
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

    const SAMPLE: &[u8] = b"hosts:\n  - example.net\nloglevel: 4\nlisten:\n  -\n    port: 5222\n    ip: \"::\"\n    module: ejabberd_c2s\n    starttls: true\n  -\n    port: 5269\n    module: ejabberd_s2s_in\nacl:\n  admin:\n    user:\n      - \"admin@example.net\"\nshaper_rules:\n  max_user_sessions: 10\nmodules:\n  mod_muc:\n    host: \"conference.@HOST@\"\n    history_size: 50\n  mod_mam:\n    default: always\n  mod_last: {}\n";

    #[test]
    fn detects_ejabberd() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.known_tops, 6);
        assert_eq!(c.mod_items, 3);
        assert_eq!(c.list_items, 4);
    }

    #[test]
    fn rejects_other_yaml() {
        assert!(!detect(b"key:\n  sub: 1\n  sub2: 2\nlist:\n  - a\n  - b\n"));
        assert!(!detect(b"name: x\nvalue: y\n"));
    }
}
