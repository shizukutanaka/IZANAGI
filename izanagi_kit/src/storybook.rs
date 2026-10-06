//! Storybook `.storybook/main.ts`/`main.js` census.
//!
//! `main.ts` is JS/TS exporting `StorybookConfig`/`config`:
//! `stories`, `addons` (`@storybook/addon-*`,
//! `@storybook/preset-*`, `@chromatic-com/storybook`,
//! `@storybook/addon-essentials`/`links`/`interactions`/`a11y`/
//! `coverage`/`docs`/`onboarding`/`themes`/`viewport`/`measure`/
//! `outline`/`backgrounds`/`controls`/`actions`/`toolbars`/`jest`),
//! `framework` (`@storybook/react-vite`/`react-webpack5`/
//! `nextjs`/`sveltekit`/`angular`/`vue3`/`html-vite`/`web-components-vite`/
//! `react-native-web-vite`/`server-webpack5`/`experimental-nextjs-vite`/
//! `react-rsbuild`/`vue3-rsbuild`), `core`, `docs` (`autodocs`),
//! `typescript`, `features` (`buildStoriesJson`/`legacyMdx1`),
//! `refs`, `staticDirs`, `viteFinal`, `webpackFinal`,
//! `managerHead`, `previewHead`, `babel`, `swc`, `env`,
//! `logLevel`, `previewAnnotations`, `experimental_indexers`.
//!
//! ```rust
//! let k = b"const config: StorybookConfig = {\n  stories: ['../src/**/*.stories.@(js|ts)'],\n  addons: ['@storybook/addon-essentials'],\n  framework: '@storybook/react-vite',\n  docs: { autodocs: 'tag' },\n};\nexport default config;\n";
//! assert!(izanagi_kit::storybook::detect(k));
//! ```

/// .storybook/main.* census.
#[derive(Debug, Clone)]
pub struct Storybook {
    /// `key:`/`key =` config lines.
    pub settings: usize,
    /// recognised Storybook markers present.
    pub keys: usize,
    /// `//`/`#` comment lines.
    pub comments: usize,
}

const MARKERS: &[&str] = &[
    "stories:",
    "@storybook/",
    "addons:",
    "framework:",
    "autodocs",
    "staticDirs:",
    "viteFinal",
    "webpackFinal",
    "managerHead",
    "previewHead",
    "previewAnnotations",
    "experimental_indexers",
    "buildStoriesJson",
    "legacyMdx1",
    "@chromatic-com/storybook",
    "@storybook/addon-",
    "@storybook/preset-",
    "storyStoreV7",
    "StorybookConfig",
    "satisfies",
    "core:",
    "docs:",
    "features:",
    "typescript:",
    "babel:",
    "swc:",
    "env:",
    "refs:",
    "logLevel:",
];

fn code(line: &str) -> bool {
    let s = line.trim();
    !s.is_empty()
        && !s.starts_with("//")
        && !s.starts_with('#')
        && !s.starts_with("/*")
        && !s.starts_with('*')
}

/// Detect a Storybook `main.ts`/`main.js`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // `stories:`/`addons:`/`@storybook/`/`StorybookConfig` are
    // storybook-exclusive markers.
    let mut n = 0usize;
    for line in t.lines() {
        if code(line) && MARKERS.iter().any(|m| line.contains(m)) {
            n += 1;
        }
    }
    n >= 2
}

impl Storybook {
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
            if s.starts_with("//") || s.starts_with('#') {
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
        let b = b"const config: StorybookConfig = {\n  stories: ['../src/**/*.stories.@(js|ts)'],\n  addons: ['@storybook/addon-essentials'],\n  framework: '@storybook/react-vite',\n  docs: { autodocs: 'tag' },\n};\nexport default config;\n";
        assert!(detect(b));
        let c = Storybook::parse(b).unwrap();
        assert!(c.keys >= 4);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"const x = 1;\nexport default x;\n"));
        assert!(!detect(
            b"// stories: ['x']\n// addons: ['@storybook/a']\nconst y = 1;\n"
        ));
    }
}
