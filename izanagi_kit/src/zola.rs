//! Zola `config.toml` census.
//!
//! Zola requires `base_url =` and accepts `title =`/`description =`/
//! `theme =`/`default_language =`/`languages`, `author`,
//! `generate_feed`/`generate_feeds`/`feed_filename`/`feed_filenames`,
//! `taxonomies`, `compile_sass`, `build_search_index`,
//! `minify_html`/`minify_output`, `highlight_code`/`highlight_theme`/
//! `highlight_themes_css`, `ignored_content`, `ignored_content_globset`,
//! `link_checker`/`skip_prefixes`/`internal_level`,
//! `slugify`/`paths_keep_dates`/`paths`/`anchors`,
//! `search_output_format`, `hard_link_static`,
//! `[markdown]`/`[link_checker]`/`[slugify]`/`[search]`/`[imaging]`/
//! `[taxonomies]`/`[extra]`/`[theme]`/`[translations]`/`[languages.*]`
//! sections and `extra`/`output_dir`/`mode =` values.
//!
//! ```rust
//! let c = izanagi_kit::zola::Zola::parse(
//!     b"base_url = \"https://e.com\"\ntitle = \"z\"\ntheme = \"x\"\n[markdown]\n[extra]\n").unwrap();
//! assert_eq!(c.keys, 3);
//! ```

/// Zola `config.toml` census.
#[derive(Debug, Clone)]
pub struct Zola {
    /// `key = value` entries.
    pub keys: usize,
    /// Zola-known top-level keys.
    pub known: usize,
    /// `[section]` headers.
    pub sections: usize,
    /// `[[array]]` headers.
    pub arrays: usize,
}

const KEYS: &[&str] = &[
    "base_url",
    "title",
    "description",
    "theme",
    "default_language",
    "author",
    "generate_feed",
    "generate_feeds",
    "feed_filename",
    "feed_filenames",
    "taxonomies",
    "compile_sass",
    "build_search_index",
    "minify_html",
    "minify_output",
    "highlight_code",
    "highlight_theme",
    "highlight_themes_css",
    "ignored_content",
    "ignored_content_globset",
    "skip_prefixes",
    "internal_level",
    "paths_keep_dates",
    "anchors",
    "search_output_format",
    "hard_link_static",
    "output_dir",
    "content_dir",
    "sass_dir",
    "static_dir",
    "templates_dir",
    "themes_dir",
    "taxonomy_kind",
    "smart_punctuation",
    "render_async",
    "insert_anchor_links",
    "lazy_async_image",
    "external_code_block",
    "bottom_footnotes",
    "extra_syntaxes_and_themes",
    "mode",
    "pretty_url",
    "preserve_dotfiles_in_output",
];

const SECTIONS: &[&str] = &[
    "markdown",
    "link_checker",
    "slugify",
    "search",
    "imaging",
    "taxonomies",
    "extra",
    "theme",
    "translations",
    "languages",
    "output_formats",
];

/// Whether the buffer looks like a Zola `config.toml`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    if !t.lines().any(|l| l.trim_start().starts_with("base_url")) {
        return false;
    }
    t.lines()
        .filter(|l| {
            let s = l.trim();
            let key = s.split('=').next().unwrap_or("").trim();
            KEYS.contains(&key)
                || SECTIONS
                    .iter()
                    .any(|x| s.trim_start_matches('[').trim_end_matches(']') == *x)
        })
        .count()
        >= 2
}

impl Zola {
    /// Parse a Zola config into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            known: 0,
            sections: 0,
            arrays: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() || s.starts_with('#') {
                continue;
            }
            if s.starts_with("[[") {
                c.arrays += 1;
                continue;
            }
            if s.starts_with('[') {
                c.sections += 1;
                continue;
            }
            if !s.contains('=') {
                continue;
            }
            c.keys += 1;
            let key = s.split('=').next().unwrap_or("").trim();
            if KEYS.contains(&key) {
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
    fn parses_zola_config() {
        let b = concat!(
            "base_url = \"https://e.com\"\n",
            "title = \"Zola Site\"\n",
            "description = \"test\"\n",
            "theme = \"apollo\"\n",
            "default_language = \"en\"\n",
            "compile_sass = true\n",
            "build_search_index = true\n",
            "minify_html = true\n",
            "generate_feeds = true\n",
            "highlight_code = true\n",
            "hard_link_static = false\n",
            "ignored_content = [\"*.txt\"]\n",
            "taxonomies = [\n",
            "[markdown]\n",
            "[link_checker]\n",
            "[slugify]\n",
            "[extra]\n",
            "[[languages]]\n",
        );
        let c = Zola::parse(b.as_bytes()).unwrap();
        assert_eq!(c.known, 13);
        assert_eq!(c.keys, 13);
        assert_eq!(c.sections, 4);
        assert_eq!(c.arrays, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Zola::parse(b"[package]\nname = \"x\"\n").is_none());
    }
}
