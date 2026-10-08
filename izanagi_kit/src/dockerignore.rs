//! `.dockerignore` / `.gitignore`-style pattern file format.
//!
//! Ignore files list glob patterns (`*`, `?`, `**`), `!` exceptions,
//! `#` comments, and `/`-anchored paths.
//!
//! ```
//! let b = concat!(
//!     "*.log\n",
//!     "target/\n",
//!     "!keep.log\n",
//!     "# comment\n",
//!     "build/**/tmp\n"
//! ).as_bytes();
//! assert!(izanagi_kit::dockerignore::detect(b));
//! let c = izanagi_kit::dockerignore::Dockerignore::parse(b).unwrap();
//! assert_eq!(c.patterns, 4);
//! assert_eq!(c.exceptions, 1);
//! ```

/// Parsed .dockerignore summary.
#[derive(Debug, Clone)]
pub struct Dockerignore {
    /// Pattern lines.
    pub patterns: usize,
    /// `!` exception lines.
    pub exceptions: usize,
    /// `**` recursive globs.
    pub recursions: usize,
    /// `/`-anchored patterns.
    pub anchored: usize,
    /// `?` single-char globs.
    pub questions: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// One ignore-file pattern line: non-empty, no whitespace, and none of
/// the characters that would make it an assignment, URL, markup or
/// code fragment instead of a path pattern.
fn pattern_line(s: &str) -> bool {
    !s.is_empty()
        && !s.contains(char::is_whitespace)
        && !s.chars().any(|c| {
            matches!(
                c,
                '=' | ':' | '"' | '\'' | '{' | '}' | ';' | ',' | '(' | ')' | '<' | '>'
            )
        })
}

/// Whether the buffer looks like an ignore file: *every* content line
/// must be pattern-shaped, with at least two patterns and one
/// glob/`!`-negation/`/`-anchor marker.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut patterns = 0usize;
    let mut markers = 0usize;
    let mut bad = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') {
            continue;
        }
        if !pattern_line(tr) {
            bad += 1;
            continue;
        }
        patterns += 1;
        let p = tr.strip_prefix('!').unwrap_or(tr);
        if p.contains('*')
            || p.contains('?')
            || p.contains('[')
            || p.starts_with('/')
            || p.ends_with('/')
        {
            markers += 1;
        }
    }
    bad == 0 && patterns >= 2 && markers >= 1
}

impl Dockerignore {
    /// Parses an ignore-file summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            patterns: 0,
            exceptions: 0,
            recursions: 0,
            anchored: 0,
            questions: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            c.patterns += 1;
            let body = tr.strip_prefix('!').unwrap_or(tr);
            if tr.starts_with('!') {
                c.exceptions += 1;
            }
            if body.contains("**") {
                c.recursions += 1;
            }
            if body.starts_with('/') {
                c.anchored += 1;
            }
            if body.contains('?') {
                c.questions += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_dockerignore() {
        let b = concat!(
            "*.log\n",
            "target/\n",
            "!keep.log\n",
            "# comment\n",
            "build/**/tmp\n",
            "/root_only\n",
            "file?.txt\n"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Dockerignore::parse(b).unwrap();
        assert_eq!(c.patterns, 6);
        assert_eq!(c.exceptions, 1);
        assert_eq!(c.recursions, 1);
        assert_eq!(c.anchored, 1);
        assert_eq!(c.questions, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"plain\ntext\nlines\n"));
        // a single glob inside an unrelated config is not an ignore file
        assert!(!detect(b"key = value\n*.log\n"));
        assert!(Dockerignore::parse(b"x").is_none());
    }
}
