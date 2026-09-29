//! Parser for Alacritty configuration files (`alacritty.toml` / `alacritty.yml`).
//!
//! Counts `[font]`/`[window]`/`[colors]`/`[cursor]`/`[mouse]`/`[keyboard]`/
//! `[scrolling]`/`[bell]`/`[env]`/`[hints]`/`[selection]`/`[terminal]`/`[general]`
//! sections, `key:value`/`key = value` options, `key_bindings`/`mouse_bindings`
//! entries, `import`/`live_config_reload`, and comments.
//!
//! ```
//! let b = b"[font]\nsize = 12\n[window]\npadding.x = 2\n";
//! assert!(izanagi_kit::alacritty::detect(b));
//! let c = izanagi_kit::alacritty::Alacritty::parse(b).unwrap();
//! assert_eq!(c.sections, 2);
//! ```

/// Parsed alacritty config summary.
#[derive(Debug, Clone)]
pub struct Alacritty {
    /// `[section]` headers counted.
    pub sections: usize,
    /// `[font]`/`[font.normal]`/`[font.bold]`/`[font.italic]`/`[font.bold_italic]`/`[font.offset]`/`[font.glyph_offset]`.
    pub font_sections: usize,
    /// `[window]`/`[window.padding]`/`[window.dimensions]`/`[window.class]`.
    pub window_sections: usize,
    /// `[colors]`/`[colors.primary]`/`[colors.normal]`/`[colors.bright]`/`[colors.dim]`/`[colors.cursor]`/`[colors.vi_mode_cursor]`/`[colors.search]`/`[colors.hints]`/`[colors.line_indicator]`/`[colors.footer_bar]`/`[colors.selection]`/`[colors.transparent_background_colors]`.
    pub colors_sections: usize,
    /// `[key_bindings]`/`[[keyboard]]`/`[[key_bindings]]` bindings entries.
    pub key_bindings: usize,
    /// `mouse_bindings`/`[[mouse]]` bindings entries.
    pub mouse_bindings: usize,
    /// `env`/`shell`/`working_directory`/`live_config_reload`/`ipc_socket`/`import` keys.
    pub env_keys: usize,
    /// Total `key = value`/`key: value` option lines.
    pub options: usize,
    /// `[hints]`/`[selection]`/`[cursor]`/`[mouse]`/`[bell]`/`[scrolling]`/`[debug]`/`[general]`/`[terminal]`/`[keyboard]` other sections.
    pub other_sections: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const FONT_SECTS: &[&str] = &[
    "font",
    "font.normal",
    "font.bold",
    "font.italic",
    "font.bold_italic",
    "font.offset",
    "font.glyph_offset",
];
const WINDOW_SECTS: &[&str] = &[
    "window",
    "window.padding",
    "window.dimensions",
    "window.class",
];
const COLOR_SECTS: &[&str] = &[
    "colors",
    "colors.primary",
    "colors.normal",
    "colors.bright",
    "colors.dim",
    "colors.cursor",
    "colors.vi_mode_cursor",
    "colors.search",
    "colors.hints",
    "colors.line_indicator",
    "colors.footer_bar",
    "colors.selection",
    "colors.transparent_background_colors",
    "colors.indexed_colors",
];
const ENV_KEYS: &[&str] = &[
    "env",
    "shell",
    "working_directory",
    "live_config_reload",
    "ipc_socket",
    "import",
    "general.import",
];

/// Returns `true` when the bytes look like an alacritty config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = [
        "[font",
        "[window",
        "[colors",
        "key_bindings",
        "mouse_bindings",
    ]
    .iter()
    .filter(|k| t.contains(**k))
    .count();
    hits >= 1 && (t.contains('=') || t.contains(':'))
}

impl Alacritty {
    /// Parses an alacritty config, returning a summary or `None`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let mut c = Self {
            sections: 0,
            font_sections: 0,
            window_sections: 0,
            colors_sections: 0,
            key_bindings: 0,
            mouse_bindings: 0,
            env_keys: 0,
            options: 0,
            other_sections: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if tr.starts_with("[[") {
                c.sections += 1;
                if tr.starts_with("[[key_bindings]]") || tr.starts_with("[[keyboard]]") {
                    c.key_bindings += 1;
                } else if tr.starts_with("[[mouse]]") || tr.starts_with("[[mouse_bindings]]") {
                    c.mouse_bindings += 1;
                }
                continue;
            }
            if tr.starts_with('[') {
                c.sections += 1;
                let name = tr.trim_matches(['[', ']']);
                if FONT_SECTS.contains(&name) {
                    c.font_sections += 1;
                } else if WINDOW_SECTS.contains(&name) {
                    c.window_sections += 1;
                } else if COLOR_SECTS.contains(&name) {
                    c.colors_sections += 1;
                } else if name == "key_bindings" || name == "keyboard" {
                    c.key_bindings += 1;
                } else if name == "mouse" || name == "mouse_bindings" {
                    c.mouse_bindings += 1;
                } else {
                    c.other_sections += 1;
                }
                continue;
            }
            let is_kv = tr.contains('=') || tr.contains(':');
            if is_kv {
                c.options += 1;
                let key = tr
                    .split(['=', ':'])
                    .next()
                    .unwrap_or("")
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'');
                if ENV_KEYS.contains(&key) || key.starts_with("env") || key == "shell" {
                    c.env_keys += 1;
                }
                if key == "key" || key == "mods" || key == "action" || key == "mode" {
                    c.key_bindings += 1;
                }
                if key == "mouse" || key == "chars" && key != "mode" {
                    c.mouse_bindings += 1;
                }
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONF: &[u8] = b"# alacritty\n[font]\nnormal = { family = \"Fira\" }\nsize = 12\n[font.offset]\nx = 0\n[window]\npadding.x = 2\ndecorations = \"full\"\n[colors.primary]\nbackground = \"#000000\"\nforeground = \"#ffffff\"\n[key_bindings]\n- { key = \"V\", mods = \"Control\", action = \"Paste\" }\n[env]\nTERM = \"alacritty\"\n";

    #[test]
    fn parses_alacritty() {
        let c = Alacritty::parse(CONF).unwrap();
        assert_eq!(c.sections, 6);
        assert_eq!(c.font_sections, 2);
        assert_eq!(c.window_sections, 1);
        assert_eq!(c.colors_sections, 1);
        assert_eq!(c.key_bindings, 1);
        assert!(c.options > 0);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_non_alacritty() {
        assert!(!detect(b"[package]\nname = \"x\""));
        assert!(Alacritty::parse(b"x").is_none());
    }
}
