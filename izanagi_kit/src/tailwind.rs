//! Tailwind CSS `tailwind.config.*` census.
//!
//! JS/TS object literal: `content` (`'./src/**/*.{html,js}'`),
//! `theme` (`extend`, `colors`, `fontFamily`, `fontSize`,
//! `spacing`, `screens`, `container`, `borderRadius`, `boxShadow`,
//! `backgroundImage`, `keyframes`, `animation`, `transitionDuration`,
//! `zIndex`, `opacity`, `aspectRatio`, `gridTemplateColumns`,
//! `maxWidth`, `minHeight`, `letterSpacing`, `lineHeight`,
//! `listStyleType`, `objectPosition`, `willChange`, `accentColor`,
//! `caretColor`, `scrollMargin`, `textIndent`, `aria`, `supports`),
//! `plugins` (`require('@tailwindcss/typography')`/`forms`/`line-clamp`/
//! `container-queries`/`aspect-ratio`), `darkMode` (`'class'`/`'media'`),
//! `prefix`, `important`, `separator`, `safelist`, `blocklist`,
//! `presets`, `corePlugins`, `variantOrder`, `variants`,
//! `mode: 'jit'`, `future`, `experimental`, `satisfies Config`,
//! `export default`.
//!
//! ```rust
//! let k = b"export default {\n  content: ['./src/**/*.{html,js}'],\n  darkMode: 'class',\n  theme: {\n    extend: { colors: { primary: '#fff' } },\n  },\n  plugins: [require('@tailwindcss/forms')],\n};\n";
//! assert!(izanagi_kit::tailwind::detect(k));
//! ```

/// tailwind.config census.
#[derive(Debug, Clone)]
pub struct Tailwind {
    /// `key:`/`key =` config lines.
    pub settings: usize,
    /// recognised tailwind keys present.
    pub keys: usize,
    /// `//`/`/*` comment lines.
    pub comments: usize,
}

const STRONG: &[&str] = &[
    "content:",
    "darkMode",
    "safelist",
    "blocklist",
    "corePlugins",
    "variantOrder",
    "@tailwindcss/",
    "fontFamily",
    "fontSize",
    "fontWeight",
    "borderRadius",
    "borderWidth",
    "boxShadow",
    "backgroundImage",
    "backgroundColor",
    "backgroundPosition",
    "backgroundSize",
    "backgroundOpacity",
    "keyframes",
    "animation",
    "transitionDuration",
    "transitionTimingFunction",
    "transitionProperty",
    "transitionDelay",
    "aspectRatio",
    "gridTemplateColumns",
    "gridTemplateRows",
    "gridColumn",
    "gridRow",
    "maxWidth",
    "minHeight",
    "minWidth",
    "maxHeight",
    "letterSpacing",
    "lineHeight",
    "listStyleType",
    "objectPosition",
    "willChange",
    "accentColor",
    "caretColor",
    "scrollMargin",
    "scrollPadding",
    "textIndent",
    "textOpacity",
    "textDecorationColor",
    "placeholderColor",
    "outlineColor",
    "ringColor",
    "ringOffsetColor",
    "divideColor",
    "borderColor",
    "strokeWidth",
    "fill",
    "stroke",
    "blur",
    "brightness",
    "contrast",
    "dropShadow",
    "grayscale",
    "hueRotate",
    "invert",
    "saturate",
    "sepia",
    "backdropBlur",
    "zIndex",
    "order",
    "flexGrow",
    "flexShrink",
    "extend:",
    "mode: 'jit'",
    "mode: \"jit\"",
];

const WEAK: &[&str] = &[
    "theme",
    "plugins",
    "extend",
    "colors",
    "spacing",
    "screens",
    "container",
    "prefix",
    "important",
    "separator",
    "presets",
    "variants",
    "future",
    "experimental",
    "opacity",
    "margin",
    "padding",
    "width",
    "height",
    "font",
    "shadow",
    "module.exports",
    "export default",
    "require(",
];

fn marker(line: &str) -> bool {
    let s = line.trim();
    if s.is_empty()
        || s.starts_with("//")
        || s.starts_with("/*")
        || s.starts_with('*')
        || s.starts_with('#')
    {
        return false;
    }
    if STRONG.iter().any(|m| line.contains(m)) {
        return true;
    }
    false
}

fn weak_hit(line: &str) -> bool {
    let s = line.trim();
    WEAK.iter()
        .any(|w| s.starts_with(w) || s.contains(&format!("{w}:")))
}

/// Detect a `tailwind.config.*` file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // `content:`+`theme`+`darkMode`/`@tailwindcss/`/`safelist`
    // are tailwind-exclusive combos.
    let mut strong = 0usize;
    let mut weak = 0usize;
    for line in t.lines() {
        if marker(line) {
            strong += 1;
        } else if weak_hit(line) {
            weak += 1;
        }
    }
    strong >= 2 || (strong >= 1 && weak >= 2)
}

impl Tailwind {
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
            if marker(line) || weak_hit(line) {
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
        let b = b"export default {\n  content: ['./src/**/*.{html,js}'],\n  darkMode: 'class',\n  theme: {\n    extend: { colors: { primary: '#fff' } },\n  },\n  plugins: [require('@tailwindcss/forms')],\n};\n";
        assert!(detect(b));
        let c = Tailwind::parse(b).unwrap();
        assert!(c.keys >= 4);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(
            b"const theme = { colors: {} };\nconst plugins = [];\n"
        ));
        assert!(!detect(
            b"// content: ['./x']\n// darkMode: 'class'\ntheme: {}\n"
        ));
    }
}
