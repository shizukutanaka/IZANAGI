//! `qt5ct.conf` / `qt6ct.conf` 検出モジュール。
//!
//! qt5ct/qt6ct(Qt Configuration Tool)の設定は INI 形式で、
//! `[Appearance]`/`[Fonts]`/`[Interface]`/`[PaletteEditor]`/
//! `[SettingsWindow]`/`[Troubleshooting]`/`[Debug]` セクションと
//! `color_scheme_path`/`custom_palette`/`icon_theme`/
//! `standard_dialogs`/`style`/`buttonbox_layout`/
//! `check_for_updates`/`menus_have_icons`/`toolbutton_style`/
//! `activate_item_on_single_click`/`stylesheets`/`gtk2_theme`/
//! `keyboard_scheme`/`double_click_activate`/`palette`/
//! `geometry`/`font_substitutions`/`trace` 等のキーで構成される。
//!
//! ```
//! let b = b"[Appearance]\n\
//!           color_scheme_path=\n\
//!           custom_palette=false\n\
//!           icon_theme=Papirus-Dark\n\
//!           style=kvantum-dark\n\
//!           [Interface]\n\
//!           activate_item_on_single_click=1\n";
//! let c = izanagi_kit::qt5ctconf::parse(b);
//! assert!(izanagi_kit::qt5ctconf::detect(b));
//! assert_eq!(c.sections, 2);
//! ```

const SECTIONS: &[&str] = &[
    "[Appearance]",
    "[Fonts]",
    "[Interface]",
    "[PaletteEditor]",
    "[SettingsWindow]",
    "[Troubleshooting]",
    "[Debug]",
    "[General]",
];

const KEYS: &[&str] = &[
    "activate_item_on_single_click",
    "buttonbox_layout",
    "check_for_updates",
    "color_scheme_path",
    "custom_palette",
    "dialog_buttons_have_icons",
    "double_click_activate",
    "font_substitutions",
    "geometry",
    "gtk2_theme",
    "icon_theme",
    "keyboard_scheme",
    "menus_have_icons",
    "palette",
    "running_under_wayland",
    "show_effects",
    "standard_dialogs",
    "style",
    "stylesheets",
    "toolbutton_style",
    "trace",
    "under_terminal",
    "wheel_scroll_lines",
];

fn is_section(t: &str) -> bool {
    SECTIONS.contains(&t)
}

fn key_present(t: &str) -> bool {
    let k = t.split('=').next().unwrap_or("").trim();
    KEYS.contains(&k)
}

/// `b` が qt5ct/qt6ct の設定に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut secs = 0usize;
    let mut keys = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') || tr.starts_with(';') {
            continue;
        }
        if is_section(tr) {
            secs += 1;
        } else if key_present(tr) {
            keys += 1;
        }
    }
    (secs >= 1 && keys >= 1) || keys >= 3
}

/// qt5ct/qt6ct 設定の統計。
#[derive(Debug, Default, Clone)]
pub struct Qt5ctConf {
    /// 既知セクション数。
    pub sections: usize,
    /// 既知キー行数。
    pub keys: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を qt5ct/qt6ct 設定として統計する。
pub fn parse(b: &[u8]) -> Qt5ctConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = Qt5ctConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') || tr.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if is_section(tr) {
            c.sections += 1;
        } else if key_present(tr) {
            c.keys += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"[Appearance]\nstyle=Fusion\nicon_theme=Papirus\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.sections, 1);
    }

    #[test]
    fn detects_keys() {
        let b = b"style=Fusion\nicon_theme=Papirus\nstandard_dialogs=gtk3\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"[Appearance]\n"));
        assert!(!detect(b"[section]\nkey = x\nfoo = y\n"));
        assert!(!detect(b"style=Fusion\nicon_theme=x\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.sections, 0);
    }
}
