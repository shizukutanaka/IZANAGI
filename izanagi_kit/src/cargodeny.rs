//! cargo-deny `deny.toml` 形式の検出と構造カウント。
//!
//! `[bans]`/`[licenses]`/`[advisories]`/`[sources]`/`[graph]`/`[build]`
//! テーブルと `multiple-versions`/`wildcards`/`vulnerability`/
//! `unmaintained`/`yanked`/`allow-osi-fsf-free`/`copyleft`/`exceptions`
//! 等のポリシーキーを識別する。
//!
//! ```
//! let b = b"[graph]\ntargets = [\"x86_64-unknown-linux-gnu\"]\n[advisories]\nvulnerability = \"deny\"\nyanked = \"warn\"\n[bans]\nmultiple-versions = \"warn\"\nwildcards = \"deny\"\n[licenses]\nallow = [\"MIT\", \"Apache-2.0\"]\ncopyleft = \"deny\"\n";
//! assert!(izanagi_kit::cargodeny::detect(b));
//! let c = izanagi_kit::cargodeny::CargoDeny::parse(b).unwrap();
//! assert_eq!(c.tables, 4);
//! ```

/// Parsed deny.toml summary.
#[derive(Debug, Clone)]
pub struct CargoDeny {
    /// Recognized policy tables (`[bans]`/`[licenses]`/...).
    pub tables: usize,
    /// Recognized option keys.
    pub keys: usize,
    /// `key = value` lines.
    pub assignments: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// cargo-deny table names (sub-tables like `[licenses.private]` also match).
const TABLES: &[&str] = &[
    "bans",
    "licenses",
    "advisories",
    "sources",
    "graph",
    "build",
    "exceptions",
    "license-files",
];

/// cargo-deny option keys.
const KEYS: &[&str] = &[
    "accept",
    "accepts",
    "allow",
    "allow-osi-fsf-free",
    "allow-fsf",
    "allow-registry",
    "allow-git",
    "allow-org",
    "allow-not-allow",
    "allow-with-exception",
    "allowed-extensions",
    "bans",
    "build",
    "build-default-features",
    "check",
    "clarifications",
    "confidence-threshold",
    "copyleft",
    "deny",
    "deny-with-exception",
    "disabled",
    "duplicate",
    "exact",
    "exceptions",
    "exclude",
    "exclude-dev",
    "feature-additives",
    "feature-depth",
    "features",
    "highlight",
    "ignore",
    "include-build-dependencies",
    "include-dev-dependencies",
    "licenses",
    "minimum-severity",
    "multiple-versions",
    "no-default-features",
    "notice",
    "private",
    "publishable-crates",
    "required-extensions",
    "severity-threshold",
    "skip",
    "skip-tree",
    "sources",
    "target",
    "targets",
    "unmaintained",
    "unlicensed",
    "unsound",
    "url",
    "version",
    "vulnerability",
    "wildcards",
    "workspace-default-features",
    "yanked",
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

/// Detect a cargo-deny config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let tables = table_hit(t);
    let n = KEYS.iter().filter(|k| key_present(t, k)).count();
    tables >= 1 || n >= 4
}

impl CargoDeny {
    /// Count categories. Returns `None` when the input does not look like
    /// a cargo-deny config.
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

/// Convenience wrapper around [`CargoDeny::parse`].
#[must_use]
pub fn parse(b: &[u8]) -> Option<CargoDeny> {
    CargoDeny::parse(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"# policy\n[graph]\ntargets = [\"x86_64-unknown-linux-gnu\"]\nall-features = false\n[advisories]\nvulnerability = \"deny\"\nunmaintained = \"warn\"\nyanked = \"warn\"\nignore = [\"RUSTSEC-0000-0000\"]\n[bans]\nmultiple-versions = \"warn\"\nwildcards = \"deny\"\nhighlight = \"all\"\nskip = [{ name = \"windows-sys\" }]\n[licenses]\nallow = [\"MIT\", \"Apache-2.0\", \"BSD-3-Clause\"]\nexceptions = [{ allow = [\"Unicode-DFS-2016\"], name = \"unicode-ident\" }]\ncopyleft = \"deny\"\nconfidence-threshold = 0.8\n[sources]\nunknown-registry = \"deny\"\nunknown-git = \"deny\"\nallow-org = [\"github\"]\n";
        assert!(detect(b));
        let c = CargoDeny::parse(b).unwrap();
        assert_eq!(c.tables, 5);
        assert!(c.keys >= 12);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn detects_table_only() {
        assert!(detect(b"[licenses]\nallow = [\"MIT\"]\n"));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"[dependencies]\nserde = \"1\"\n"));
        assert!(!detect(b"random = true\nnothing = 1\n"));
    }

    #[test]
    fn empty_and_binary_inputs_do_not_panic() {
        assert!(!detect(b""));
        assert!(CargoDeny::parse(b"").is_none());
        let mut b = vec![0xFF, 0x00];
        b.extend_from_slice(b"[bans]");
        assert!(!detect(&b));
    }
}
