//! detekt `detekt.yml` / `detekt-config.yml` census.
//!
//! detekt config is YAML: top-level sections (`build`, `processors`,
//! `console-reports`, `output-reports`, `config`, `comments`,
//! `complexity`, `coroutines`, `empty-blocks`, `exceptions`,
//! `formatting`, `libraries`, `naming`, `performance`,
//! `potential-bugs`, `style`, `unnecessary`…) each holding rule
//! blocks (`RuleName:`) with `active:`/`autoCorrect:`/`excludes:`/
//! `severity:`/`threshold:`/`ignoreAnnotated:` entries.
//!
//! ```rust
//! let c = izanagi_kit::detekt::Detekt::parse(b"style:\n  NewLineAtEndOfFile:\n    active: true\n").unwrap();
//! assert_eq!(c.rules, 1);
//! ```

/// detekt config census.
#[derive(Debug, Clone)]
pub struct Detekt {
    /// Top-level `key:` sections.
    pub sections: usize,
    /// Nested `Key:` rule/config blocks.
    pub rules: usize,
    /// `active:` entries.
    pub actives: usize,
    /// Other scalar `key: value` settings.
    pub settings: usize,
    /// `- ` list items.
    pub listitems: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const SECTIONS: &[&str] = &[
    "build",
    "processors",
    "console-reports",
    "output-reports",
    "config",
    "comments",
    "complexity",
    "coroutines",
    "empty-blocks",
    "exceptions",
    "formatting",
    "libraries",
    "naming",
    "performance",
    "potential-bugs",
    "style",
    "unnecessary",
    "test-pattern",
];

/// Whether the buffer looks like a detekt config file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut hits = 0usize;
    for l in t.lines() {
        let s = l.trim();
        if let Some(k) = s.strip_suffix(':') {
            if SECTIONS.contains(&k) {
                hits += 1;
            }
        }
    }
    hits >= 1 && (hits >= 2 || t.contains("active:") || t.contains("autoCorrect:"))
}

impl Detekt {
    /// Parse a detekt config into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            rules: 0,
            actives: 0,
            settings: 0,
            listitems: 0,
            comments: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            let ind = l.len() - l.trim_start().len();
            if s.strip_prefix("- ").is_some() || s == "-" {
                c.listitems += 1;
                continue;
            }
            if s.ends_with(':') {
                if ind == 0 {
                    c.sections += 1;
                } else {
                    c.rules += 1;
                }
                continue;
            }
            if let Some((k, _)) = s.split_once(':') {
                if k.trim() == "active" {
                    c.actives += 1;
                }
                c.settings += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_detekt() {
        let b = concat!(
            "# detekt\n",
            "build:\n",
            "  maxIssues: 10\n",
            "style:\n",
            "  NewLineAtEndOfFile:\n",
            "    active: true\n",
            "  WildcardImport:\n",
            "    active: true\n",
            "    excludes: ['**/test/**']\n",
            "complexity:\n",
            "  LongMethod:\n",
            "    active: true\n",
            "    threshold: 60\n",
        );
        let c = Detekt::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.rules, 3);
        assert_eq!(c.actives, 3);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Detekt::parse(b"foo:\n  bar: 1").is_none());
    }
}
