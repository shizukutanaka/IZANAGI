//! Waybar config / style (`.jsonc`/`config` + `style.css`) census.
//!
//! Waybar config is JSONC: top object with `layer`/`position`/`height`/
//! `width`/`margin`/`spacing`/`mode`/`name`/`output`/`include`/
//! `modules-left`/`modules-center`/`modules-right` arrays and one
//! `"{mod}"/"{mod}#name"` object per module (`format`/`format-*`/
//! `interval`/`tooltip`/`exec`/`on-click`/`on-scroll-*`/`return-type`/
//! `rotate`/`escape`/`max-length`/`min-length`/`align`/`justify`/
//! `states`/`icon`/`icons`/`rewrite`/`expand`/`scroll-step`/`smooth-*`/
//! `reverse-scrolling`/`transition-*`/`bat`/`adapter`/`interval`/
//! `on-update`/`signal`/`exec-if`/`hide*`/`show*`).
//! `style.css` is GTK CSS: `#waybar`, `.modules-*`, `#module` selectors
//! with `@import`/`@define-color`/font/border/padding rules.
//!
//! ```rust
//! let w = concat!(
//!     "{\n",
//!     "    \"layer\": \"top\",\n",
//!     "    \"position\": \"top\",\n",
//!     "    \"modules-left\": [\"sway/workspaces\"],\n",
//!     "    \"sway/workspaces\": {\n",
//!     "        \"format\": \"{name}\"\n",
//!     "    }\n",
//!     "}\n",
//! );
//! let c = izanagi_kit::waybar::Waybar::parse(w.as_bytes()).unwrap();
//! assert_eq!(c.modules, 3);
//! ```

/// Waybar config/style census.
#[derive(Debug, Clone)]
pub struct Waybar {
    /// `layer`/`position`/`height`/`width`/`margin`/`spacing`/`mode`/`name`/`output`/`include`/`ipc`/`start_hidden`/`passthrough`/`exclusive`/`gtk-layer-shell`/`reload_style_on_change` top-level keys.
    pub top: usize,
    /// `modules-left`/`modules-center`/`modules-right` array entries + `"{mod}"/"{mod}#name"` module object headers + `format*`/`interval`/`tooltip`/`exec`/`on-*`/`return-type`/`rotate`/`states`/`icons`/`icon`/`rewrite`/`expand`/`scroll-step`/`smooth-*`/`reverse-scrolling`/`transition-*`/`bat`/`adapter`/`on-update`/`signal`/`exec-if`/`hide*`/`show*`/`min-length`/`max-length`/`align`/`justify`/`escape`/`on-scroll-*`/`min-length`/`click-through`/`fixed-center`/`separate-outputs`/`layer`/`position`/`exclude`/`all-outputs`/`reset-on-connect`/`bar`/`hide*`/`app_id`/`taskbar*`/`group*`/`persistent*`/`on-click-right`/`minimize`/`launch_prefix`/`tooltip-format`/`format-icons`/`active-*`/`special-*`/`window*`/`submap`/`scrolling`/`keyboard-state`/`idle_inhibitor`/`media`/`mpris`/`player`/`format-player`/`dynamic-icons`/`widget-*`/`menu`/`menu-*`/`menu-file`/`menu-actions`/`orientation`/`side-*`/`expand-padding`/`disable-scroll`/`all-outputs`/`sort-by-*`/`sort-*`/`tooltip-disabled`/`expand-enabled`/`password`/`proxy-*`/`retry-*`/`interval`/`timeout`/`on-click`/`on-click-right`/`on-click-middle`/`on-scroll-up`/`on-scroll-down`/`update-*`/`markup`/`wrap`/`icon`/`caps-lock`/`num-lock`/`ipv*`/`interface`/`max-volume`/`scroll-step`/`smooth-scrolling-threshold`/`device`/`systemd*`/`families`/`appmenu*`/`shown`/`hidden`/`order`/`layout`/`dots`/`host`/`urls`/`seq`/`total`/`buttons`/`hidden-classes`/`factory`/`slide`/`reverse`/`completion`/`chamfer`/`fog`/`show-special`/`workspaces`/`current-only`/`empty`/`local-only`/`remote-only`/`icons`.
    pub modules: usize,
    /// `#waybar`/`.modules-*`/`#module`/`#tags`/`#window`/`#workspaces`/`#taskbar`/`#mpd`/`#tray`/`#clock`/`#cpu`/`#memory`/`#network`/`#battery`/`#pulseaudio`/`#backlight`/`#custom-*`/`#idle_inhibitor`/`#keyboard-state`/`#language`/`#privacy`/`#scratchpad`/`#submap`/`#systemd-failed-units`/`#upower`/`#wlr`/`#sway`/`#hyprland`/`#river`/`#dwl`/`#niri`/`#recorder`/`#wireplumber`/`#power-profiles-daemon`/`#disk`/`#temperature`/`#load`/`#bluetooth`/`#gamemode`/`#group`/`#expander`/`#menu`/`#menu-*`/`#sndio`/`#jack`/`#vfs`/`#osk`/`#image`/`#hyprshell`/`#cffi`/`#user`/`#label`/`#mode`/`#window` selectors + `@import`/`@define-color` directives in CSS.
    pub selectors: usize,
    /// CSS property lines (`:` inside `{...}`): `font-`/`padding`/`margin`/`border`/`background`/`color`/`opacity`/`border-radius`/`min-*`/`box-shadow`/`text-shadow`/`transition`/`animation`/`spacing`/`icon-size`/`min-width`/`min-height`/`padding-*`/`margin-*`/`border-*`/`width`/`height`.
    pub props: usize,
}

/// Whether the buffer looks like a Waybar config or stylesheet.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("modules-left")
        || t.contains("modules-right")
        || t.contains("sway/workspaces")
        || t.contains("hyprland/workspaces")
        || t.contains("#waybar")
        || t.contains("\"layer\"") && t.contains("\"position\"")
        || (t.contains("style") && t.contains("@define-color"))
}

impl Waybar {
    /// Parse a Waybar config/style file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            top: 0,
            modules: 0,
            selectors: 0,
            props: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty()
                || s.starts_with("//")
                || s.starts_with("/*")
                || s == "{"
                || s == "}"
                || s == "["
                || s == "]"
            {
                continue;
            }
            if s.starts_with('@') || s.starts_with('#') || s.starts_with('.') {
                c.selectors += 1;
                continue;
            }
            let key = s.trim_matches('"').split('"').next().unwrap_or("");
            if [
                "layer",
                "position",
                "height",
                "width",
                "margin",
                "spacing",
                "mode",
                "name",
                "output",
                "include",
                "ipc",
                "start_hidden",
                "passthrough",
                "exclusive",
                "gtk-layer-shell",
                "reload_style_on_change",
                "fixed-center",
                "separate-outputs",
            ]
            .contains(&key)
            {
                c.top += 1;
                continue;
            }
            if s.starts_with('"') && s.contains('"') {
                c.modules += 1;
                continue;
            }
            if s.contains(':') && !s.starts_with('"') {
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
    fn parses_config() {
        let b = concat!(
            "{\n",
            "    \"layer\": \"top\",\n",
            "    \"position\": \"top\",\n",
            "    \"modules-left\": [\"sway/workspaces\", \"custom/arrow\"],\n",
            "    \"modules-right\": [\"tray\", \"clock\"],\n",
            "    \"sway/workspaces\": {\n",
            "        \"format\": \"{name}\",\n",
            "        \"disable-scroll\": true\n",
            "    },\n",
            "    \"clock\": {\n",
            "        \"format\": \"{:%H:%M}\",\n",
            "        \"tooltip\": false\n",
            "    }\n",
            "}\n",
        );
        let c = Waybar::parse(b.as_bytes()).unwrap();
        assert_eq!(c.top, 2);
        assert!(c.modules >= 6);
    }

    #[test]
    fn parses_style() {
        let b = concat!(
            "@define-color fg #ffffff\n",
            "#waybar {\n",
            "    background: #000000;\n",
            "    color: @fg;\n",
            "}\n",
            "#workspaces button {\n",
            "    padding: 0 5px;\n",
            "}\n",
        );
        let c = Waybar::parse(b.as_bytes()).unwrap();
        assert!(c.selectors >= 2);
        assert!(c.props >= 3);
    }

    #[test]
    fn rejects_other() {
        assert!(Waybar::parse(b"foo = 1").is_none());
    }
}
