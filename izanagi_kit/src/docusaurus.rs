//! Docusaurus `docusaurus.config.js` census.
//!
//! `docusaurus.config.js`/`docusaurus.config.ts`/`sidebars.js`
//! assign `module.exports = { … }` with `title`, `tagline`, `favicon`,
//! `url`, `baseUrl`, `organizationName`, `projectName`,
//! `deploymentBranch`, `trailingSlash`, `onBrokenLinks`,
//! `onBrokenMarkdownLinks`, `onDuplicateRoutes`, `i18n`, `presets`,
//! `plugins`, `themes`, `themeConfig` (+`navbar`,`footer`,`prism`,
//! `colorMode`,`docs`,`blog`,`algolia`,`announcementBar`,
//! `image`,`metadata`,`headTags`,`scripts`,`stylesheets`,
//! `markdown`,`customFields`,`future`,`experimental_*`).
//!
//! ```rust
//! let c = izanagi_kit::docusaurus::Docusaurus::parse(
//!     b"module.exports = {\n  title: 'x',\n  url: 'https://e.com',\n  baseUrl: '/',\n  themeConfig: {},\n};\n").unwrap();
//! assert_eq!(c.keys, 5);
//! ```

/// `docusaurus.config.js` census.
#[derive(Debug, Clone)]
pub struct Docusaurus {
    /// `key:` / `key =` entries at any depth.
    pub keys: usize,
    /// Docusaurus-known keys.
    pub known: usize,
    /// `themeConfig:` scope entries.
    pub theme_config: usize,
    /// `//`/`/*` comments.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "title",
    "tagline",
    "favicon",
    "url",
    "baseurl",
    "organizationname",
    "projectname",
    "deploymentbranch",
    "trailingslash",
    "onbrokenlinks",
    "onbrokenmarkdownlinks",
    "onbrokenanchors",
    "onduplicateroutes",
    "i18n",
    "presets",
    "plugins",
    "themes",
    "themeconfig",
    "markdown",
    "customfields",
    "future",
    "stylesheets",
    "scripts",
    "headtags",
    "clientmodules",
    "staticdirectories",
    "noconfig",
    "titleDelimiter",
    "tagline",
];

const THEME_KEYS: &[&str] = &[
    "navbar",
    "footer",
    "prism",
    "colormode",
    "docs",
    "blog",
    "algolia",
    "announcementbar",
    "image",
    "metadata",
    "headtags",
    "hideablesidebar",
    "tableofcontents",
    "livecodeblock",
    "respectpreferscolorscheme",
];

/// Whether the buffer looks like a Docusaurus config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    if !(t.contains("module.exports") || t.contains("export default") || t.contains("const config"))
    {
        return false;
    }
    let hits = t
        .lines()
        .filter(|l| {
            let s = l.trim().trim_end_matches(',');
            let key = s
                .split(':')
                .next()
                .unwrap_or("")
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_ascii_lowercase();
            KEYS.contains(&key.as_str()) || THEME_KEYS.contains(&key.as_str())
        })
        .count();
    hits >= 2 && (t.contains("baseUrl") || t.contains("themeConfig"))
}

impl Docusaurus {
    /// Parse a Docusaurus config into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            known: 0,
            theme_config: 0,
            comments: 0,
        };
        let mut in_theme = false;
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with("//") || s.starts_with("/*") || s.starts_with('*') {
                c.comments += 1;
                continue;
            }
            let head = s
                .split(':')
                .next()
                .unwrap_or("")
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_ascii_lowercase();
            if head == "themeconfig" {
                in_theme = true;
            }
            if s.contains(':') || s.contains('=') {
                c.keys += 1;
                if KEYS.contains(&head.as_str()) {
                    c.known += 1;
                }
                if in_theme && s.contains(':') {
                    c.theme_config += 1;
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
    fn parses_docusaurus() {
        let b = concat!(
            "// config\n",
            "module.exports = {\n",
            "  title: 'My Site',\n",
            "  tagline: 'docs',\n",
            "  url: 'https://e.com',\n",
            "  baseUrl: '/',\n",
            "  onBrokenLinks: 'throw',\n",
            "  presets: [],\n",
            "  plugins: [],\n",
            "  themeConfig: {\n",
            "    navbar: {},\n",
            "    footer: {},\n",
            "    prism: {},\n",
            "  },\n",
            "};\n",
        );
        let c = Docusaurus::parse(b.as_bytes()).unwrap();
        assert_eq!(c.known, 8);
        assert_eq!(c.keys, 12);
        assert_eq!(c.theme_config, 4);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Docusaurus::parse(b"module.exports = { x: 1 };\n").is_none());
    }
}
