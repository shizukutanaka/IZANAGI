//! Stack `stack.yaml` census (Haskell Stack).
//!
//! Top-level keys: `resolver`, `snapshot`, `compiler`,
//! `packages`, `extra-deps`, `flags`, `ghc-options`,
//! `extra-package-dbs`, `docker`, `nix`, `system-ghc`,
//! `install-ghc`, `allow-newer`, `allow-different-user`,
//! `default-snapshot`, `stack-root`, `work-dir`,
//! `local-bin-path`, `extra-lib-dirs`, `extra-include-dirs`,
//! `snapshot-location-base`, `apply-ghc-options`,
//! `recommended-stack-version`, `ghc-variant`, `jobs`,
//! `drop-packages`, `curator`, `save-hackage-creds`,
//! `ghc-build`, `ghc-options-by-package`, `warn-other`,
//! `develop-packages`, `extra-dep-timestamps`, `custom-preprocessors`.
//!
//! ```rust
//! let k = b"resolver: lts-22.0\npackages:\n- .\nextra-deps:\n- acme-missiles-0.3\n";
//! assert!(izanagi_kit::stack::detect(k));
//! ```

/// stack.yaml census.
#[derive(Debug, Clone)]
pub struct Stack {
    /// `key:`/`key: value` lines.
    pub settings: usize,
    /// `- ` list items.
    pub items: usize,
    /// recognised top-level keys present.
    pub top_keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const STRONG: &[&str] = &[
    "resolver",
    "extra-deps",
    "ghc-options",
    "extra-package-dbs",
    "system-ghc",
    "install-ghc",
    "allow-newer",
    "allow-different-user",
    "default-snapshot",
    "stack-root",
    "work-dir",
    "local-bin-path",
    "extra-lib-dirs",
    "extra-include-dirs",
    "snapshot-location-base",
    "apply-ghc-options",
    "recommended-stack-version",
    "ghc-variant",
    "drop-packages",
    "curator",
    "save-hackage-creds",
    "ghc-build",
    "warn-other",
    "develop-packages",
    "extra-dep-timestamps",
    "custom-preprocessors",
    "snapshot",
    "compiler-check",
    "compiler-repository",
    "package-indices",
    "pantry-tree",
];

const WEAK: &[&str] = &[
    "snapshot",
    "compiler",
    "packages",
    "flags",
    "docker",
    "nix",
    "jobs",
    "build",
    "haddock",
    "test",
    "bench",
    "configure-options",
    "cabal-verbosity",
    "program-locations",
    "templates",
    "connection-count",
    "concurrent-tests",
    "ghc-options-by-package",
    "extra-path",
    "notify-if-nix-on-path",
    "required-stack-version",
    "setup-info-locations",
    "urls",
    "color",
    "snapshot-locations",
];

fn top_key(line: &str) -> Option<&str> {
    if line.starts_with(' ') || line.starts_with('\t') || line.starts_with('-') {
        return None;
    }
    let s = line.trim_end();
    if s.is_empty() || s.starts_with('#') {
        return None;
    }
    match s.find(':') {
        Some(i) => {
            let k = s[..i].trim().trim_matches('"').trim_matches('\'');
            if k.is_empty() {
                None
            } else {
                Some(k)
            }
        }
        None => None,
    }
}

/// Detect a `stack.yaml` file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // `resolver`/`extra-deps`/`ghc-options`/`system-ghc` are
    // Stack-exclusive top keys.
    let mut strong = 0usize;
    let mut weak = 0usize;
    for line in t.lines() {
        if let Some(k) = top_key(line) {
            if STRONG.contains(&k) {
                strong += 1;
            } else if WEAK.contains(&k) {
                weak += 1;
            }
        }
    }
    strong >= 1 && strong + weak >= 2
}

impl Stack {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            settings: 0,
            items: 0,
            top_keys: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if s.starts_with("- ") {
                c.items += 1;
                continue;
            }
            if s.contains(':') {
                c.settings += 1;
                if let Some(k) = top_key(line) {
                    if STRONG.contains(&k) || WEAK.contains(&k) {
                        c.top_keys += 1;
                    }
                }
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"resolver: lts-22.0\npackages:\n- .\nextra-deps:\n- acme-missiles-0.3\n";
        assert!(detect(b));
        let c = Stack::parse(b).unwrap();
        assert!(c.top_keys >= 2);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"packages:\n- x\nflags:\n  a: b\n"));
        assert!(!detect(
            b"# resolver: lts-22\n# extra-deps: [x]\npackages: []\n"
        ));
    }
}
