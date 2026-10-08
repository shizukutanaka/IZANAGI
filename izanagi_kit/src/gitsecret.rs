//! git-secret `.gitsecret` file census.
//!
//! git-secret tracks files to encrypt in a top-level `.gitsecret`
//! file using gitignore-style patterns: one path per line
//! (`secrets.env`, `*.key`, `certs/*.pem`, `!not-this.txt`),
//! `#` comments, blank lines ignored.
//!
//! ```rust
//! let c = izanagi_kit::gitsecret::Gitsecret::parse(b"*.secret\nkeys/*.pem\n").unwrap();
//! assert_eq!(c.patterns, 2);
//! ```

use crate::textutil::strip_bom;
/// `.gitsecret` file census.
#[derive(Debug, Clone)]
pub struct Gitsecret {
    /// Path/pattern entries.
    pub patterns: usize,
    /// `!`-negated entries.
    pub negated: usize,
    /// `#` comments.
    pub comments: usize,
}

/// Whether the buffer looks like a `.gitsecret` list.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    if t.contains("git-secret") || t.contains("gitsecret") {
        return true;
    }
    t.lines()
        .map(|l| l.trim())
        .filter(|s| {
            !s.is_empty()
                && !s.starts_with('#')
                && (s.ends_with(".secret")
                    || s.ends_with(".gpg")
                    || s.ends_with(".key")
                    || s.ends_with(".pem")
                    || s.ends_with(".env")
                    || s.contains("*.secret")
                    || s.starts_with("!*."))
        })
        .count()
        >= 2
}

impl Gitsecret {
    /// Parse a `.gitsecret` list into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            patterns: 0,
            negated: 0,
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
            c.patterns += 1;
            if s.starts_with('!') {
                c.negated += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_gitsecret() {
        let b = concat!(
            "# git-secret list\n",
            "credentials.env\n",
            "config/prod.key\n",
            "*.secret\n",
            "keys/app.pem\n",
            "!keys/public.pem\n",
            "tokens.gpg\n",
        );
        let c = Gitsecret::parse(b.as_bytes()).unwrap();
        assert_eq!(c.patterns, 6);
        assert_eq!(c.negated, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Gitsecret::parse(b"just some text\n").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
