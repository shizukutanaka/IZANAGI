//! Jekyll `_config.yml` census.
//!
//! A Jekyll config lists `title:`/`description:`/`email:`/`url:`/
//! `baseurl:`/`permalink:`/`theme:`/`remote_theme:`/`plugins:`(or
//! `gems:`)/`exclude:`/`include:`/`collections:`/`defaults:`/`sass:`/
//! `markdown:`/`highlighter:`/`incremental:`/`livereload:`/`port:`/
//! `host:`/`source:`/`destination:`/`layouts_dir:`/`data_dir:`/
//! `collections_dir:`/`drafts_dir:`/`posts_dir:`/`future:`/
//! `unpublished:`/`timezone:`/`encoding:`/`strict_front_matter:`/
//! `kramdown:`/`liquid:`/`webrick:`/`exclude_from_relocated_files:`.
//!
//! ```rust
//! let c = izanagi_kit::jekyll::Jekyll::parse(
//!     b"title: Blog\ntheme: minima\npermalink: pretty\nplugins:\n").unwrap();
//! assert_eq!(c.keys, 4);
//! ```

/// Jekyll `_config.yml` census.
#[derive(Debug, Clone)]
pub struct Jekyll {
    /// Top-level `key:` entries.
    pub keys: usize,
    /// `- ` list items at any depth.
    pub items: usize,
    /// Jekyll-known top-level keys.
    pub known: usize,
    /// `#` comments.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "title",
    "description",
    "email",
    "url",
    "baseurl",
    "permalink",
    "theme",
    "remote_theme",
    "plugins",
    "gems",
    "exclude",
    "include",
    "collections",
    "defaults",
    "sass",
    "markdown",
    "highlighter",
    "incremental",
    "livereload",
    "port",
    "host",
    "source",
    "destination",
    "layouts_dir",
    "data_dir",
    "collections_dir",
    "drafts_dir",
    "posts_dir",
    "future",
    "unpublished",
    "timezone",
    "encoding",
    "strict_front_matter",
    "kramdown",
    "liquid",
    "webrick",
    "author",
    "lang",
    "excerpt_separator",
    "show_drafts",
    "lsi",
    "limit_posts",
    "detach",
    "watch",
    "safe",
    "serve",
];
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// Whether the buffer looks like a Jekyll config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    let known = t
        .lines()
        .filter(|l| {
            let s = l.trim_end();
            if s.starts_with(' ') || s.starts_with('-') || s.starts_with('#') {
                return false;
            }
            let key = s.split(':').next().unwrap_or("").trim();
            KEYS.contains(&key)
        })
        .count();
    known >= 3
}

impl Jekyll {
    /// Parse a Jekyll config into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            keys: 0,
            items: 0,
            known: 0,
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
            if s.contains(':') {
                c.keys += 1;
                let key = s.split(':').next().unwrap_or("").trim();
                if KEYS.contains(&key) && !s.starts_with(' ') && !s.starts_with('\t') {
                    c.known += 1;
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
    fn parses_jekyll_config() {
        let b = concat!(
            "title: My Blog\n",
            "description: Notes\n",
            "url: https://e.com\n",
            "baseurl: /blog\n",
            "permalink: pretty\n",
            "theme: minima\n",
            "markdown: kramdown\n",
            "highlighter: rouge\n",
            "plugins:\n",
            "  - jekyll-feed\n",
            "  - jekyll-sitemap\n",
            "  - jekyll-seo-tag\n",
            "exclude:\n",
            "  - Gemfile\n",
            "  - vendor\n",
            "# end\n",
        );
        let c = Jekyll::parse(b.as_bytes()).unwrap();
        assert_eq!(c.known, 10);
        assert_eq!(c.keys, 10);
        assert_eq!(c.items, 5);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Jekyll::parse(b"foo: 1\nbar: 2\n").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
