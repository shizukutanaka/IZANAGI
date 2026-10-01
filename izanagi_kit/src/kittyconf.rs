//! Parser for kitty terminal configuration files (`kitty.conf`).
//!
//! Counts `key value` option lines (font_family/font_size/cursor_shape/
//! scrollback_lines/window_padding_width/…), `map` key bindings,
//! `symbol_map`/`font_features`, `include`/`env`/`kitten` lines,
//! `action_alias`, and comments.
//!
//! ```
//! let b = b"font_family FiraCode\nfont_size 12\nmap ctrl+shift+c copy_to_clipboard\n";
//! assert!(izanagi_kit::kittyconf::detect(b));
//! let c = izanagi_kit::kittyconf::Kittyconf::parse(b).unwrap();
//! assert_eq!(c.options, 2);
//! assert_eq!(c.maps, 1);
//! ```

/// Parsed kitty.conf summary.
#[derive(Debug, Clone)]
pub struct Kittyconf {
    /// `key value` option lines (non-map, non-map-adjacent keys).
    pub options: usize,
    /// `map <shortcut> <action>` bindings.
    pub maps: usize,
    /// `symbol_map`/`font_features` lines.
    pub font_maps: usize,
    /// `include`/`env`/`kitten_alias`/`action_alias` lines.
    pub includes: usize,
    /// `kitten`/`launch`/`shell`/`startup_session`/`watcher`/`exe_search_path` lines.
    pub startup: usize,
    /// Color entries (`color0`-`color255`/`foreground`/`background`/`selection_*`/`url_color`/`cursor`/`active_border_color`/`inactive_border_color`).
    pub colors: usize,
    /// `enable_audio_bell`/`visual_bell_*`/`window_alert_*`/`confirm_os_window_close`/`remember_window_*` toggles.
    pub toggles: usize,
    /// `pointer_shape_when_*`/`mouse_map`/`click_interval` lines.
    pub mouse: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const COLOR_KEYS: &[&str] = &[
    "foreground",
    "background",
    "selection_foreground",
    "selection_background",
    "url_color",
    "cursor",
    "cursor_text_color",
    "active_border_color",
    "inactive_border_color",
    "bell_border_color",
    "active_tab_foreground",
    "active_tab_background",
    "inactive_tab_foreground",
    "inactive_tab_background",
    "mark1_foreground",
    "mark1_background",
    "mark2_foreground",
    "mark2_background",
    "mark3_foreground",
    "mark3_background",
];

const TOGGLE_KEYS: &[&str] = &[
    "enable_audio_bell",
    "visual_bell_duration",
    "visual_bell_color",
    "window_alert_on_bell",
    "bell_on_tab",
    "confirm_os_window_close",
    "remember_window_size",
    "remember_window_position",
    "adjust_line_height",
    "adjust_column_width",
    "disable_ligatures",
    "allow_remote_control",
    "listen_on",
    "repaint_delay",
    "input_delay",
    "sync_to_monitor",
];

const MOUSE_KEYS: &[&str] = &[
    "mouse_map",
    "click_interval",
    "pointer_shape_when_grabbed",
    "pointer_shape_when_dragging",
    "url_style",
    "open_url_with",
    "copy_on_select",
];

const STARTUP_KEYS: &[&str] = &[
    "kitten",
    "launch",
    "shell",
    "startup_session",
    "watcher",
    "exe_search_path",
    "close_on_child_death",
    "term",
    "shell_integration",
    "editor",
];

const INC_KEYS: &[&str] = &[
    "include",
    "env",
    "kitten_alias",
    "action_alias",
    "clear_all_shortcuts",
    "clear_all_mouse_actions",
    "clipboard_control",
];

/// Returns `true` when the bytes look like a kitty.conf.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let known = t
        .lines()
        .filter(|l| {
            let tr = l.trim();
            let mut it = tr.split_whitespace();
            let k = it.next().unwrap_or("");
            it.next().is_some()
                && (COLOR_KEYS.contains(&k)
                    || TOGGLE_KEYS.contains(&k)
                    || MOUSE_KEYS.contains(&k)
                    || STARTUP_KEYS.contains(&k)
                    || INC_KEYS.contains(&k)
                    || k == "map"
                    || k == "symbol_map"
                    || k == "font_features"
                    || k.starts_with("color")
                    || k.starts_with("font_"))
        })
        .count();
    known >= 1
}

impl Kittyconf {
    /// Parses a kitty.conf, returning a summary or `None`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let mut c = Self {
            options: 0,
            maps: 0,
            font_maps: 0,
            includes: 0,
            startup: 0,
            colors: 0,
            toggles: 0,
            mouse: 0,
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
            let mut it = tr.split_whitespace();
            let k = it.next().unwrap_or("");
            if it.next().is_none() {
                continue;
            }
            match k {
                "map" => c.maps += 1,
                "symbol_map" | "font_features" => c.font_maps += 1,
                _ => {
                    if INC_KEYS.contains(&k) {
                        c.includes += 1;
                    } else if STARTUP_KEYS.contains(&k) {
                        c.startup += 1;
                    } else if COLOR_KEYS.contains(&k)
                        || (k.starts_with("color") && k[5..].chars().all(|ch| ch.is_ascii_digit()))
                    {
                        c.colors += 1;
                    } else if TOGGLE_KEYS.contains(&k) {
                        c.toggles += 1;
                    } else if MOUSE_KEYS.contains(&k) {
                        c.mouse += 1;
                    } else {
                        c.options += 1;
                    }
                }
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONF: &[u8] = b"# kitty\nfont_family FiraCode\nfont_size 12\nscrollback_lines 10000\nforeground #dddddd\nbackground #111111\ncolor0 #000000\ncolor255 #eeeeee\nenable_audio_bell no\nmap ctrl+shift+c copy_to_clipboard\nmap ctrl+shift+v paste_from_clipboard\nsymbol_map U+23FB-U+23FE Fira Code\ninclude other.conf\n";

    #[test]
    fn parses_kittyconf() {
        let c = Kittyconf::parse(CONF).unwrap();
        assert_eq!(c.options, 3);
        assert_eq!(c.maps, 2);
        assert_eq!(c.font_maps, 1);
        assert_eq!(c.includes, 1);
        assert_eq!(c.colors, 4);
        assert_eq!(c.toggles, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_non_kittyconf() {
        assert!(!detect(b"[section]\nkey=value"));
        assert!(Kittyconf::parse(b"x").is_none());
    }
}
