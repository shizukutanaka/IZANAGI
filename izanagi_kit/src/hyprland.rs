//! Hyprland `hyprland.conf` census.
//!
//! A Hyprland config is `key = value` assignments plus section headers
//! like `general {`, `decoration {`, `animations {`, `input {`,
//! `gestures {`, `group {`, `misc {`, `binds {`, `xwayland {`,
//! `cursor {`, `debug {`, `render {`, `opengl {`, `dwindle {`,
//! `master {`, `plugin {`, `device {`/`monitor {...}`/`workspace {...}`,
//! `env = NAME,VALUE`, `exec-once`/`execr`/`exec`/`source`, `monitor`,
//! `workspace`/`windowrule`/`windowrulev2`/`layerrule`, `bind`/`bindm`/
//! `bindr`/`bindl`/`binde`/`bindd`/`bindc`/`bindi`/`bindt`/`bindn`,
//! `unbind`, `blurls`, `bezier`, `animation`, `plugin`, `permission`,
//! `gaps_in`/`gaps_out`/`border_size`/`col.*`/`layout`/`decorate`/
//! `rounding`/`blur {`/`shadow {`/`dim_inactive`/`active_opacity`/
//! `inactive_opacity`/`fullscreen_opacity`/`enabled`/`range`/
//! `render_titles`/`ignore_empty`/`selective`/`new_on_top`/`special`.
//!
//! ```rust
//! let h = concat!(
//!     "monitor = , preferred, auto, 1\n",
//!     "general {\n",
//!     "    gaps_in = 5\n",
//!     "}\n",
//!     "bind = SUPER, Return, exec, foot\n",
//! );
//! let c = izanagi_kit::hyprland::Hyprland::parse(h.as_bytes()).unwrap();
//! assert_eq!(c.binds, 1);
//! ```

const SECTIONS: &[&str] = &[
    "general",
    "decoration",
    "animations",
    "input",
    "gestures",
    "group",
    "misc",
    "binds",
    "xwayland",
    "cursor",
    "debug",
    "render",
    "opengl",
    "dwindle",
    "master",
    "plugin",
    "device",
    "monitor",
    "workspace",
    "touchdevice",
    "tablet",
    "keyboard",
    "autoscale",
    "ecosystem",
    "experimental",
    "layerrule",
    "windowrule",
    "submap",
    "snap",
    "overview",
    "hypervnc",
    "permission",
    "bezier",
    "animation",
    "shadow",
    "blur",
    "dim_inactive",
    "col",
    "touchdevice",
];

/// Hyprland config census.
#[derive(Debug, Clone)]
pub struct Hyprland {
    /// `general`/`decoration`/`animations`/`input`/`gestures`/`group`/`misc`/`binds`/`xwayland`/`cursor`/`debug`/`render`/`opengl`/`dwindle`/`master`/`plugin`/`device`/`touchdevice`/`tablet`/`keyboard`/`autoscale`/`ecosystem`/`experimental`/`snap`/`overview`/`hypervnc`/`shadow`/`blur`/`submap` section openers (line ends `{`).
    pub sections: usize,
    /// `monitor`/`workspace`/`windowrule`/`windowrulev2`/`layerrule`/`permission`/`bezier`/`animation`/`blurls`/`source`/`env`/`exec`/`exec-once`/`execr`/`unbind`/`plugin`/`submap` assignments outside/inside sections.
    pub rules: usize,
    /// `bind`/`bindm`/`bindr`/`bindl`/`binde`/`bindd`/`bindc`/`bindi`/`bindt`/`bindn`/`binds` keybind lines.
    pub binds: usize,
    /// Every other `key = value` line (general/decoration/input/dwindle/master/gestures/misc/binds/xwayland/cursor/debug/opengl option names).
    pub options: usize,
    /// `}` closers + `source`/`env`-style structural lines not covered above.
    pub other: usize,
}

/// Whether the buffer looks like a Hyprland config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("monitor") && t.contains("bind"))
        || t.contains("windowrule")
        || t.contains("exec-once")
        || (t.contains("general {") && t.contains("gaps_"))
        || t.contains("bindm")
        || t.contains("binde")
}

impl Hyprland {
    /// Parse a Hyprland config into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            rules: 0,
            binds: 0,
            options: 0,
            other: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() || s.starts_with('#') {
                continue;
            }
            if s == "}" {
                c.other += 1;
                continue;
            }
            let head = s.split_whitespace().next().unwrap_or("");
            if s.ends_with('{') && SECTIONS.contains(&head) {
                c.sections += 1;
                continue;
            }
            if [
                "monitor",
                "workspace",
                "windowrule",
                "windowrulev2",
                "layerrule",
                "permission",
                "bezier",
                "animation",
                "blurls",
                "source",
                "env",
                "exec",
                "exec-once",
                "execr",
                "unbind",
                "plugin",
                "submap",
            ]
            .contains(&head)
            {
                c.rules += 1;
                continue;
            }
            if head.starts_with("bind") && head != "binds" || head == "binds" && !s.ends_with('{') {
                c.binds += 1;
                continue;
            }
            if s.contains('=') {
                c.options += 1;
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
            "monitor = , preferred, auto, 1\n",
            "exec-once = waybar\n",
            "env = XCURSOR_SIZE,24\n",
            "general {\n",
            "    gaps_in = 5\n",
            "    gaps_out = 10\n",
            "    border_size = 2\n",
            "    col.active_border = rgba(33ccffee)\n",
            "    layout = dwindle\n",
            "}\n",
            "decoration {\n",
            "    rounding = 10\n",
            "    blur {\n",
            "        enabled = true\n",
            "    }\n",
            "}\n",
            "input {\n",
            "    kb_layout = us\n",
            "}\n",
            "windowrulev2 = float, class:^(firefox)$\n",
            "workspace = 1, monitor:DP-1\n",
            "bind = SUPER, Return, exec, foot\n",
            "bindm = SUPER, mouse:272, movewindow\n",
            "binde = SUPER, R, resizeactive\n",
            "source = ~/.config/hypr/local.conf\n",
        );
        let c = Hyprland::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 4);
        assert_eq!(c.binds, 3);
        assert!(c.rules >= 6);
        assert!(c.options >= 8);
    }

    #[test]
    fn rejects_other() {
        assert!(Hyprland::parse(b"foo = 1").is_none());
    }
}
