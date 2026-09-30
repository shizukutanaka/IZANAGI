//! Rofi config/theme (`.rasi`/config.rasi) census.
//!
//! A rofi file is a rasi dictionary: `name: value;` properties inside
//! blocks `configuration {}`/`window {}`/`mainbox {}`/`inputbar {}`/
//! `entry {}`/`listview {}`/`element {}`/`element-text {}`/
//! `element-icon {}`/`mode-switcher {}`/`message {}`/`textbox {}`/
//! `button {}`/`iconbox {}`/`case-indicator {}`/`prompt {}`/`num-* {}`/
//! `alternate {}`/`urgent {}`/`active {}`/`selected {}`/`normal {}`/
//! `error-message {}`/`sidebar {}`/`inputbar {}`/`dummy {}`/`split {}`/
//! `stack {}`/`box {}`/`overlay {}`/`child {}`/`enabled {}`/`horizontal {}`
//! plus `theme:`/`@theme`/`@import`/`@media`/`@property`/`configuration`/
//! `@import url(...)`/`@default`/`* {}`/hovered/selected/alternate/urgent
//! modifiers, `calc()`/`var()`/`env()`/`@property <name>` and color
//! literals (`rgba`/`#`/`argb:`/`lighten`/`darken`/`transparent`).
//! Older `!`-style `rofi.` prefs (`rofi.color-enabled`) also count.
//!
//! ```rust
//! let r = concat!(
//!     "configuration {\n",
//!     "    modi: \"drun,run\";\n",
//!     "    font: \"mono 12\";\n",
//!     "}\n",
//! );
//! let c = izanagi_kit::rofi::Rofi::parse(r.as_bytes()).unwrap();
//! assert_eq!(c.props, 2);
//! ```

/// Rofi config/theme census.
#[derive(Debug, Clone)]
pub struct Rofi {
    /// Named/`{}` block openers (`window {`, `element-text {`, `* {`, `element selected {`, `@media {...}`).
    pub blocks: usize,
    /// `key: value;` property lines + legacy `rofi.`/`!` prefix lines.
    pub props: usize,
    /// `@theme`/`@import`/`@media`/`@property`/`@default`/`@include`/`theme:` directives.
    pub directives: usize,
    /// `calc(`/`var(`/`env(`/`rgba(`/`hsla(`/`hsl(`/`lighten(`/`darken(`/`url(`/`linear-gradient(`/`argb:`/`#` color/function tokens.
    pub funcs: usize,
}

/// Whether the buffer looks like a rofi config or theme.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("rofi.")
        || t.contains("@theme")
        || (t.contains("configuration {") && t.contains("modi"))
        || t.contains("mainbox {")
        || t.contains("inputbar {")
        || t.contains("listview {")
}

impl Rofi {
    /// Parse a rofi config/theme into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            blocks: 0,
            props: 0,
            directives: 0,
            funcs: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() || s.starts_with("//") || s.starts_with("/*") || s == "}" || s == "{" {
                continue;
            }
            if s.starts_with('@') {
                c.directives += 1;
                continue;
            }
            c.funcs += s.matches("calc(").count()
                + s.matches("var(").count()
                + s.matches("env(").count()
                + s.matches("rgba(").count()
                + s.matches("hsla(").count()
                + s.matches("hsl(").count()
                + s.matches("lighten(").count()
                + s.matches("darken(").count()
                + s.matches("url(").count()
                + s.matches("linear-gradient(").count()
                + s.matches("argb:").count()
                + s.matches('#').count();
            if s.ends_with('{') {
                c.blocks += 1;
                continue;
            }
            if s.contains(':') && s.ends_with(';') {
                c.props += 1;
                continue;
            }
            if s.starts_with("rofi.") || s.starts_with('!') {
                c.props += 1;
                continue;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_theme() {
        let b = concat!(
            "@theme \"/usr/share/rofi/themes/arthur.rasi\"\n",
            "@import \"colors.rasi\"\n",
            "configuration {\n",
            "    modi: \"drun,run,window\";\n",
            "    font: \"mono 12\";\n",
            "    show-icons: true;\n",
            "    display-drun: \"Apps\";\n",
            "    kb-row-up: \"Up,Control+k\";\n",
            "}\n",
            "window {\n",
            "    width: 640px;\n",
            "    border-radius: 8px;\n",
            "    background-color: rgba(0, 0, 0, 0.8);\n",
            "}\n",
            "element selected {\n",
            "    background-color: var(accent, #ff0000);\n",
            "}\n",
            "rofi.color-enabled: true\n",
        );
        let c = Rofi::parse(b.as_bytes()).unwrap();
        assert_eq!(c.blocks, 3);
        assert_eq!(c.directives, 2);
        assert!(c.props >= 9);
        assert!(c.funcs >= 3);
    }

    #[test]
    fn rejects_other() {
        assert!(Rofi::parse(b"foo = 1").is_none());
    }
}
