//! Zulip `/etc/zulip/zulip.conf` — `[machine]`/`[postgresql]`/`[memcached]`/
//! `[rabbitmq]`/`[application_server]` 等セクションの INI 形式。
//!
//! ```
//! let cfg = b"[machine]\npuppet_classes = zulip::profile::standalone\ndeploy_type = production\n[postgresql]\nversion = 14\n[memcached]\n[rabbitmq]\nnodename = zulip@localhost\n[application_server]\nhttp_only = false\n";
//! assert!(izanagi_kit::zulipconf::detect(cfg));
//! let c = izanagi_kit::zulipconf::parse(cfg).unwrap();
//! assert_eq!(c.sections, 5);
//! assert_eq!(c.known_sections, 5);
//! ```

/// Zulip zulip.conf の既知セクション名。
const KNOWN_SECTIONS: &[&str] = &[
    "machine",
    "postgresql",
    "memcached",
    "rabbitmq",
    "application_server",
    "sentry",
    "nagios",
    "loadbalancer",
    "queue",
    "help_center",
    "deploy_options",
    "grooming",
];

/// Zulip の既知設定キー (代表)。
const KNOWN_KEYS: &[&str] = &[
    "puppet_classes",
    "deploy_type",
    "version",
    "nodename",
    "http_only",
    "rate_limiting",
    "rate_limiting_rules",
    "s3_key",
    "s3_secret_key",
    "s3_region",
    "s3_bucket",
    "s3_endpoint",
    "gravatar_proxy",
    "remote_postgres_host",
    "remote_postgres_port",
    "remote_postgres_sslmode",
    "missing_dictionaries",
    "nginx_listen_port",
    "use_mandrill",
    "loadbalancer_",
    "private_name",
    "username",
    "password",
    "gateway",
    "num_multiprocess_workers",
];

/// 走査結果の集計。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 物理行数(空行除く)。
    pub lines: usize,
    /// `[name]` セクション数。
    pub sections: usize,
    /// 既知名のセクション数。
    pub known_sections: usize,
    /// `key = value` 行数。
    pub entries: usize,
    /// 既知キーの行数。
    pub known_keys: usize,
    /// `#`/`;` コメント行数。
    pub comments: usize,
}

/// zulip.conf らしい構造かを判定する。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Some(c) = parse(b) else {
        return false;
    };
    c.known_sections >= 2 && c.entries >= 2
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
        sections: 0,
        known_sections: 0,
        entries: 0,
        known_keys: 0,
        comments: 0,
    };
    for raw in s.lines() {
        let t = raw.trim();
        if t.is_empty() {
            continue;
        }
        c.lines += 1;
        if t.starts_with('#') || t.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if t.starts_with('[') && t.ends_with(']') && t.len() > 2 {
            let name = &t[1..t.len() - 1];
            if name
                .bytes()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == b'_')
            {
                c.sections += 1;
                if KNOWN_SECTIONS.contains(&name) {
                    c.known_sections += 1;
                }
                continue;
            }
        }
        if let Some((key, _)) = t.split_once('=') {
            let key = key.trim();
            if !key.is_empty()
                && key
                    .bytes()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == b'_')
            {
                c.entries += 1;
                if KNOWN_KEYS.contains(&key) {
                    c.known_keys += 1;
                }
            }
        }
    }
    if c.sections == 0 && c.entries == 0 {
        return None;
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# zulip.conf\n[machine]\npuppet_classes = zulip::profile::standalone\ndeploy_type = production\n[postgresql]\nversion = 14\n[memcached]\n[rabbitmq]\nnodename = zulip@localhost\n[application_server]\nhttp_only = false\nrate_limiting = true\n[sentry]\n";

    #[test]
    fn detects_zulipconf() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 6);
        assert_eq!(c.known_sections, 6);
        assert_eq!(c.entries, 6);
        assert_eq!(c.known_keys, 6);
    }

    #[test]
    fn rejects_ini() {
        assert!(!detect(b"[ui]\nx = 1\n[font]\nsize = 3\n"));
        assert!(!detect(b"[machine]\nx = 1\n"));
    }
}
