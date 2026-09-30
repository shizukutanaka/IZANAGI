//! `.gitignore` / `.git/info/exclude` pattern census.
//!
//! One glob per line: `*.o`, `build/`, `/target`, `**/logs`,
//! `!keep.txt` negation, `**/doc/**/*.md` double-star,
//! `foo?.txt`/`bar[ab]` classes, `#` comments, `\\` escaping.
//!
//! ```rust
//! let c = izanagi_kit::gitignore::Gitignore::parse(b"target/\n*.o\n!keep.o\n").unwrap();
//! assert_eq!(c.patterns, 2);
//! ```

/// `.gitignore`-style pattern census.
#[derive(Debug, Clone)]
pub struct Gitignore {
    /// Pattern lines (excludes negations/comments).
    pub patterns: usize,
    /// `!` negations.
    pub negations: usize,
    /// `#` comments.
    pub comments: usize,
    /// Directory-only patterns (`dir/`).
    pub dir_only: usize,
    /// Patterns with `*`/`?`/`[`/`]`.
    pub globs: usize,
    /// Anchored patterns (`/` at start or middle).
    pub anchored: usize,
}

/// Whether the buffer looks like a gitignore-style file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut hits = 0usize;
    for l in t.lines() {
        let s = l.trim();
        if s.is_empty() {
            continue;
        }
        let s = s.strip_prefix('!').unwrap_or(s);
        if s == "*"
            || s == "**"
            || s.starts_with('*')
            || s.ends_with('/')
            || s.starts_with('/')
            || s.contains("**")
            || s.contains('.') && !s.contains(' ')
            || s.starts_with('[')
        {
            hits += 1;
        }
    }
    hits >= 2
}

impl Gitignore {
    /// Parse a gitignore-style file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            patterns: 0,
            negations: 0,
            comments: 0,
            dir_only: 0,
            globs: 0,
            anchored: 0,
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
            let neg = s.starts_with('!');
            let p = if neg { &s[1..] } else { s };
            if neg {
                c.negations += 1;
            } else {
                c.patterns += 1;
            }
            if p.ends_with('/') {
                c.dir_only += 1;
            }
            if p.contains('*') || p.contains('?') || p.contains('[') {
                c.globs += 1;
            }
            if p.contains('/') {
                c.anchored += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_gitignore() {
        let b = concat!(
            "# build\n",
            "target/\n",
            "build/\n",
            "*.o\n",
            "*.log\n",
            "node_modules/\n",
            "!keep.o\n",
            "!dist/keep.txt\n",
            "/.env\n",
            "**/tmp/*\n",
            "doc/**/*.pdf\n",
            "[Ll]ocal\n",
        );
        let c = Gitignore::parse(b.as_bytes()).unwrap();
        assert_eq!(c.patterns, 9);
        assert_eq!(c.negations, 2);
        assert_eq!(c.comments, 1);
        assert_eq!(c.dir_only, 3);
        assert_eq!(c.globs, 5);
    }

    #[test]
    fn rejects_other() {
        assert!(Gitignore::parse(b"hello\nworld\n").is_none());
    }
}
