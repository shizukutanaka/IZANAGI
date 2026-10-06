//! UnoCSS `uno.config.*`/`unocss.config.*` census.
//!
//! `defineConfig({ presets: [presetUno(), presetAttributify(),
//! presetIcons(), presetTypography(), presetWebFonts(), presetWind(),
//! presetWind3(), presetWind4(), presetMini(), presetTagify(),
//! presetRemToPx(), presetScrollbar(), presetLegacyCompat(),
//! presetShadcn(), presetAutoprefixer(), presetFluid()],
//! transformers: [transformerDirectives(), transformerVariantGroup(),
//! transformerCompileClass(), transformerAttributifyJsx()],
//! rules: [[regex, css]], shortcuts: { 'x': 'y' }, theme: {
//! colors/breakpoints/fontFamily/animation/easing/icons/preflights },
//! safelist: ['x'], blocklist: [], exclude: [], include: [],
//! content: { pipeline: { include: [], exclude: [] },
//! filesystem: [] }, preflights: [], layers: {},
//! cli: { entry: [] }, transformers, configDeps, mergeSelectors,
//! autocomplete: { templates: [] } })` or `presetUno()` imports
//! `from 'unocss'`/`'@unocss/*'`.
//!
//! ```rust
//! let k = b"import { defineConfig, presetUno, presetIcons } from 'unocss'\nexport default defineConfig({\n  presets: [presetUno(), presetIcons()],\n  shortcuts: { 'btn': 'px-4 py-1' },\n  theme: { colors: { primary: '#fff' } },\n});\n";
//! assert!(izanagi_kit::unocss::detect(k));
//! ```

/// UnoCSS config census.
#[derive(Debug, Clone)]
pub struct Unocss {
    /// `key:`/`key =` config lines.
    pub settings: usize,
    /// recognised unocss markers present.
    pub keys: usize,
    /// `//`/`/*` comment lines.
    pub comments: usize,
}

const MARKERS: &[&str] = &[
    "presetUno",
    "presetAttributify",
    "presetIcons",
    "presetTypography",
    "presetWebFonts",
    "presetWind",
    "presetWind3",
    "presetWind4",
    "presetMini",
    "presetTagify",
    "presetRemToPx",
    "presetScrollbar",
    "presetLegacyCompat",
    "presetShadcn",
    "presetAutoprefixer",
    "presetFluid",
    "transformerDirectives",
    "transformerVariantGroup",
    "transformerCompileClass",
    "transformerAttributifyJsx",
    "from 'unocss'",
    "from \"unocss\"",
    "'unocss'",
    "\"unocss\"",
    "@unocss/",
    "shortcuts:",
    "rules:",
    "safelist:",
    "blocklist:",
    "preflights:",
    "configDeps",
    "mergeSelectors",
    "autocomplete:",
    "presetSvg",
    "presetFonts",
    "uno.config",
    "unocss.config",
];

fn code(line: &str) -> bool {
    let s = line.trim();
    !s.is_empty()
        && !s.starts_with("//")
        && !s.starts_with('#')
        && !s.starts_with("/*")
        && !s.starts_with('*')
}

/// Detect a `uno.config.*`/`unocss.config.*` file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // `presetUno`/`presetAttributify`/`transformerDirectives`/
    // `'unocss'`/`@unocss/` are unocss-exclusive.
    let mut n = 0usize;
    for line in t.lines() {
        if code(line) && MARKERS.iter().any(|m| line.contains(m)) {
            n += 1;
        }
    }
    n >= 2
}

impl Unocss {
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
        let b = b"import { defineConfig, presetUno, presetIcons } from 'unocss'\nexport default defineConfig({\n  presets: [presetUno(), presetIcons()],\n  shortcuts: { 'btn': 'px-4 py-1' },\n  theme: { colors: { primary: '#fff' } },\n});\n";
        assert!(detect(b));
        let c = Unocss::parse(b).unwrap();
        assert!(c.keys >= 3);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"export default { theme: {} };\n"));
        assert!(!detect(
            b"// presetUno()\n// presetIcons()\nexport default {}\n"
        ));
    }
}
