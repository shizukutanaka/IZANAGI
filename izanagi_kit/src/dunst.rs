//! Dunst `dunstrc` (INI) census.
//!
//! A dunstrc is INI: `[global]` settings (`monitor`/`follow`/`width`/
//! `height`/`origin`/`offset`/`scale`/`notification_limit`/`indicate_hidden`/
//! `transparency`/`separator_height`/`padding`/`horizontal_padding`/
//! `text_icon_padding`/`frame_width`/`frame_color`/`gap_size`/`separator_color`/
//! `sort`/`font`/`line_height`/`markup`/`format`/`alignment`/`vertical_alignment`/
//! `show_age_threshold`/`ellipsize`/`ignore_newline`/`stack_duplicates`/
//! `hide_duplicate_count`/`show_indicators`/`enable_recursive_icon_lookup`/
//! `icon_theme`/`icon_size`/`min_icon_size`/`max_icon_size`/`icon_path`/
//! `sticky_history`/`history_length`/`dmenu`/`browser`/`always_run_script`/
//! `title`/`class`/`corner_radius`/`ignore_dbusclose`/`force_xwayland`/
//! `force_xinerama`/`mouse_left_click`/`mouse_middle_click`/`mouse_right_click`/
//! `progress_bar*`/`notification_height`/`layer`/`output`/`wayland`/`x11`/
//! `close`/`close_all`/`history`/`history_push`/`context`/`print`/`critical`/
//! `idle_threshold`/`fullscreen`/`word_wrap`/`icon_position`/`min_width`/
//! `max_width`/`progress_bar_frame_width`/`progress_bar_min_width`/
//! `progress_bar_max_width`/`corner_radius`), `[urgency_low]`/`[urgency_normal]`/
//! `[urgency_critical]` sections (`background`/`foreground`/`highlight`/
//! `frame_color`/`timeout`/`default_icon`/`icon`/`override_urgency`/`format`),
//! and per-rule sections like `[play_sound]`/`[ignore_*]` (`appname`/`summary`/
//! `body`/`icon`/`urgency`/`category`/`msg_urgency`/`transient`/`timeout`/
//! `set_*`/`new_icon`/`foreground`/`background`/`format`/`script`/
//! `skip_display`/`history_ignore`/`always_run`/`desktop_entry`/`stack_tag`/
//! `set_transient`/`set_category`/`match_*`/`fullscreen`/`enabled`/
//! `notification_limit`) filters plus `[shortcuts]` (`close`/`close_all`/
//! `history`/`context`) — all `key = value` with `#`/`;` comments.
//!
//! ```rust
//! let d = concat!(
//!     "[global]\n",
//!     "    monitor = 0\n",
//!     "    font = Monospace 10\n",
//!     "    format = \"<b>%s</b>\\n%b\"\n",
//!     "[urgency_critical]\n",
//!     "    background = \"#ff0000\"\n",
//! );
//! let c = izanagi_kit::dunst::Dunst::parse(d.as_bytes()).unwrap();
//! assert_eq!(c.sections, 2);
//! ```

/// Dunst config census.
#[derive(Debug, Clone)]
pub struct Dunst {
    /// `[global]`/`[urgency_low]`/`[urgency_normal]`/`[urgency_critical]`/`[shortcuts]`/`[experimental]`/rule `[*]` section headers.
    pub sections: usize,
    /// `key = value` settings inside `[global]`/`[experimental]`/`[shortcuts]`.
    pub globals: usize,
    /// `key = value` settings inside `[urgency_*]` sections.
    pub urgency: usize,
    /// `key = value` settings inside per-rule sections (everything after a `[*]` that isn't global/urgency/shortcuts/experimental).
    pub rules: usize,
}

/// Whether the buffer looks like a dunstrc.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("[urgency_low]")
        || t.contains("[urgency_normal]")
        || t.contains("[urgency_critical]")
        || (t.contains("[global]")
            && (t.contains("notification_limit")
                || t.contains("separator_color")
                || t.contains("frame_color")
                || t.contains("show_age_threshold")
                || t.contains("stack_duplicates")))
}

impl Dunst {
    /// Parse a dunstrc into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            globals: 0,
            urgency: 0,
            rules: 0,
        };
        let mut scope = 0u8; // 0 global-ish, 1 urgency, 2 rule
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() || s.starts_with('#') || s.starts_with(';') {
                continue;
            }
            if s.starts_with('[') && s.ends_with(']') {
                c.sections += 1;
                let name = &s[1..s.len() - 1];
                scope = if name.starts_with("urgency_") {
                    1
                } else if ["global", "shortcuts", "experimental", "filter"].contains(&name) {
                    0
                } else {
                    2
                };
                continue;
            }
            if !s.contains('=') {
                continue;
            }
            match scope {
                1 => c.urgency += 1,
                2 => c.rules += 1,
                _ => c.globals += 1,
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_dunstrc() {
        let b = concat!(
            "[global]\n",
            "    monitor = 0\n",
            "    follow = mouse\n",
            "    width = 300\n",
            "    origin = top-right\n",
            "    font = Monospace 10\n",
            "    format = \"<b>%s</b>\\n%b\"\n",
            "    icon_theme = Papirus\n",
            "    mouse_left_click = close_current\n",
            "[experimental]\n",
            "    per_monitor_dpi = false\n",
            "[urgency_low]\n",
            "    background = \"#222222\"\n",
            "    foreground = \"#888888\"\n",
            "    timeout = 10\n",
            "[urgency_normal]\n",
            "    background = \"#285577\"\n",
            "    timeout = 10\n",
            "[urgency_critical]\n",
            "    background = \"#900000\"\n",
            "    foreground = \"#ffffff\"\n",
            "    frame_color = \"#ff0000\"\n",
            "    timeout = 0\n",
            "[ignore_discord]\n",
            "    appname = Discord\n",
            "    skip_display = true\n",
            "    history_ignore = yes\n",
        );
        let c = Dunst::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 6);
        assert_eq!(c.globals, 9);
        assert_eq!(c.urgency, 9);
        assert_eq!(c.rules, 3);
    }

    #[test]
    fn rejects_other() {
        assert!(Dunst::parse(b"foo = 1").is_none());
    }
}
