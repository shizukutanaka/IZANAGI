//! Astro `astro.config.*` census.
//!
//! `defineConfig({ ... })` + keys: `site`, `base`, `trailingSlash`,
//! `output` (`'static'`/`'server'`/`'hybrid'`), `adapter`
//! (`@astrojs/node`/`vercel`/`netlify`/`cloudflare`/`deno`),
//! `integrations` (`@astrojs/tailwind`/`react`/`vue`/`svelte`/`mdx`/
//! `sitemap`/`alpinejs`/`lit`/`preact`/`solid-js`/`partytown`/`markdoc`/
//! `db`/`netlify`/`node`/`cloudflare`/`vercel`/`deno`),
//! `build` (`format`/`client`/`server`/`assets`/`inlineStylesheets`/
//! `split`/`excludeMiddleware`/`concurrency`/`redirects`),
//! `server` (`host`/`port`/`open`), `devToolbar` (`enabled`),
//! `prefetch`, `image` (`service`/`remotePatterns`/`domains`),
//! `markdown` (`shikiConfig`/`syntaxHighlight`/`remarkPlugins`/
//! `rehypePlugins`/`remarkRehype`/`gfm`/`smartypants`/`drafts`),
//! `vite`, `compressHTML`, `scopedStyleStrategy`, `security`
//! (`checkOrigin`/`allowedDomains`/`contentSecurityPolicy`),
//! `env` (`schema`/`validateSecrets`), `experimental` (`contentLayer`/
//! `serverIslands`/`responsiveImages`/`clientPrerender`/`fonts`/
//! `serializeConfig`/`svg`/`sessions`/`headingIdCompat`/
//! `liveContentCollections`/`preserveScriptOrder`/`staticImportMetaEnv`),
//! `i18n` (`defaultLocale`/`locales`/`routing`/`fallback`/`domains`/`detectBrowserLanguage`),
//! `redirects`, `outDir`, `publicDir`, `cacheDir`, `root`,
//! `srcDir`, `trailingSlash`, `styled`/`content`/`routes`/`astro`.
//!
//! ```rust
//! let k = b"export default defineConfig({\n  site: 'https://x.dev',\n  output: 'server',\n  adapter: node(),\n  integrations: [tailwind(), mdx()],\n});\n";
//! assert!(izanagi_kit::astro::detect(k));
//! ```

/// astro.config census.
#[derive(Debug, Clone)]
pub struct Astro {
    /// `key:`/`key =` config lines.
    pub settings: usize,
    /// recognised astro markers present.
    pub keys: usize,
    /// `//`/`/*` comment lines.
    pub comments: usize,
}

const MARKERS: &[&str] = &[
    "defineConfig",
    "@astrojs/",
    "integrations",
    "trailingSlash",
    "compressHTML",
    "scopedStyleStrategy",
    "shikiConfig",
    "syntaxHighlight",
    "remarkPlugins",
    "rehypePlugins",
    "remarkRehype",
    "devToolbar",
    "remotePatterns",
    "inlineStylesheets",
    "excludeMiddleware",
    "serverIslands",
    "responsiveImages",
    "clientPrerender",
    "contentLayer",
    "liveContentCollections",
    "preserveScriptOrder",
    "staticImportMetaEnv",
    "headingIdCompat",
    "defaultLocale",
    "detectBrowserLanguage",
    "checkOrigin",
    "contentSecurityPolicy",
    "validateSecrets",
    "adapter:",
    "output:",
    "site:",
    "prefetch:",
];

fn code(line: &str) -> bool {
    let s = line.trim();
    !s.is_empty()
        && !s.starts_with("//")
        && !s.starts_with('#')
        && !s.starts_with("/*")
        && !s.starts_with('*')
}

/// Detect an `astro.config.*` file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // `@astrojs/`/`integrations`/`shikiConfig`/`devToolbar`/
    // `trailingSlash` are astro-exclusive markers.
    let mut n = 0usize;
    for line in t.lines() {
        if code(line) && MARKERS.iter().any(|m| line.contains(m)) {
            n += 1;
        }
    }
    n >= 2
}

impl Astro {
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
        let b = b"export default defineConfig({\n  site: 'https://x.dev',\n  output: 'server',\n  adapter: node(),\n  integrations: [tailwind(), mdx()],\n});\n";
        assert!(detect(b));
        let c = Astro::parse(b).unwrap();
        assert!(c.keys >= 4);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"export default { name: 'x' };\n"));
        assert!(!detect(
            b"// integrations: []\n// adapter: node()\nsite: x\n"
        ));
    }
}
