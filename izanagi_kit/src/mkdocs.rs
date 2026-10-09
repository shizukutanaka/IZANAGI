//! MkDocs `mkdocs.yml` census.
//!
//! MkDocs requires `site_name:` and commonly adds `site_url:`,
//! `site_description:`/`site_author:`/`copyright:`, `docs_dir:`/
//! `site_dir:`/`repo_url:`/`repo_name:`/`edit_uri:`, `theme:`,
//! `nav:`, `plugins:`, `markdown_extensions:`, `extra_css:`/
//! `extra_javascript:`/`extra_templates:`/`extra:`, `use_directory_urls:`,
//! `strict:`/`watch:`/`dev_addr:`/`copyright:`/`not_in_nav:`/
//! `validation:`/`hooks:`/`inherits:`/`remote_branch:`/`remote_name:`.
//!
//! ```rust
//! let c = izanagi_kit::mkdocs::Mkdocs::parse(
//!     b"site_name: Docs\nsite_url: https://e.com\ntheme:\n  name: mkdocs\nnav:\n  - Home: index.md\n").unwrap();
//! assert_eq!(c.top_keys, 4);
//! ```

use crate::textutil::strip_bom;
/// `mkdocs.yml` census.
#[derive(Debug, Clone)]
pub struct Mkdocs {
    /// Top-level (column 0) `key:` entries.
    pub top_keys: usize,
    /// Nested `key:` entries (indented).
    pub nested_keys: usize,
    /// `- ` list items.
    pub items: usize,
    /// `#` comments.
    pub comments: usize,
}

const TOP: &[&str] = &[
    "site_name",
    "site_url",
    "site_description",
    "site_author",
    "docs_dir",
    "site_dir",
    "repo_url",
    "repo_name",
    "edit_uri",
    "edit_uri_template",
    "theme",
    "nav",
    "plugins",
    "markdown_extensions",
    "extra_css",
    "extra_javascript",
    "extra_templates",
    "extra",
    "use_directory_urls",
    "strict",
    "watch",
    "dev_addr",
    "copyright",
    "not_in_nav",
    "validation",
    "hooks",
    "inherits",
    "remote_branch",
    "remote_name",
    "locale",
    "exclude_docs",
    "exclude_nav",
    "draft_docs",
    "exclude_unreferenced",
    "exclude_unused_files",
    "navigation",
];

/// Whether the buffer looks like a `mkdocs.yml`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    if !t.lines().any(|l| l.trim_start().starts_with("site_name:")) {
        return false;
    }
    t.lines()
        .filter(|l| {
            let s = l.trim_end();
            if s.starts_with(' ') || s.starts_with('-') || s.starts_with('#') {
                return false;
            }
            let key = s.split(':').next().unwrap_or("").trim();
            TOP.contains(&key)
        })
        .count()
        >= 2
}

impl Mkdocs {
    /// Parse a `mkdocs.yml` into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            top_keys: 0,
            nested_keys: 0,
            items: 0,
            comments: 0,
        };
        for l in t.lines() {
            let s = l.trim_end();
            if s.trim().is_empty() {
                continue;
            }
            if s.trim_start().starts_with('#') {
                c.comments += 1;
                continue;
            }
            if s.trim_start().starts_with("- ") || s.trim_start() == "-" {
                c.items += 1;
                continue;
            }
            if !s.contains(':') {
                continue;
            }
            if s.starts_with(' ') || s.starts_with('\t') {
                c.nested_keys += 1;
            } else {
                c.top_keys += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mkdocs_yml() {
        let b = concat!(
            "site_name: My Docs\n",
            "site_url: https://e.com/docs\n",
            "site_author: me\n",
            "repo_url: https://git.example.com/r\n",
            "edit_uri: edit/main/docs/\n",
            "theme:\n",
            "  name: mkdocs\n",
            "  locale: en\n",
            "nav:\n",
            "  - Home: index.md\n",
            "  - Guide:\n",
            "    - Start: guide/start.md\n",
            "plugins:\n",
            "  - search\n",
            "  - tags\n",
            "markdown_extensions:\n",
            "  - admonition\n",
            "  - codehilite\n",
            "extra_css:\n",
            "  - css/extra.css\n",
            "# done\n",
        );
        let c = Mkdocs::parse(b.as_bytes()).unwrap();
        assert_eq!(c.top_keys, 10);
        assert_eq!(c.nested_keys, 2);
        assert_eq!(c.items, 8);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Mkdocs::parse(b"name: x\nvalue: 1\n").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
