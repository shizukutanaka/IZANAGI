//! `.gitattributes` / `.git/info/attributes` census.
//!
//! `<pattern> <attr> <attr>` lines: `*.py text`,
//! `*.txt -merge`, `*.bin binary`, `*.svg diff=svg`,
//! `*.pdf -diff`, `*.c text eol=lf`, `*.gen -text`,
//! `macro` attrs `binary`, `export-ignore`, `export-subst`,
//! `filter=`/`diff=`/`merge=`/`eol=`/`text=auto`,
//! `working-tree-encoding=`, `ident`, `whitespace=`/
//! `-whitespace`, `encoding=`, `builtin_…` are not used.
//! `#` comments; `*`, `**`/`[a-z]` globs.
//!
//! ```rust
//! let c = izanagi_kit::gitattributes::Gitattributes::parse(b"* text=auto\n*.bin binary\n").unwrap();
//! assert_eq!(c.rules, 2);
//! ```

/// `.gitattributes` census.
#[derive(Debug, Clone)]
pub struct Gitattributes {
    /// Pattern lines.
    pub rules: usize,
    /// Attribute assignments (`attr`/`attr=value`/`-attr`).
    pub attrs: usize,
    /// `-attr` unset attributes.
    pub unset: usize,
    /// `attr=value` valued attributes.
    pub valued: usize,
    /// `#` comments.
    pub comments: usize,
}

const ATTRS: &[&str] = &[
    "text",
    "-text",
    "binary",
    "diff",
    "-diff",
    "merge",
    "-merge",
    "union",
    "whitespace",
    "-whitespace",
    "export-ignore",
    "export-subst",
    "ident",
    "filter",
    "eol",
    "working-tree-encoding",
    "encoding",
    "linguist-vendored",
    "linguist-generated",
    "linguist-documentation",
    "linguist-language",
    "linguist-detectable",
    "crlf",
    "conflict-marker-size",
    "git",
    "delta",
    "ad",
    "conflict-style",
    "conflict-marker-style",
    "ours",
    "theirs",
    "conflictdiff",
    "textconv",
    "cachetextconv",
    "binarydiff",
    "word-diff",
    "ignorediff",
];
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// Whether the buffer looks like `.gitattributes`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    let hits = t
        .lines()
        .filter(|l| {
            let s = l.trim();
            if s.is_empty() || s.starts_with('#') {
                return false;
            }
            let mut it = s.split_whitespace();
            let pat = it.next().unwrap_or("");
            let globish = pat.contains('*')
                || pat.contains('?')
                || pat.contains('[')
                || pat.contains('/')
                || pat.contains('.')
                || pat.starts_with('!');
            globish
                && it.any(|tok| {
                    ATTRS
                        .iter()
                        .any(|a| *a == tok.split('=').next().unwrap_or(""))
                })
        })
        .count();
    hits >= 1 || t.contains("linguist-")
}

impl Gitattributes {
    /// Parse a `.gitattributes` file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            rules: 0,
            attrs: 0,
            unset: 0,
            valued: 0,
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
            let mut toks = s.split_whitespace();
            toks.next(); // pattern
            c.rules += 1;
            for tok in toks {
                c.attrs += 1;
                if tok.starts_with('-') {
                    c.unset += 1;
                } else if tok.contains('=') {
                    c.valued += 1;
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
    fn parses_gitattributes() {
        let b = concat!(
            "* text=auto\n",
            "*.py text eol=lf\n",
            "*.sh text eol=lf\n",
            "*.bat text eol=crlf\n",
            "*.bin binary\n",
            "*.pdf -diff -text\n",
            "*.svg diff=svg\n",
            "*.gen -merge\n",
            "dist/ export-ignore\n",
            "*.dat filter=myf\n",
            "# comment\n",
            "src/** linguist-vendored\n",
        );
        let c = Gitattributes::parse(b.as_bytes()).unwrap();
        assert_eq!(c.rules, 11);
        assert_eq!(c.attrs, 15);
        assert_eq!(c.unset, 3);
        assert_eq!(c.valued, 6);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Gitattributes::parse(b"just text\nno attrs\n").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
