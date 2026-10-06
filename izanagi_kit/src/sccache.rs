//! sccache 設定（`SCCACHE_CONF` TOML）の検出と構造カウント。
//!
//! `[cache.*]`/`[dist]`/`[dist.scheduler]`/`[dist.auth]` テーブルと
//! `dir`/`size`/`bucket`/`endpoint`/`scheduler_url`/`auth`/`url`/
//! `toolchain_cache_size` 等のキーを識別する。
//!
//! ```
//! let b = b"[cache.disk]\ndir = \"/tmp/sccache\"\nsize = 10737418240\n[cache.s3]\nbucket = \"cc\"\nendpoint = \"s3.amazonaws.com\"\nregion = \"us-east-1\"\n";
//! assert!(izanagi_kit::sccache::detect(b));
//! let c = izanagi_kit::sccache::Sccache::parse(b).unwrap();
//! assert_eq!(c.tables, 2);
//! ```

/// Parsed sccache config summary.
#[derive(Debug, Clone)]
pub struct Sccache {
    /// Recognized tables (`[cache.*]`/`[dist.*]`).
    pub tables: usize,
    /// Recognized option keys.
    pub keys: usize,
    /// `key = value` lines.
    pub assignments: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// sccache table names (`cache.*`/`dist.*` match by prefix).
const TABLES: &[&str] = &["cache", "dist"];

/// sccache option keys.
const KEYS: &[&str] = &[
    "access_key",
    "application_id",
    "auth",
    "bucket",
    "cache_type",
    "dir",
    "endpoint",
    "env",
    "heartbeat_error_marks_job_as_finished",
    "job_cache_size",
    "key_prefix",
    "local_url",
    "no_credentials",
    "oauth2_client_id",
    "oauth2_client_secret",
    "oauth2_config_location",
    "public_addr",
    "region",
    "restarts",
    "rewrite_includes_only",
    "scheduler_url",
    "secret_key",
    "selector",
    "server_url",
    "service_account",
    "size",
    "storage_account",
    "toolchain_cache_size",
    "token",
    "type",
    "url",
    "use_preprocessor_cache_mode",
];

fn key_present(t: &str, k: &str) -> bool {
    t.lines().any(|l| {
        let tr = l.trim();
        !tr.starts_with('#') && tr.split('=').next().is_some_and(|p| p.trim() == k)
    })
}

fn table_hit(t: &str) -> usize {
    t.lines()
        .filter_map(|l| {
            let tr = l.trim();
            if tr.starts_with('[') && tr.ends_with(']') && !tr.starts_with("[[") {
                Some(tr[1..tr.len() - 1].trim())
            } else {
                None
            }
        })
        .filter(|n| {
            TABLES.iter().any(|p| {
                *n == *p
                    || (n.len() > p.len() && n.starts_with(*p) && n.as_bytes()[p.len()] == b'.')
            })
        })
        .count()
}

/// Detect an sccache config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    table_hit(t) >= 1 || KEYS.iter().filter(|k| key_present(t, k)).count() >= 4
}

impl Sccache {
    /// Count categories. Returns `None` when the input does not look like
    /// an sccache config.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            tables: table_hit(t),
            keys: 0,
            assignments: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
            } else if tr.contains('=') {
                c.assignments += 1;
                if let Some(k) = tr.split('=').next() {
                    if KEYS.contains(&k.trim()) {
                        c.keys += 1;
                    }
                }
            }
        }
        Some(c)
    }
}

/// Convenience wrapper around [`Sccache::parse`].
#[must_use]
pub fn parse(b: &[u8]) -> Option<Sccache> {
    Sccache::parse(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"[cache.disk]\ndir = \"/var/cache/sccache\"\nsize = 10737418240\n[cache.s3]\nbucket = \"build-cache\"\nendpoint = \"s3.amazonaws.com\"\nregion = \"us-east-1\"\nno_credentials = true\n[cache.redis]\nurl = \"redis://redis/\"\n[dist]\nscheduler_url = \"https://dist.example.com\"\n[dist.auth]\ntype = \"token\"\ntoken = \"secret\"\ntoolchain_cache_size = 10737418240\n";
        assert!(detect(b));
        let c = Sccache::parse(b).unwrap();
        assert_eq!(c.tables, 5);
        assert!(c.keys >= 10);
    }

    #[test]
    fn detects_cache_table_only() {
        assert!(detect(b"[cache.memcached]\nurl = \"memcached://x\"\n"));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"[package]\nname = \"x\"\n"));
        assert!(!detect(b"[cached]\ndir = \"/x\"\n"));
    }

    #[test]
    fn empty_and_binary_inputs_do_not_panic() {
        assert!(!detect(b""));
        assert!(Sccache::parse(b"").is_none());
        let mut b = vec![0xFF, 0x00];
        b.extend_from_slice(b"[cache.disk]");
        assert!(!detect(&b));
    }
}
