//! PostCSS config census (`postcss.config.*`/`.postcssrc*`).
//!
//! JS object `{ plugins: { autoprefixer: {}, tailwindcss: {},
//! 'postcss-preset-env': {}, cssnano: {}, 'postcss-import': {},
//! 'postcss-nested': {}, 'postcss-custom-properties': {},
//! 'postcss-mixins': {}, 'postcss-extend': {},
//! 'postcss-for': {}, 'postcss-each': {}, 'postcss-conditionals': {},
//! 'postcss-media-minmax': {}, 'postcss-nesting': {},
//! 'postcss-preset-env': {}, 'postcss-color-function': {},
//! 'postcss-flexbugs-fixes': {}, 'postcss-gap-properties': {},
//! 'postcss-hexrgba': {}, 'postcss-pxtorem': {},
//! 'postcss-reporter': {}, 'postcss-url': {}, 'postcss-utilities': {},
//! 'postcss-simple-vars': {}, '@tailwindcss/postcss': {},
//! 'postcss-plugin': ... } }` or `plugins: [require('x')]`;
//! `.postcssrc` JSON `{"plugins": {...}}`; YAML `plugins:` +
//! plugin-name keys. Recognised plugin keys are postcss-exclusive.
//!
//! ```rust
//! let k = b"module.exports = {\n  plugins: {\n    tailwindcss: {},\n    autoprefixer: {},\n    'postcss-preset-env': {},\n  },\n};\n";
//! assert!(izanagi_kit::postcss::detect(k));
//! ```

/// PostCSS config census.
#[derive(Debug, Clone)]
pub struct Postcss {
    /// plugin-name lines.
    pub plugins: usize,
    /// `key:`/`key =` config lines.
    pub settings: usize,
    /// `//`/`#`/`/*` comment lines.
    pub comments: usize,
}

const PLUGINS: &[&str] = &[
    "autoprefixer",
    "tailwindcss",
    "postcss-preset-env",
    "cssnano",
    "postcss-import",
    "postcss-nested",
    "postcss-nesting",
    "postcss-custom-properties",
    "postcss-mixins",
    "postcss-extend",
    "postcss-for",
    "postcss-each",
    "postcss-conditionals",
    "postcss-media-minmax",
    "postcss-color-function",
    "postcss-flexbugs-fixes",
    "postcss-gap-properties",
    "postcss-hexrgba",
    "postcss-pxtorem",
    "postcss-reporter",
    "postcss-url",
    "postcss-utilities",
    "postcss-simple-vars",
    "@tailwindcss/postcss",
    "postcss-discard-comments",
    "postcss-discard-duplicates",
    "postcss-discard-empty",
    "postcss-discard-overridden",
    "postcss-merge-rules",
    "postcss-minify-selectors",
    "postcss-normalize",
    "postcss-ordered-values",
    "postcss-reduce-transforms",
    "postcss-svgo",
    "postcss-zindex",
    "postcss-cli",
    "postcss-replace-overflow-wrap",
    "postcss-selector-not",
    "postcss-dir-pseudo-class",
    "postcss-logical",
    "postcss-lab-function",
    "postcss-icss",
    "postcss-modules",
    "postcss-font-magician",
    "postcss-center",
    "postcss-clearfix",
    "postcss-vertical-rhythm",
    "postcss-responsive-type",
    "postcss-write-svg",
    "postcss-will-change",
    "postcss-sprites",
    "postcss-darker-than-black",
];

fn code(line: &str) -> bool {
    let s = line.trim();
    !s.is_empty()
        && !s.starts_with("//")
        && !s.starts_with('#')
        && !s.starts_with("/*")
        && !s.starts_with('*')
}

/// Detect a PostCSS config file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // `postcss-*`/`autoprefixer`/`cssnano`/`tailwindcss` plugin
    // names under a `plugins` key are postcss-exclusive.
    let mut n = 0usize;
    for line in t.lines() {
        if code(line) && PLUGINS.iter().any(|p| line.contains(p)) {
            n += 1;
        }
    }
    n >= 2
}

impl Postcss {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            plugins: 0,
            settings: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with("//") || s.starts_with('#') || s.starts_with("/*") {
                c.comments += 1;
                continue;
            }
            if s.contains(':') || s.contains('=') {
                c.settings += 1;
            }
            if PLUGINS.iter().any(|p| line.contains(p)) {
                c.plugins += 1;
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
        let b = b"module.exports = {\n  plugins: {\n    tailwindcss: {},\n    autoprefixer: {},\n    'postcss-preset-env': {},\n  },\n};\n";
        assert!(detect(b));
        let c = Postcss::parse(b).unwrap();
        assert_eq!(c.plugins, 3);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"plugins: { x: {}, y: {} }\n"));
        assert!(!detect(
            b"// autoprefixer: {}\n// cssnano: {}\nplugins: {}\n"
        ));
    }
}
