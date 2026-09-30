//! Hugo site configuration census (`config.toml`/`hugo.yaml`).
//!
//! Hugo's site config declares `baseURL`/`baseurl`, `title`, `theme`,
//! `languageCode`, `defaultContentLanguage`, `taxonomies`,
//! `enableRobotsTXT`, `paginate`, `permalinks`, `menus`/`menu`,
//! `[params]`/`[outputs]`/`[markup]`/`[security]`/`[imaging]`/
//! `[mediaTypes]`/`[outputFormats]`/`[caches]`/`[deployment]`/
//! `[sitemap]`/`[minify]` sections and `[[menu.x]]` arrays.
//!
//! ```rust
//! let c = izanagi_kit::hugoconf::Hugoconf::parse(
//!     b"baseURL = \"https://e.com\"\ntitle = \"Site\"\ntheme = \"ananke\"\n").unwrap();
//! assert_eq!(c.keys, 3);
//! ```

/// Hugo config census.
#[derive(Debug, Clone)]
pub struct Hugoconf {
    /// `key = value` / `key:` entries.
    pub keys: usize,
    /// `[section]` headers.
    pub sections: usize,
    /// `[[array]]` table headers.
    pub arrays: usize,
    /// `#`/`//` comments.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "baseurl",
    "title",
    "theme",
    "languagecode",
    "defaultcontentlanguage",
    "defaultcontentlanguageinsubdir",
    "taxonomies",
    "enablerobotstxt",
    "enablegitinfo",
    "enableemoji",
    "paginate",
    "paginatepath",
    "permalinks",
    "menus",
    "menu",
    "summarylength",
    "copyright",
    "author",
    "publishdir",
    "contentdir",
    "layoutdir",
    "themesdir",
    "staticdir",
    "archetypedir",
    "datadir",
    "i18ndir",
    "assetdir",
    "resourcedir",
    "uglyurls",
    "canonifyurls",
    "relativeurls",
    "disablepathtolower",
    "hascloze",
    "timeout",
    "workingdir",
    "modulename",
    "frontmatter",
    "metabd",
];

const SECTIONS: &[&str] = &[
    "params",
    "outputs",
    "markup",
    "markup.goldmark",
    "markup.highlight",
    "markup.tableofcontents",
    "markup.asciidocext",
    "security",
    "security.funcs",
    "security.exec",
    "imaging",
    "mediatypes",
    "outputformats",
    "caches",
    "caches.getjson",
    "caches.getcsv",
    "deployment",
    "deployment.targets",
    "deployment.matchers",
    "sitemap",
    "minify",
    "minify.minifyoutput",
    "privacy",
    "privacy.youtube",
    "privacy.vimeo",
    "privacy.x",
    "privacy.instagram",
    "privacy.googleanalytics",
    "services",
    "services.googleanalytics",
    "services.rss",
    "services.x",
    "frontmatter",
    "module",
    "module.imports",
    "module.mounts",
    "languages",
    "related",
    "related.indices",
];

/// Whether the buffer looks like a Hugo site config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = t
        .lines()
        .filter(|l| {
            let s = l.trim().trim_end_matches(',');
            let key = s
                .split(['=', ':'])
                .next()
                .unwrap_or("")
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_ascii_lowercase();
            KEYS.contains(&key.as_str())
                || SECTIONS
                    .iter()
                    .any(|x| s.trim_start_matches('[').trim_end_matches(']') == *x)
                || s.to_ascii_lowercase().starts_with("baseurl")
        })
        .count();
    hits >= 2 && t.to_ascii_lowercase().contains("baseurl")
}

impl Hugoconf {
    /// Parse a Hugo config into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            sections: 0,
            arrays: 0,
            comments: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') || s.starts_with("//") {
                c.comments += 1;
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
            if s.contains('=') || s.contains(':') {
                c.keys += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hugo_config() {
        let b = concat!(
            "# site\n",
            "baseURL = \"https://example.com\"\n",
            "title = \"My Site\"\n",
            "theme = \"ananke\"\n",
            "languageCode = \"en-us\"\n",
            "defaultContentLanguage = \"en\"\n",
            "enableRobotsTXT = true\n",
            "paginate = 10\n",
            "[taxonomies]\n",
            "[params]\n",
            "[outputs]\n",
            "[[menu.main]]\n",
            "[[menu.footer]]\n",
        );
        let c = Hugoconf::parse(b.as_bytes()).unwrap();
        assert_eq!(c.keys, 7);
        assert_eq!(c.sections, 3);
        assert_eq!(c.arrays, 2);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Hugoconf::parse(b"title = \"x\"\nname = \"y\"\n").is_none());
    }
}
