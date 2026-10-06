//! Nuxt `nuxt.config.*` census.
//!
//! `defineNuxtConfig({ ... })` wrapper + keys: `modules`
//! (`'@nuxtjs/tailwindcss'`/`'@pinia/nuxt'`/`'nuxt-icon'`/`'@nuxt/content'`/
//! `'@nuxt/image'`/`'@nuxtjs/i18n'`/`'nuxt-security'`/`'@nuxt/fonts'`/
//! `'@nuxt/eslint'`/`'@vueuse/nuxt'`/`'@nuxt/ui'`/`'@nuxtjs/color-mode'`/
//! `'@nuxtjs/google-fonts'`/`'nuxt-og-image'`/`'@nuxtjs/mdc'`/`'nuxt-auth-utils'`),
//! `ssr`, `devtools`, `app` (`head`/`pageTransition`/`layoutTransition`),
//! `css`, `components`, `imports` (`dirs`/`autoImport`/`presets`),
//! `pages`, `nitro` (`preset`/`routeRules`/`storage`/`prerender`),
//! `routeRules`, `runtimeConfig` (`public`), `vite`, `webpack`,
//! `postcss`, `typescript`, `hooks`, `build`, `buildDir`,
//! `srcDir`, `serverDir`, `dir`, `extensions`, `compatibilityDate`,
//! `future`, `experimental`, `features`, `telemetry`, `devtools`,
//! `sourcemap`, `alias`, `ignore`, `watch`, `appConfig`,
//! `vue` (`compilerOptions`), `router` (`options`), `i18n`,
//! `image`, `content`, `ui`, `colorMode`, `fonts`, `eslint`,
//! `extends`, `layers`, `devServer`, `generate`, `test`, `ogImage`.
//!
//! ```rust
//! let k = b"export default defineNuxtConfig({\n  modules: ['@nuxtjs/tailwindcss'],\n  ssr: true,\n  devtools: { enabled: true },\n  nitro: { preset: 'vercel' },\n});\n";
//! assert!(izanagi_kit::nuxt::detect(k));
//! ```

/// nuxt.config census.
#[derive(Debug, Clone)]
pub struct Nuxt {
    /// `key:`/`key =` config lines.
    pub settings: usize,
    /// recognised nuxt markers present.
    pub keys: usize,
    /// `//`/`/*` comment lines.
    pub comments: usize,
}

const MARKERS: &[&str] = &[
    "defineNuxtConfig",
    "@nuxtjs/",
    "@nuxt/",
    "nuxt-",
    "modules:",
    "devtools",
    "nitro",
    "routeRules",
    "runtimeConfig",
    "compatibilityDate",
    "pageTransition",
    "layoutTransition",
    "autoImport",
    "colorMode",
    "ogImage",
    "devServer",
    "buildDir",
    "srcDir",
    "serverDir",
    "appConfig",
    "router:",
    "imports:",
    "ssr:",
    "extends:",
    "layers:",
];

fn code(line: &str) -> bool {
    let s = line.trim();
    !s.is_empty()
        && !s.starts_with("//")
        && !s.starts_with('#')
        && !s.starts_with("/*")
        && !s.starts_with('*')
}

/// Detect a `nuxt.config.*` file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // `defineNuxtConfig` is nuxt-exclusive; `@nuxt*`/`nuxt-*`
    // modules + `nitro`/`routeRules`/`runtimeConfig` reinforce.
    let mut n = 0usize;
    for line in t.lines() {
        if code(line) && MARKERS.iter().any(|m| line.contains(m)) {
            n += 1;
        }
    }
    n >= 2
}

impl Nuxt {
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
        let b = b"export default defineNuxtConfig({\n  modules: ['@nuxtjs/tailwindcss'],\n  ssr: true,\n  devtools: { enabled: true },\n  nitro: { preset: 'vercel' },\n});\n";
        assert!(detect(b));
        let c = Nuxt::parse(b).unwrap();
        assert!(c.keys >= 4);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"export default { ssr: true };\n"));
        assert!(!detect(
            b"// defineNuxtConfig({\n// modules: ['@nuxt/x']\nssr: true\n"
        ));
    }
}
