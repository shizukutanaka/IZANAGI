//! Git `.gitconfig` / `config` INI-like format.
//!
//! git config files use `[section]` / `[section "subsection"]` headers,
//! `key = value` settings, bare `key` booleans, `;`/`#` comments, and
//! `[include]`/`[includeIf]`/`[url]`/`[alias]`-style sections.
//!
//! ```
//! let b = concat!(
//!     "[user]\n",
//!     "\tname = Alice\n",
//!     "\temail = a@example.com\n",
//!     "[core]\n",
//!     "\tautocrlf = false\n",
//!     "[includeIf \"gitdir:~/work/\"]\n",
//!     "\tpath = ~/.gitconfig-work\n",
//!     "[alias]\n",
//!     "\tst = status -sb\n"
//! ).as_bytes();
//! assert!(izanagi_kit::gitconfig::detect(b));
//! let c = izanagi_kit::gitconfig::Gitconfig::parse(b).unwrap();
//! assert_eq!(c.sections, 4);
//! assert_eq!(c.entries, 5);
//! ```

/// Parsed gitconfig summary.
#[derive(Debug, Clone)]
pub struct Gitconfig {
    /// `[section]` / `[section "sub"]` headers.
    pub sections: usize,
    /// Subsection headers `[sec "sub"]`.
    pub subsections: usize,
    /// `key = value` entries.
    pub entries: usize,
    /// Bare `key` boolean lines.
    pub bare_bools: usize,
    /// `[include]`/`[includeIf]` headers.
    pub includes: usize,
    /// `[url]`/`[credential]`/`[alias]` headers.
    pub special: usize,
    /// `;`/`#` comment lines.
    pub comments: usize,
}

/// Whether the buffer looks like a gitconfig.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut sections = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.starts_with('[') && tr.ends_with(']') && tr.len() > 2 {
            sections += 1;
        }
    }
    sections >= 1
        && t.lines().any(|l| {
            let tr = l.trim();
            !tr.starts_with('[') && tr.contains('=') && !tr.is_empty()
        })
}

impl Gitconfig {
    /// Parses a gitconfig summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            subsections: 0,
            entries: 0,
            bare_bools: 0,
            includes: 0,
            special: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with(';') || tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if tr.starts_with('[') && tr.ends_with(']') {
                c.sections += 1;
                let inner = &tr[1..tr.len() - 1];
                if inner.contains('"') {
                    c.subsections += 1;
                }
                let name = inner.split_whitespace().next().unwrap_or("");
                match name.to_ascii_lowercase().as_str() {
                    "include" | "includeif" => c.includes += 1,
                    "url" | "credential" | "alias" | "credential.helper" => c.special += 1,
                    _ => {}
                }
                continue;
            }
            if tr.contains('=') {
                c.entries += 1;
            } else {
                c.bare_bools += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_gitconfig() {
        let b = concat!(
            "[user]\n",
            "\tname = Alice\n",
            "\temail = a@example.com\n",
            "[core]\n",
            "\tautocrlf = false\n",
            "[includeIf \"gitdir:~/work/\"]\n",
            "\tpath = ~/.gitconfig-work\n",
            "[alias]\n",
            "\tst = status -sb\n",
            "[url \"ssh://git@x\"]\n",
            "\tinsteadOf = https://x\n",
            "; tail\n"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Gitconfig::parse(b).unwrap();
        assert_eq!(c.sections, 5);
        assert_eq!(c.subsections, 2);
        assert_eq!(c.entries, 6);
        assert_eq!(c.includes, 1);
        assert_eq!(c.special, 2);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"no sections here\n"));
        assert!(Gitconfig::parse(b"x").is_none());
    }
}
