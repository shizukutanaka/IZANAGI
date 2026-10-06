//! Svelte/SvelteKit `svelte.config.*` census.
//!
//! `export default { kit: { adapter: adapter(), ... } }` or
//! `import adapter from '@sveltejs/adapter-*'` (`adapter-node`/
//! `adapter-static`/`adapter-auto`/`adapter-vercel`/`adapter-netlify`/
//! `adapter-cloudflare`/`adapter-cloudflare-workers`/`adapter-deno`/
//! `adapter-bun`/`adapter-azure`/`adapter-aws`/`adapter-firebase`),
//! `preprocess: vitePreprocess()`/`sveltePreprocess()`/`mdsvex`,
//! `kit` (`paths`/`appDir`/`files`/`alias`/`csp`/`csrf`/`env`/
//! `prerender`/`serviceWorker`/`version`/`inlineStyleThreshold`/
//! `embedded`/`output`/`moduleExtensions`/`routes`/`endpoints`/
//! `typescript`/`router`/`adapter`/`outDir`/`beforeNavigate`/
//! `afterNavigate`/`appTemplates`/`files.lib`/`files.routes`),
//! `compilerOptions` (`dev`/`generate`/`css`/`runes`/`immutable`/
//! `accessors`/`preserveComments`/`compatibility`/`namespace`/
//! `experimental`/`fragments`/`format`/`legacy`/`name`/`cssHash`/
//! `discloseVersion`/`enableSourcemap`/`customElement`/`tag`/`sveltePath`),
//! `extensions`, `package`, `vitePlugin` (`inspector`/`experimental`),
//! `onwarn`, `warn`, `preprocess`/`emitCss`/`preserveWhitespace`.
//!
//! ```rust
//! let k = b"import adapter from '@sveltejs/adapter-node'\nimport { vitePreprocess } from '@sveltejs/vite-plugin-svelte'\nexport default {\n  preprocess: vitePreprocess(),\n  kit: { adapter: adapter(), alias: { '$lib': 'src/lib' } },\n};\n";
//! assert!(izanagi_kit::svelte::detect(k));
//! ```

/// svelte.config census.
#[derive(Debug, Clone)]
pub struct Svelte {
    /// `key:`/`key =` config lines.
    pub settings: usize,
    /// recognised svelte markers present.
    pub keys: usize,
    /// `//`/`/*` comment lines.
    pub comments: usize,
}

const MARKERS: &[&str] = &[
    "@sveltejs/adapter",
    "adapter-node",
    "adapter-static",
    "adapter-auto",
    "adapter-vercel",
    "adapter-netlify",
    "adapter-cloudflare",
    "adapter-deno",
    "adapter-bun",
    "adapter-azure",
    "adapter-aws",
    "adapter-firebase",
    "vitePreprocess",
    "sveltePreprocess",
    "@sveltejs/vite-plugin-svelte",
    "mdsvex",
    "kit:",
    "appDir",
    "inlineStyleThreshold",
    "moduleExtensions",
    "serviceWorker",
    "prerender:",
    "compilerOptions",
    "sveltePath",
    "discloseVersion",
    "enableSourcemap",
    "customElement",
    "preserveComments",
    "preserveWhitespace",
    "emitCss",
    "vitePlugin",
    "kit.adapter",
    "adapter(",
    "vite-plugin-svelte",
    "svelte.config",
];

fn code(line: &str) -> bool {
    let s = line.trim();
    !s.is_empty()
        && !s.starts_with("//")
        && !s.starts_with('#')
        && !s.starts_with("/*")
        && !s.starts_with('*')
}

/// Detect a `svelte.config.*` file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // `@sveltejs/adapter*`/`vitePreprocess`/`kit:`/`mdsvex`/
    // `sveltePreprocess` are svelte-exclusive markers.
    let mut n = 0usize;
    for line in t.lines() {
        if code(line) && MARKERS.iter().any(|m| line.contains(m)) {
            n += 1;
        }
    }
    n >= 2
}

impl Svelte {
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
        let b = b"import adapter from '@sveltejs/adapter-node'\nimport { vitePreprocess } from '@sveltejs/vite-plugin-svelte'\nexport default {\n  preprocess: vitePreprocess(),\n  kit: { adapter: adapter(), alias: { '$lib': 'src/lib' } },\n};\n";
        assert!(detect(b));
        let c = Svelte::parse(b).unwrap();
        assert!(c.keys >= 4);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"export default { name: 'x' };\n"));
        assert!(!detect(
            b"// kit: {}\n// adapter: adapter()\nexport default {}\n"
        ));
    }
}
