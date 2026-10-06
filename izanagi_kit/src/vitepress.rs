//! VitePress `.vitepress/config.*` census.
//!
//! `defineConfig({ ... })` + keys: `title`, `description`,
//! `themeConfig` (`nav`/`sidebar`/`socialLinks`/`footer`/`editLink`/
//! `search`/`logo`/`siteTitle`/`outline`/`docFooter`/`lastUpdated`/
//! `carbonAds`/`algolia`/`localSearch`/`prev`/`next`/`darkModeSwitchLabel`/
//! `returnToTopLabel`/`sidebarMenuLabel`/`externalLinkIcon`/
//! `notFound`/`i18nRouting`), `base`, `srcDir`, `srcExclude`,
//! `outDir`, `cacheDir`, `cleanUrls`, `rewrites`, `locales`,
//! `lang`, `head`, `metaChunk`, `appearance`, `lastUpdated`,
//! `markdown` (`theme`/`lineNumbers`/`math`/`config`/`toc`/
//! `container`/`codeTransformers`/`languages`/`highlight`),
//! `vite`, `vue`, `transformHead`, `transformHtml`,
//! `transformPageData`, `buildEnd`, `postRender`, `mpa`,
//! `ignoreDeadLinks`, `shouldPreload`, `sitemap`, `titleTemplate`,
//! `cleanUrls`, `useWebFonts`.
//!
//! ```rust
//! let k = b"export default defineConfig({\n  title: 'X',\n  themeConfig: {\n    nav: [{ text: 'a', link: '/' }],\n    sidebar: { '/': [] },\n    socialLinks: [],\n  },\n  cleanUrls: true,\n});\n";
//! assert!(izanagi_kit::vitepress::detect(k));
//! ```

/// vitepress config census.
#[derive(Debug, Clone)]
pub struct Vitepress {
    /// `key:`/`key =` config lines.
    pub settings: usize,
    /// recognised vitepress markers present.
    pub keys: usize,
    /// `//`/`/*` comment lines.
    pub comments: usize,
}

const MARKERS: &[&str] = &[
    "defineConfig",
    "themeConfig",
    "socialLinks",
    "editLink",
    "carbonAds",
    "algolia",
    "localSearch",
    "docFooter",
    "darkModeSwitchLabel",
    "returnToTopLabel",
    "sidebarMenuLabel",
    "externalLinkIcon",
    "notFound",
    "i18nRouting",
    "cleanUrls",
    "rewrites:",
    "locales:",
    "metaChunk",
    "srcExclude",
    "transformHead",
    "transformHtml",
    "transformPageData",
    "postRender",
    "ignoreDeadLinks",
    "shouldPreload",
    "titleTemplate",
    "useWebFonts",
    "codeTransformers",
    "lineNumbers",
    "vitepress",
    "siteTitle",
    "outline:",
    "footer:",
    "search:",
    "nav:",
    "sidebar:",
];

fn code(line: &str) -> bool {
    let s = line.trim();
    !s.is_empty()
        && !s.starts_with("//")
        && !s.starts_with('#')
        && !s.starts_with("/*")
        && !s.starts_with('*')
}

/// Detect a VitePress `config.*`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // `themeConfig`/`socialLinks`/`cleanUrls`/`defineConfig`
    // + docNav labels are vitepress-exclusive.
    let mut n = 0usize;
    for line in t.lines() {
        if code(line) && MARKERS.iter().any(|m| line.contains(m)) {
            n += 1;
        }
    }
    n >= 2
}

impl Vitepress {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            settings: 0,
            keys: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with("//") || s.starts_with("/*") || s.starts_with('*') {
                c.comments += 1;
                continue;
            }
            if s.contains(':') || s.contains('=') {
                c.settings += 1;
            }
            if MARKERS.iter().any(|m| line.contains(m)) {
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
    fn detects() {
        let b = b"export default defineConfig({\n  title: 'X',\n  themeConfig: {\n    nav: [{ text: 'a', link: '/' }],\n    sidebar: { '/': [] },\n    socialLinks: [],\n  },\n  cleanUrls: true,\n});\n";
        assert!(detect(b));
        let c = Vitepress::parse(b).unwrap();
        assert!(c.keys >= 4);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"export default { title: 'x' };\n"));
        assert!(!detect(
            b"// themeConfig: { nav: [] }\n// cleanUrls: true\ntitle: x\n"
        ));
    }
}
