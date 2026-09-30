//! Sway (Wayland i3-compatible) config census.
//!
//! A sway config shares i3's grammar: `set $name value`, `bindsym`/
//! `bindcode`/`bindgesture`/`bindswitch`/`bindsym --release`/etc,
//! `exec`/`exec_always`, `include`, `font`/`gaps`/`default_*`/
//! `floating_modifier`/`focus_*`/`mouse_warping`/`for_window`/`assign`/
//! `workspace_layout`, `mode`/`bar`/`colors`/`input`/`output`/`seat`/
//! `idle`/`corner_radius`/`default_dim_inactive`/`smart_borders`/
//! `smart_gaps` blocks and per-input/output options
//! (`xkb_layout`/`tap`/`natural_scroll`/`resolution`/`position`/`scale`/
//! `transform`/`background`/`disable`/`adaptive_sync`/`subpixel`/
//! `dpms`/`hdr`/`render_bit_depth`/`power`/`hotplug`/`shadow`/`blur`/
//! `titlebar_*`/`workspace <name> output`/`client.*` color blocks).
//!
//! ```rust
//! let s = concat!(
//!     "set $mod Mod4\n",
//!     "bindsym $mod+Return exec foot\n",
//!     "input type:keyboard {\n",
//!     "    xkb_layout us\n",
//!     "}\n",
//! );
//! let c = izanagi_kit::sway::Sway::parse(s.as_bytes()).unwrap();
//! assert_eq!(c.binds, 1);
//! ```

/// Sway config census.
#[derive(Debug, Clone)]
pub struct Sway {
    /// `set`/`set_from_resource` variable definitions.
    pub vars: usize,
    /// `bindsym`/`bindcode`/`bindgesture`/`bindswitch` binding lines.
    pub binds: usize,
    /// `exec`/`exec_always` autostart lines.
    pub execs: usize,
    /// `mode`/`bar`/`colors`/`input`/`output`/`seat`/`idle`/`workspace`/`gaps`/`default_dim_inactive` block openers (line ends with `{`).
    pub blocks: usize,
    /// Everything else recognized: `font`/`floating_*`/`focus_*`/`mouse_warping`/`for_window`/`assign`/`workspace_layout`/`titlebar_*`/`smart_*`/`show_marks`/`new_window`/`new_float`/`hide_edge_borders`/`default_border`/`default_floating_border`/`default_orientation`/`popup_during_fullscreen`/`workspace_auto_back_and_forth`/`client.*`/`include`/`corner_radius`/`shadow`/`blur`/`xkb_*`/`tap`/`natural_scroll`/`dwt`/`pointer_accel`/`accel_profile`/`repeat_rate`/`repeat_delay`/`scroll_method`/`events`/`click_method`/`middle_emulation`/`left_handed`/`map_to_*`/`resolution`/`position`/`scale`/`transform`/`background`/`disable`/`adaptive_sync`/`subpixel`/`dpms`/`hdr`/`render_bit_depth`/`power`/`hotplug`/`status_command`/`position`/`tray_*`/`binding_mode_indicator`/`workspace_buttons`/`strip_workspace_*`/`separator_symbol`/`pango_markup`/`swaybar_command`/`swaynag_command`/`icon_theme`/`unbindswitch`/`unbindsym`/`unbindcode`/`unbindgesture`/`unmark`/`mark`/`opacity`/`inhibit_idle`/`urgent`/`shortcuts_inhibitor`.
    pub directives: usize,
}

/// Whether the buffer looks like a sway config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("bindsym")
        || t.contains("bindcode")
        || t.contains("bindgesture")
        || t.contains("bindswitch"))
        && (t.contains("set $")
            || t.contains("swaybar_command")
            || t.contains("xkb_")
            || t.contains("natural_scroll"))
        || t.contains("input type:")
        || t.contains("swaynag")
}

const KEYS: &[&str] = &[
    "font",
    "floating_modifier",
    "floating_minimum_size",
    "floating_maximum_size",
    "focus_follows_mouse",
    "mouse_warping",
    "focus_wrapping",
    "focus_on_window_activation",
    "for_window",
    "assign",
    "workspace_layout",
    "titlebar_border_thickness",
    "titlebar_padding",
    "smart_borders",
    "smart_gaps",
    "show_marks",
    "new_window",
    "new_float",
    "hide_edge_borders",
    "default_border",
    "default_floating_border",
    "default_orientation",
    "default_dim_inactive",
    "popup_during_fullscreen",
    "workspace_auto_back_and_forth",
    "corner_radius",
    "shadow",
    "shadow_on_focused",
    "blur",
    "xkb_layout",
    "xkb_variant",
    "xkb_options",
    "xkb_model",
    "xkb_rules",
    "xkb_numlock",
    "xkb_capslock",
    "tap",
    "natural_scroll",
    "dwt",
    "pointer_accel",
    "accel_profile",
    "repeat_rate",
    "repeat_delay",
    "scroll_method",
    "scroll_factor",
    "events",
    "click_method",
    "middle_emulation",
    "left_handed",
    "map_to_output",
    "map_to_region",
    "resolution",
    "position",
    "scale",
    "scale_filter",
    "transform",
    "background",
    "disable",
    "adaptive_sync",
    "subpixel",
    "dpms",
    "hdr",
    "render_bit_depth",
    "power",
    "hotplug",
    "status_command",
    "position",
    "tray_output",
    "tray_padding",
    "tray_bindcode",
    "tray_bindsym",
    "tray_button_press",
    "binding_mode_indicator",
    "workspace_buttons",
    "strip_workspace_numbers",
    "strip_workspace_name",
    "separator_symbol",
    "pango_markup",
    "swaybar_command",
    "swaynag_command",
    "icon_theme",
    "unbindswitch",
    "unbindsym",
    "unbindcode",
    "unbindgesture",
    "unmark",
    "mark",
    "opacity",
    "inhibit_idle",
    "urgent",
    "shortcuts_inhibitor",
    "no_focus",
    "focus_parent",
    "focus_child",
    "force_focus_wrapping",
    "force_display_urgency_hint",
    "primary_selection",
    "seat",
    "gesture",
    "xwayland",
    "idle_inhibit",
    "create_output",
];

impl Sway {
    /// Parse a sway config into census counts.
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
            if head == "bindsym"
                || head == "bindcode"
                || head == "bindgesture"
                || head == "bindswitch"
            {
                c.binds += 1;
                continue;
            }
            if head == "exec" || head == "exec_always" {
                c.execs += 1;
                continue;
            }
            if s.ends_with('{')
                && [
                    "mode",
                    "bar",
                    "colors",
                    "input",
                    "output",
                    "seat",
                    "idle",
                    "workspace",
                    "gaps",
                    "default_dim_inactive",
                ]
                .contains(&head)
            {
                c.blocks += 1;
                continue;
            }
            if KEYS.contains(&head)
                || head.starts_with("client.")
                || head == "include"
                || head == "workspace"
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
            "# sway config\n",
            "set $mod Mod4\n",
            "set $term foot\n",
            "font pango:monospace 8\n",
            "bindsym $mod+Return exec $term\n",
            "bindcode 116 exec wofi\n",
            "bindgesture swipe:right workspace next\n",
            "bindswitch lid:on output eDP-1 disable\n",
            "exec_always --no-startup-id waybar\n",
            "input type:keyboard {\n",
            "    xkb_layout us\n",
            "    repeat_rate 25\n",
            "    repeat_delay 600\n",
            "}\n",
            "output DP-1 {\n",
            "    resolution 1920x1080\n",
            "    position 0,0\n",
            "    background ~/wall.jpg fill\n",
            "    adaptive_sync on\n",
            "}\n",
            "bar {\n",
            "    swaybar_command waybar\n",
            "    position top\n",
            "}\n",
            "for_window [app_id=\"firefox\"] inhibit_idle fullscreen\n",
            "include ~/.config/sway/local.conf\n",
        );
        let c = Sway::parse(b.as_bytes()).unwrap();
        assert_eq!(c.vars, 2);
        assert_eq!(c.binds, 4);
        assert_eq!(c.execs, 1);
        assert_eq!(c.blocks, 3);
        assert!(c.directives >= 9);
    }

    #[test]
    fn rejects_other() {
        assert!(Sway::parse(b"foo = 1").is_none());
    }
}
