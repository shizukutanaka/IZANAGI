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

/// Whether the buffer looks like an ignore file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    // A single glob-ish line is not evidence; require two pattern-shaped
    // lines (no `=`/`:` — those mean a config file, not a path list).
    t.lines()
        .filter(|l| {
            let tr = l.trim();
            !tr.is_empty()
                && !tr.starts_with('#')
                && !tr.contains('=')
                && !tr.contains(':')
                && (tr.contains('*')
                    || tr.contains('?')
                    || tr.starts_with('!')
                    || tr.ends_with('/')
                    || tr.starts_with('/'))
        })
        .count()
        >= 2
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
        assert!(Dockerignore::parse(b"x").is_none());
    }
}
