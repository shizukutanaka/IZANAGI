//! Hexo `_config.yml` census.
//!
//! Hexo's config is YAML with `title:`/`subtitle:`/`description:`/
//! `keywords:`/`author:`/`language:`/`timezone:`, `url:`/`root:`/
//! `permalink:`/`permalink_defaults:`/`pretty_urls:`, `source_dir:`/
//! `public_dir:`/`tag_dir:`/`archive_dir:`/`category_dir:`/
//! `code_dir:`/`i18n_dir:`/`skip_render:`, `new_post_name:`/
//! `default_layout:`/`titlecase:`/`external_link:`/`filename_case:`/
//! `render_drafts:`/`post_asset_folder:`/`relative_link:`/`future:`/
//! `highlight:`/`prismjs:`/`prettier:`/`auto_spacing:`/`line_number:`/
//! `tab_replace:`/`wrap:`/`hljs:`, `index_generator:`/
//! `category_generator:`/`tag_generator:`/`archive_generator:`,
//! `default_category:`/`category_map:`/`tag_map:`, `date_format:`/
//! `time_format:`/`updated_option:`, `per_page:`/`pagination_dir:`,
//! `theme:`/`theme_config:`/`deploy:`/`include:`/`exclude:`/
//! `ignore:`/`meta_generator:`/`server:`/`static:`/`limit:`/
//! `local_search:`/`feed:`/`sitemap:`/`baidu_url_submit:`.
//!
//! ```rust
//! let c = izanagi_kit::hexo::Hexo::parse(
//!     b"title: Blog\npermalink: :year/:title/\ntheme: landscape\nper_page: 10\n").unwrap();
//! assert_eq!(c.known, 4);
//! ```

/// Hexo `_config.yml` census.
#[derive(Debug, Clone)]
pub struct Hexo {
    /// `key:` entries at any depth.
    pub keys: usize,
    /// Hexo-known top-level keys.
    pub known: usize,
    /// `*_generator:` scope entries (`index`/`category`/`tag`/`archive`).
    pub generators: usize,
    /// `- ` list items.
    pub items: usize,
}

const KEYS: &[&str] = &[
    "title",
    "subtitle",
    "description",
    "keywords",
    "author",
    "language",
    "timezone",
    "url",
    "root",
    "permalink",
    "permalink_defaults",
    "pretty_urls",
    "source_dir",
    "public_dir",
    "tag_dir",
    "archive_dir",
    "category_dir",
    "code_dir",
    "i18n_dir",
    "skip_render",
    "new_post_name",
    "default_layout",
    "titlecase",
    "external_link",
    "filename_case",
    "render_drafts",
    "post_asset_folder",
    "relative_link",
    "future",
    "highlight",
    "prismjs",
    "prettier",
    "auto_spacing",
    "line_number",
    "tab_replace",
    "wrap",
    "hljs",
    "index_generator",
    "category_generator",
    "tag_generator",
    "archive_generator",
    "default_category",
    "category_map",
    "tag_map",
    "date_format",
    "time_format",
    "updated_option",
    "per_page",
    "pagination_dir",
    "theme",
    "theme_config",
    "deploy",
    "include",
    "exclude",
    "ignore",
    "meta_generator",
    "server",
    "static",
    "local_search",
    "feed",
    "sitemap",
];

const GENS: &[&str] = &[
    "index_generator",
    "category_generator",
    "tag_generator",
    "archive_generator",
];
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// Whether the buffer looks like a Hexo `_config.yml`.
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
            KEYS.contains(&key) || GENS.contains(&key)
        })
        .count();
    known >= 3 && t.contains("permalink")
}

impl Hexo {
    /// Parse a Hexo config into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            keys: 0,
            known: 0,
            generators: 0,
            items: 0,
        };
        for l in t.lines() {
            let s = l.trim_end();
            if s.trim().is_empty() || s.trim_start().starts_with('#') {
                continue;
            }
            if s.trim_start().starts_with("- ") || s.trim_start() == "-" {
                c.items += 1;
                continue;
            }
            if !s.contains(':') {
                continue;
            }
            c.keys += 1;
            let key = s.split(':').next().unwrap_or("").trim();
            if GENS.contains(&key) {
                c.generators += 1;
            } else if KEYS.contains(&key) && !s.starts_with(' ') && !s.starts_with('\t') {
                c.known += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hexo_config() {
        let b = concat!(
            "title: Hexo Site\n",
            "subtitle: blog\n",
            "description: test\n",
            "author: me\n",
            "language: en\n",
            "url: https://e.com\n",
            "root: /\n",
            "permalink: :year/:month/:day/:title/\n",
            "source_dir: source\n",
            "public_dir: public\n",
            "skip_render: README.md\n",
            "new_post_name: :title.md\n",
            "default_layout: post\n",
            "theme: landscape\n",
            "per_page: 10\n",
            "pagination_dir: page\n",
            "index_generator:\n",
            "  path: ''\n",
            "category_generator:\n",
            "  per_page: 10\n",
            "deploy:\n",
            "  type: git\n",
            "  repo: git@e.com:x.git\n",
            "  - extra\n",
        );
        let c = Hexo::parse(b.as_bytes()).unwrap();
        assert_eq!(c.known, 17);
        assert_eq!(c.keys, 23);
        assert_eq!(c.generators, 2);
        assert_eq!(c.items, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Hexo::parse(b"foo: 1\n").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
