//! Sonic `config.cfg` parser.
//!
//! Detects the Sonic search backend config by `[channel]`/`[store]`/
//! `[store.kv]`/`[store.fst]`/`[server]` sections plus `inet`/
//! `auth_password`/`query_alternate_terms`/`suggest_limit`/`retain` keys,
//! and counts structure.
//!
//! ```
//! let b = b"[server]\nlog_level = \"info\"\n[channel]\ninet = \"0.0.0.0:1491\"\ntcp_timeout = 300\n[channel.search]\nquery_alternate_terms_limit = 2\n[store]\n[store.kv]\npath = \"data/store/kv/\"\n[store.fst]\npath = \"data/store/fst/\"\n";
//! assert!(izanagi_kit::soniccfg::detect(b));
//! let c = izanagi_kit::soniccfg::Sonic::parse(b).unwrap();
//! assert_eq!(c.sections, 6);
//! ```

/// Parsed config.cfg summary.
#[derive(Debug, Clone)]
pub struct Sonic {
    /// Recognized key occurrences.
    pub keys: usize,
    /// `[section]` header lines.
    pub sections: usize,
    /// `key = value` lines.
    pub assignments: usize,
    /// `#`/`;` comment lines.
    pub comments: usize,
}

/// Known sections.
const SECTION_KEYS: &[&str] = &[
    "[channel]",
    "[channel.search]",
    "[store]",
    "[store.kv]",
    "[store.fst]",
    "[server]",
];

/// Known option keys.
const OPTION_KEYS: &[&str] = &[
    "inet",
    "auth_password",
    "tcp_timeout",
    "query_alternate_terms_limit",
    "query_alternate_terms",
    "suggest_limit",
    "suggest_results",
    "results_limit",
    "retain",
    "pool_size",
    "path",
    "daemonize",
    "log_level",
    "backup",
    "consolidate",
    "store",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(k)
}

/// Detect a Sonic config.cfg.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = SECTION_KEYS
        .iter()
        .chain(OPTION_KEYS.iter())
        .filter(|k| key_present(t, k))
        .count();
    hits >= 2
}

impl Sonic {
    /// Count categories. Returns `None` when the input does not look like
    /// a config.cfg.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            sections: 0,
            assignments: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') || tr.starts_with(';') {
                c.comments += 1;
            } else if tr.starts_with('[') && tr.ends_with(']') {
                c.sections += 1;
            } else if tr.contains('=') {
                c.assignments += 1;
            }
        }
        for k in SECTION_KEYS {
            c.keys += t.matches(k).count();
        }
        for k in OPTION_KEYS {
            c.keys += t.matches(k).count();
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"[server]\nlog_level = \"info\"\n[channel]\ninet = \"0.0.0.0:1491\"\ntcp_timeout = 300\n[channel.search]\nquery_alternate_terms_limit = 2\nsuggest_limit = 4\n[store]\n[store.kv]\npath = \"data/store/kv/\"\nretain = 60\n[store.fst]\npath = \"data/store/fst/\"\n";
        assert!(detect(b));
        let c = Sonic::parse(b).unwrap();
        assert_eq!(c.sections, 6);
        assert_eq!(c.assignments, 8);
        assert!(c.keys >= 10);
    }

    #[test]
    fn rejects_ini() {
        assert!(!detect(b"[main]\nfoo = bar\n"));
        assert!(Sonic::parse(b"").is_none());
    }
}
