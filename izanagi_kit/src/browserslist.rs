//! Browserslist query file parser (`browserslist` / `.browserslistrc`).
//!
//! Detects the line-based query syntax (`> 1%`, `last 2 versions`,
//! `not dead`, `defaults`, `cover 99.5%`, `ie 11`, `current node`,
//! environment sections `[production]` …) and counts queries, `not`
//! exclusions, coverage queries, and env sections.
//!
//! ```
//! let b = b"[production]\n> 1%\nlast 2 versions\nnot dead\n\n[development]\nlast 1 chrome version\n";
//! assert!(izanagi_kit::browserslist::detect(b));
//! let c = izanagi_kit::browserslist::Browserslist::parse(b).unwrap();
//! assert_eq!(c.entries, 4);
//! assert_eq!(c.sections, 2);
//! ```

/// Parsed browserslist summary.
#[derive(Debug, Clone)]
pub struct Browserslist {
    /// Query lines (non-comment, non-section).
    pub entries: usize,
    /// `[env]` section headers (`[production]`/`[development]`/`[staging]`).
    pub sections: usize,
    /// `not …` exclusion queries.
    pub not_queries: usize,
    /// Coverage queries (`> n%`/`>= n%`/`< n%`/`<= n%`/`cover n%`).
    pub coverage_entries: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Query prefixes that strongly mark a browserslist file.
const STRONG: &[&str] = &[
    "last ",
    "defaults",
    "not ",
    "cover ",
    "current node",
    "maintained node",
    "unreleased",
    "dead",
    "since ",
    "browserslist",
];

/// Query prefixes that also mark it (weaker on their own).
const WEAK: &[&str] = &[
    "> ",
    ">= ",
    "< ",
    "<= ",
    "ie ",
    "firefox ",
    "chrome ",
    "safari ",
    "edge ",
    "node ",
    "ios_saf",
    "and_",
    "electron ",
    "opera ",
    "op_mini",
    "samsung",
    "kaios",
    "baidu",
    "bb ",
    "android ",
    "firefoxesr",
    "operamini",
    "uc ",
];

fn is_query(tr: &str) -> bool {
    STRONG.iter().any(|k| tr.starts_with(k)) || WEAK.iter().any(|k| tr.starts_with(k))
}

/// Detect a browserslist file (one strong query, or two weak queries).
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let lc = t.to_ascii_lowercase();
    let mut weak = 0usize;
    for l in lc.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') {
            continue;
        }
        if STRONG.iter().any(|k| tr.starts_with(k))
            || tr.starts_with('>') && tr.contains('%')
            || tr.starts_with("cover")
        {
            return true;
        }
        if WEAK.iter().any(|k| tr.starts_with(k)) {
            weak += 1;
            if weak >= 2 {
                return true;
            }
        }
    }
    false
}

impl Browserslist {
    /// Count queries/sections in a browserslist file. Returns `None` when the
    /// input does not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let lc = t.to_ascii_lowercase();
        let mut c = Self {
            entries: 0,
            sections: 0,
            not_queries: 0,
            coverage_entries: 0,
            comments: 0,
        };
        for l in lc.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if tr.starts_with('[') && tr.ends_with(']') {
                c.sections += 1;
                continue;
            }
            if !is_query(tr)
                && !tr.contains('%')
                && !tr.starts_with("in ")
                && !tr.starts_with("supports")
            {
                continue;
            }
            c.entries += 1;
            if tr.starts_with("not ") {
                c.not_queries += 1;
            }
            if tr.starts_with('>') || tr.starts_with('<') || tr.starts_with("cover ") {
                c.coverage_entries += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"# browsers\n[production]\n> 1%\nlast 2 versions\nnot dead\nnot ie <= 10\n\n[development]\nlast 1 chrome version\nlast 1 firefox version\ncover 95%\ncurrent node\n";
        assert!(detect(b));
        let c = Browserslist::parse(b).unwrap();
        assert_eq!(c.sections, 2);
        assert_eq!(c.entries, 8);
        assert_eq!(c.not_queries, 2);
        assert_eq!(c.coverage_entries, 2);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn detects_simple() {
        assert!(detect(b"defaults\n"));
        assert!(detect(b"ie 11\nsafari 15\n"));
        assert!(!detect(b"hello\nworld\n"));
        assert!(Browserslist::parse(b"name value\n").is_none());
    }
}
