//! `.hgignore` pattern census.
//!
//! `.hgignore` starts optionally with a `syntax: glob` or
//! `syntax: regexp` header then one pattern per line:
//! `*.o`, `^obj/`, `build/`, `glob:*.tmp`, `re:^foo$`,
//! `path:rel`, `rootglob:`/`relglob:`/`reinclude:`/
//! `path:`/`relpath:`/`rootfilesin:`/`rootglob:`/
//! `relglob:`/`root:`/`set:`/`union:`/`listfile:`/`include:`/
//! `subinclude:`/`subtree:`/`re:`/`relre:`/`glob:` forms.
//! `#` comments; `\\` escapes.
//!
//! ```rust
//! let c = izanagi_kit::hgignore::Hgignore::parse(b"syntax: glob\n*.o\nbuild/\n").unwrap();
//! assert_eq!(c.patterns, 2);
//! ```

/// `.hgignore` census.
#[derive(Debug, Clone)]
pub struct Hgignore {
    /// Pattern lines (all syntaxes).
    pub patterns: usize,
    /// `syntax:` header lines.
    pub syntaxes: usize,
    /// Explicitly-prefixed patterns (`glob:`/`re:`/`path:`/…).
    pub prefixed: usize,
    /// `#` comments.
    pub comments: usize,
}

const PREFIXES: &[&str] = &[
    "glob:",
    "regexp:",
    "re:",
    "relre:",
    "path:",
    "relpath:",
    "rootglob:",
    "relglob:",
    "rootfilesin:",
    "root:",
    "set:",
    "union:",
    "listfile:",
    "include:",
    "subinclude:",
    "subtree:",
];
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// Whether the buffer looks like a `.hgignore`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    // `syntax:`/`glob:`/`path:`/`rootglob:`/`regexp:`/`re:`/`include:`… は
    // .hgignore 固有。globstar や `dir/` は .gitignore と同形なので単独の
    // 証拠にしない。
    let mut hits = 0usize;
    let mut prefixed = 0usize;
    for l in t.lines() {
        let s = l.trim();
        let is_hit = s.starts_with("syntax:")
            || PREFIXES.iter().any(|p| s.starts_with(p))
            || s.ends_with('/')
            || s.starts_with('*')
            || s == "**"
            || s.contains("**/");
        if is_hit {
            hits += 1;
            if s.starts_with("syntax:") || PREFIXES.iter().any(|p| s.starts_with(p)) {
                prefixed += 1;
            }
        }
    }
    hits >= 2 && prefixed >= 1
}

impl Hgignore {
    /// Parse a `.hgignore` file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            patterns: 0,
            syntaxes: 0,
            prefixed: 0,
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
            if s.starts_with("syntax:") {
                c.syntaxes += 1;
                continue;
            }
            c.patterns += 1;
            if PREFIXES.iter().any(|p| s.starts_with(p)) {
                c.prefixed += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hgignore() {
        let b = concat!(
            "syntax: glob\n",
            "*.o\n",
            "*.tmp\n",
            "build/\n",
            "dist/\n",
            "# mine\n",
            "re:^config/(dev|test)/\n",
            "path:lib/vend\n",
            "rootglob:gen/**\n",
            "listfile:extra.ignore\n",
        );
        let c = Hgignore::parse(b.as_bytes()).unwrap();
        assert_eq!(c.syntaxes, 1);
        assert_eq!(c.patterns, 8);
        assert_eq!(c.prefixed, 4);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Hgignore::parse(b"hello\nworld\n").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
