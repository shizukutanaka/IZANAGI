//! `.gitmodules` census.
//!
//! `.gitmodules` is git-config INI: `[submodule "name"]`
//! sections with `path`/`url`/`branch`/`update`/`fetchRecurseSubmodules`/
//! `ignore`/`shallow`/`worktree`/`sync`/`fetchesPerHost`/
//! `core.sparseCheckout`/`subsystem` keys.
//!
//! ```rust
//! let c = izanagi_kit::gitmodules::Gitmodules::parse(b"[submodule \"libs/a\"]\npath = libs/a\nurl = ../a\n").unwrap();
//! assert_eq!(c.submodules, 1);
//! ```

/// `.gitmodules` census.
#[derive(Debug, Clone)]
pub struct Gitmodules {
    /// `[submodule "…"]` sections.
    pub submodules: usize,
    /// `path`/`url`/`branch`/… entries.
    pub entries: usize,
    /// `url` entries.
    pub urls: usize,
    /// `#`/`;` comments.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "path",
    "url",
    "branch",
    "update",
    "fetchRecurseSubmodules",
    "ignore",
    "shallow",
    "worktree",
    "sync",
    "fetchesPerHost",
    "subsystem",
];

/// Whether the buffer looks like `.gitmodules`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("[submodule ") && (t.contains("path") || t.contains("url"))
}

impl Gitmodules {
    /// Parse `.gitmodules` into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            submodules: 0,
            entries: 0,
            urls: 0,
            comments: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') || s.starts_with(';') {
                c.comments += 1;
                continue;
            }
            if s.starts_with('[') {
                if s.starts_with("[submodule ") || s.starts_with("[submodule\t") {
                    c.submodules += 1;
                }
                continue;
            }
            if s.contains('=') {
                let key = s.split('=').next().unwrap_or("").trim();
                c.entries += 1;
                if key == "url" && KEYS.contains(&key) {
                    c.urls += 1;
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
    fn parses_gitmodules() {
        let b = concat!(
            "[submodule \"libs/a\"]\n",
            "path = libs/a\n",
            "url = https://x/a.git\n",
            "branch = main\n",
            "update = checkout\n",
            "[submodule \"deps/b\"]\n",
            "path = deps/b\n",
            "url = git@x:b.git\n",
            "ignore = all\n",
            "fetchRecurseSubmodules = false\n",
            "[submodule \"vend/c\"]\n",
            "path = vend/c\n",
            "url = ../c\n",
            "shallow = true\n",
        );
        let c = Gitmodules::parse(b.as_bytes()).unwrap();
        assert_eq!(c.submodules, 3);
        assert_eq!(c.entries, 11);
        assert_eq!(c.urls, 3);
    }

    #[test]
    fn rejects_other() {
        assert!(Gitmodules::parse(b"[section]\nx=1\n").is_none());
    }
}
