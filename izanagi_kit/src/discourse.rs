//! Discourse `containers/app.yml` — `templates:`/`expose:`/`params:`/`env:`/
//! `volumes:`/`hooks:` トップキーを持つ YAML コンテナ定義。
//!
//! ```
//! let cfg = b"templates:\n  - \"templates/postgres.template.yml\"\n  - \"templates/redis.template.yml\"\nexpose:\n  - \"80:80\"\nenv:\n  DISCOURSE_HOSTNAME: 'discourse.example.com'\n  UNICORN_WORKERS: 4\nvolumes:\n  - volume:\n      host: /var/discourse/shared\n      guest: /shared\nhooks:\n  after_code:\n    - exec:\n        cd: $home/plugins\n";
//! assert!(izanagi_kit::discourse::detect(cfg));
//! let c = izanagi_kit::discourse::parse(cfg).unwrap();
//! assert_eq!(c.known_tops, 5);
//! ```

/// Discourse app.yml の既知トップレベルキー。
const KNOWN_TOPS: &[&str] = &[
    "templates",
    "expose",
    "params",
    "env",
    "volumes",
    "hooks",
    "links",
    "run",
    "labels",
    "docker_args",
];

/// Discourse 固有の環境変数プレフィックス。
const ENV_PREFIXES: &[&str] = &["DISCOURSE_", "UNICORN_", "db_", "FORCE_", "LETSENCRYPT_"];

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
    /// Discourse 系環境変数名の行数。
    pub discourse_env: usize,
    /// `key:`/`key: value` 行数。
    pub entries: usize,
    /// `#` コメント行数。
    pub comments: usize,
}

/// app.yml らしい構造かを判定する。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Some(c) = parse(b) else {
        return false;
    };
    c.known_tops >= 3 && c.entries >= 5 && (c.discourse_env >= 1 || c.known_tops >= 4)
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
        discourse_env: 0,
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
        let key_part = tt.strip_prefix('-').map_or(tt, |rest| rest.trim_start());
        if let Some((key, _)) = key_part.split_once(':') {
            let key = key.trim().trim_matches('"').trim_matches('\'');
            let key_ok = !key.is_empty()
                && key
                    .bytes()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == b'_' || ch == b'-' || ch == b'.');
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
            if ENV_PREFIXES.iter().any(|p| key.starts_with(p)) {
                c.discourse_env += 1;
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

    const SAMPLE: &[u8] = b"templates:\n  - \"templates/postgres.template.yml\"\n  - \"templates/redis.template.yml\"\n  - \"templates/web.template.yml\"\nexpose:\n  - \"80:80\"\n  - \"443:443\"\n  - \"2222:22\"\nparams:\n  db_default_text_search_config: \"pg_catalog.english\"\n  db_shared_buffers: \"4096MB\"\nenv:\n  LC_ALL: en_US.UTF-8\n  DISCOURSE_HOSTNAME: 'discourse.example.com'\n  DISCOURSE_DEVELOPER_EMAILS: 'ume@example.net'\n  UNICORN_WORKERS: 4\nvolumes:\n  - volume:\n      host: /var/discourse/shared/standalone\n      guest: /shared\nhooks:\n  after_code:\n    - exec:\n        cd: $home/plugins\n        cmd:\n          - git clone https://github.com/discourse/docker_manager.git\n";

    #[test]
    fn detects_discourse() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.known_tops, 6);
        assert_eq!(c.discourse_env, 5);
        assert_eq!(c.list_items, 9);
    }

    #[test]
    fn rejects_other_yaml() {
        assert!(!detect(
            b"services:\n  web:\n    image: nginx\n    ports:\n      - 80:80\n"
        ));
        assert!(!detect(b"templates:\n  - a\n  - b\n"));
    }
}
