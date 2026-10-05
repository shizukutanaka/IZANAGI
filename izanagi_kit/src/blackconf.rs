//! Black `pyproject.toml` `[tool.black]` parser.
//!
//! Detects the Black formatter config by `[tool.black]` plus
//! `line-length`/`target-version`/`skip-string-normalization`/
//! `skip-magic-trailing-comma`/`preview`/`fast`/`safe` keys, and counts
//! structure.
//!
//! ```
//! let b = b"[tool.black]\nline-length = 100\ntarget-version = [\"py311\"]\nskip-string-normalization = true\n";
//! assert!(izanagi_kit::blackconf::detect(b));
//! let c = izanagi_kit::blackconf::Black::parse(b).unwrap();
//! assert!(c.keys >= 3);
//! ```

/// Parsed [tool.black] summary.
#[derive(Debug, Clone)]
pub struct Black {
    /// Recognized key occurrences.
    pub keys: usize,
    /// `key = value` lines.
    pub assignments: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Known keys.
const KEYS: &[&str] = &[
    "line-length",
    "target-version",
    "skip-string-normalization",
    "skip-magic-trailing-comma",
    "line-ending",
    "preview",
    "fast",
    "safe",
    "include",
    "exclude",
    "extend-exclude",
    "force-exclude",
    "quiet",
    "verbose",
    "diff",
    "check",
    "color",
    "code",
    "skip-source-first-line",
    "python-cell-magics",
    "enable-unstable-feature",
    "workers",
    "required-version",
];

/// Detect a `[tool.black]` section.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("[tool.black]")
}

impl Black {
    /// Count categories. Returns `None` when the input does not look like
    /// a Black config.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
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
            }
        }
        for k in KEYS {
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
        let b = b"[tool.black]\nline-length = 100\ntarget-version = [\"py311\", \"py312\"]\nskip-string-normalization = true\nskip-magic-trailing-comma = false\npreview = true\nexclude = \"/(build|dist)/\"\n";
        assert!(detect(b));
        let c = Black::parse(b).unwrap();
        assert_eq!(c.assignments, 6);
        assert!(c.keys >= 6);
    }

    #[test]
    fn rejects_toml() {
        assert!(!detect(b"[tool.isort]\nprofile = \"black\"\n"));
        assert!(Black::parse(b"").is_none());
    }
}
