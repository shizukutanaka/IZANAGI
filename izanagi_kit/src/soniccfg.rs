//! Sonic 検索バックエンド `config.cfg` の検出と構造カウント。
//!
//! `[channel]`/`[server]`/`[store]`/`[store.kv]`/`[store.fst]` セクション +
//! `inet`/`tcp`/`auth_password`/`query_limit_default` 等の既知キー。
//!
//! ```
//! let c = izanagi_kit::soniccfg::parse(
//!     b"[channel]\ninet = \"0.0.0.0:1491\"\n[channel.search]\nquery_limit_default = 10\n").unwrap();
//! assert_eq!(c.sections, 2);
//! assert!(izanagi_kit::soniccfg::detect(b"[server]\nlog_level = \"info\"\n"));
//! ```

/// 既知セクション。
const SECTIONS: &[&str] = &[
    "channel",
    "channel.inet",
    "channel.search",
    "channel.tcp",
    "server",
    "store",
    "store.fst",
    "store.fst.graph",
    "store.fst.pool",
    "store.kv",
    "store.kv.database",
    "store.kv.pool",
];
/// 既知キー。
const KEYS: &[&str] = &[
    "auth_password",
    "cleanup_interval",
    "consolidate_after",
    "database",
    "fst",
    "inet",
    "kv",
    "log_level",
    "path",
    "pool",
    "query_alternate_terms_default",
    "query_limit_default",
    "search_query_limit_default",
    "store",
    "suggest_limit_default",
    "tcp",
    "timeout",
    "tcp_idle_ttl",
];

/// Sonic 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `[section]`。
    pub sections: usize,
    /// 既知キー行。
    pub options: usize,
    /// `#`/`;` コメント。
    pub comments: usize,
    /// 分類不能行。
    pub misc: usize,
}

/// b が config.cfg かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let mut secs = 0;
    let mut opts = 0;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') && SECTIONS.contains(&&t[1..t.len() - 1]) {
            secs += 1;
        } else if t.find('=').is_some_and(|p| KEYS.contains(&t[..p].trim())) {
            opts += 1;
        }
    }
    secs >= 1 && opts >= 1
}

/// config.cfg の構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        options: 0,
        comments: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') || t.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if t.starts_with('[') && t.ends_with(']') {
            if SECTIONS.contains(&&t[1..t.len() - 1]) {
                c.sections += 1;
            } else {
                c.misc += 1;
            }
            continue;
        }
        let Some(pos) = t.find('=') else {
            c.misc += 1;
            continue;
        };
        if KEYS.contains(&t[..pos].trim()) {
            c.options += 1;
        } else {
            c.misc += 1;
        }
    }
    (c.sections >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"[channel]\ninet = \"0.0.0.0:1491\"\ntcp_idle_ttl = 300\n\n[channel.search]\nquery_limit_default = 10\nquery_alternate_terms_default = 5\nsuggest_limit_default = 5\n\n[server]\nlog_level = \"info\"\n\n[store]\ndatabase.path = \"./data/\"\n\n[store.kv]\npath = \"./data/kv/\"\nretain_word_objects = 1000\n\n[store.fst]\npath = \"./data/fst/\"\n";

    #[test]
    fn soniccfg() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 6);
        assert_eq!(c.options, 8);
        assert_eq!(c.misc, 2);
    }

    #[test]
    fn not_sonic() {
        assert!(!detect(b"[main]\nkey = v\n"));
        assert!(!detect(b"hello\n"));
    }
}
