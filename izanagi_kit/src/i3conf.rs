//! i3 window manager config census.
//!
//! An i3 config is line-based: `set $name value` variables, `bindsym`/
//! `bindcode`/`bindsym --release`/`bindsym --whole-window` bindings,
//! `exec`/`exec_always` autostart, `font`, `mode`/`for_window`/`assign`/
//! `floating_*`/`workspace_layout`/`focus_*`/`mouse_warping`/`default_*`/
//! `new_window`/`new_float`, `bar { ... }` blocks with `status_command`/
//! `position`/`tray_output`/`mode`/`colors {`/`binding_mode_indicator`/
//! `workspace_buttons`/`strip_workspace_numbers`/`separator_symbol`,
//! `include`/`set_from_resource`, `workspace <name> output <o>`,
//! `hide_edge_borders`/`smart_*`/`popup_during_fullscreen`/
//! `show_marks`/`tiling_*`/`dragging_*`.
//!
//! ```rust
//! let i = concat!(
//!     "set $mod Mod4\n",
//!     "font pango:monospace 8\n",
//!     "bindsym $mod+Return exec i3-sensible-terminal\n",
//!     "exec_always --no-startup-id nm-applet\n",
//!     "bar {\n",
//!     "    status_command i3status\n",
//!     "}\n",
//! );
//! let c = izanagi_kit::i3conf::I3conf::parse(i.as_bytes()).unwrap();
//! assert_eq!(c.vars, 1);
//! assert_eq!(c.binds, 1);
//! ```

const SUBCMDS: &[&str] = &[
    "font",
    "mode",
    "for_window",
    "assign",
    "floating_modifier",
    "floating_minimum_size",
    "floating_maximum_size",
    "workspace_layout",
    "focus_follows_mouse",
    "mouse_warping",
    "focus_wrapping",
    "focus_on_window_activation",
    "default_border",
    "default_floating_border",
    "default_orientation",
    "new_window",
    "new_float",
    "hide_edge_borders",
    "smart_borders",
    "smart_gaps",
    "popup_during_fullscreen",
    "show_marks",
    "tiling_drag",
    "tiling_resize",
    "gaps",
    "workspace",
    "include",
    "set_from_resource",
    "client.focused",
    "client.focused_inactive",
    "client.unfocused",
    "client.urgent",
    "client.placeholder",
    "client.background",
    "workspace_auto_back_and_forth",
    "force_focus_wrapping",
    "focus_parent",
    "focus_child",
    "no_focus",
    "force_display_urgency_hint",
];

/// i3 config census.
#[derive(Debug, Clone)]
pub struct I3conf {
    /// `set`/`set_from_resource` variable definitions.
    pub vars: usize,
    /// `bindsym`/`bindcode` binding lines.
    pub binds: usize,
    /// `exec`/`exec_always` autostart lines.
    pub execs: usize,
    /// `bar`/`colors`/`mode` `{`-block openers.
    pub blocks: usize,
    /// `status_command`/`position`/`tray_output`/`binding_mode_indicator`/`workspace_buttons`/`strip_workspace_numbers`/`separator_symbol`/`font`/`icon_theme`/`verbose`/`i3bar_command`/`id`/`socket_path` bar/sub-block directives + other directives (font/mode/for_window/assign/floating_*/workspace/gaps/include/client.*).
    pub directives: usize,
}

/// Whether the buffer looks like an i3 config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("bindsym") || t.contains("bindcode")) && t.contains("set $")
        || t.contains("bar {")
        || t.contains("exec_always")
}

impl I3conf {
    /// Parse an i3 config into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            vars: 0,
            binds: 0,
            execs: 0,
            blocks: 0,
            directives: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() || s.starts_with('#') || s == "}" {
                continue;
            }
            let head = s.split_whitespace().next().unwrap_or("");
            if head == "set" || head == "set_from_resource" {
                c.vars += 1;
                continue;
            }
            if head == "bindsym" || head == "bindcode" {
                c.binds += 1;
                continue;
            }
            if head == "exec" || head == "exec_always" {
                c.execs += 1;
                continue;
            }
            if head == "bar" || head == "colors" || head == "mode" && s.ends_with('{') {
                c.blocks += 1;
                continue;
            }
            if SUBCMDS.contains(&head)
                || head.starts_with("client.")
                || [
                    "status_command",
                    "position",
                    "tray_output",
                    "binding_mode_indicator",
                    "workspace_buttons",
                    "strip_workspace_numbers",
                    "separator_symbol",
                    "icon_theme",
                    "verbose",
                    "i3bar_command",
                    "id",
                    "socket_path",
                    "modifier",
                ]
                .contains(&head)
            {
                c.directives += 1;
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
    fn parses_config() {
        let b = concat!(
            "# i3 config\n",
            "set $mod Mod4\n",
            "set $term i3-sensible-terminal\n",
            "font pango:monospace 8\n",
            "floating_modifier $mod\n",
            "bindsym $mod+Return exec $term\n",
            "bindcode 116 exec dmenu_run\n",
            "bindsym --release $mod+c exec firefox\n",
            "exec_always --no-startup-id nm-applet\n",
            "exec --no-startup-id feh --bg-fill ~/wall.png\n",
            "for_window [class=\"Firefox\"] floating enable\n",
            "workspace \"1\" output HDMI-1\n",
            "mode \"resize\" {\n",
            "    bindsym h resize shrink width 10 px\n",
            "}\n",
            "bar {\n",
            "    status_command i3status\n",
            "    position top\n",
            "    colors {\n",
            "        statusline #ffffff\n",
            "    }\n",
            "}\n",
            "include ~/.config/i3/local.conf\n",
        );
        let c = I3conf::parse(b.as_bytes()).unwrap();
        assert_eq!(c.vars, 2);
        assert_eq!(c.binds, 4);
        assert_eq!(c.execs, 2);
        assert_eq!(c.blocks, 3);
        assert!(c.directives >= 7);
    }

    #[test]
    fn rejects_other() {
        assert!(I3conf::parse(b"foo = 1").is_none());
    }
}
