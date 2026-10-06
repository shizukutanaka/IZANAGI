//! cargo-nextest `.config/nextest.toml` 形式の検出と構造カウント。
//!
//! `[profile.*]`/`[store]`/`[experimental]` テーブルと `retries`/
//! `slow-timeout`/`fail-fast`/`status-level`/`failure-output`/`junit`/
//! `test-threads`/`threads-required`/`default-filter` 等のキーを識別する。
//!
//! ```
//! let b = b"[store]\ndir = \"target/nextest\"\n[profile.default]\nretries = 2\nslow-timeout = { period = \"30s\" }\nfail-fast = false\nstatus-level = \"pass\"\n[profile.ci]\nretries = 3\n";
//! assert!(izanagi_kit::nextest::detect(b));
//! let c = izanagi_kit::nextest::Nextest::parse(b).unwrap();
//! assert_eq!(c.tables, 3);
//! ```

/// Parsed nextest.toml summary.
#[derive(Debug, Clone)]
pub struct Nextest {
    /// Recognized tables (`[profile.*]`/`[store]`/`[experimental]`).
    pub tables: usize,
    /// Recognized option keys.
    pub keys: usize,
    /// `key = value` lines.
    pub assignments: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// nextest table names (`profile.*` matches by prefix).
const TABLES: &[&str] = &["profile", "store", "experimental", "scripts"];

/// nextest option keys.
const KEYS: &[&str] = &[
    "archive",
    "archive-dir",
    "cargo-options",
    "concise",
    "default-filter",
    "default-retries",
    "description",
    "fail-fast",
    "failure-output",
    "filter-expr",
    "filterset",
    "final-status-level",
    "harness",
    "immediates",
    "junit",
    "leak-timeout",
    "max-fail",
    "message-format",
    "nextest-config",
    "no-fail-fast",
    "no-tests",
    "override",
    "overrides",
    "partition",
    "persist",
    "platform",
    "profile",
    "release-mode",
    "reporter",
    "retries",
    "run-ignored",
    "setup-scripts",
    "slow-timeout",
    "status-level",
    "store-dir",
    "success-output",
    "termination-wait",
    "test-threads",
    "threads-required",
    "verbose",
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

/// Detect a nextest config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    table_hit(t) >= 1 || KEYS.iter().filter(|k| key_present(t, k)).count() >= 3
}

impl Nextest {
    /// Count categories. Returns `None` when the input does not look like
    /// a nextest config.
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

/// Convenience wrapper around [`Nextest::parse`].
#[must_use]
pub fn parse(b: &[u8]) -> Option<Nextest> {
    Nextest::parse(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"[store]\ndir = \"target/nextest\"\n[profile.default]\nretries = 2\nslow-timeout = { period = \"30s\", terminate-after = true }\nfail-fast = false\nstatus-level = \"skip\"\nfinal-status-level = \"fail\"\nfailure-output = \"immediate-final\"\nsuccess-output = \"never\"\ntest-threads = \"num-cpus\"\njunit = { path = \"junit.xml\" }\n[profile.ci]\nretries = 3\n";
        assert!(detect(b));
        let c = Nextest::parse(b).unwrap();
        assert_eq!(c.tables, 3);
        assert!(c.keys >= 10);
    }

    #[test]
    fn detects_profile_only() {
        assert!(detect(b"[profile.default]\nretries = 1\n"));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"[package]\nname = \"x\"\n"));
        assert!(!detect(b"[profiles.release]\nopt-level = 3\n"));
    }

    #[test]
    fn empty_and_binary_inputs_do_not_panic() {
        assert!(!detect(b""));
        assert!(Nextest::parse(b"").is_none());
        let mut b = vec![0xFF, 0x00];
        b.extend_from_slice(b"[profile.default]");
        assert!(!detect(&b));
    }
}
