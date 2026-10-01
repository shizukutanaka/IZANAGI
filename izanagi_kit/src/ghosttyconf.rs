//! Ghostty `config` パーサ。
//!
//! 平坦な `key = value` 形式で `font-size`/`theme`/`window-padding-x`/`keybind` 等
//! 既知キーを計数する。
//!
//! ```
//! use izanagi_kit::ghosttyconf;
//! let conf = b"font-size = 13\ntheme = catppuccin-mocha\nwindow-padding-x = 4\nkeybind = ctrl+shift+t=new_tab\n";
//! assert!(ghosttyconf::detect(conf));
//! let c = ghosttyconf::parse(conf).unwrap();
//! assert_eq!(c.entries, 4);
//! assert_eq!(c.known_keys, 4);
//! ```

/// 判定結果の計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `key = value` 行数。
    pub entries: usize,
    /// 既知キー行数。
    pub known_keys: usize,
    /// `keybind = ...` 行数。
    pub keybinds: usize,
}

const KNOWN_KEYS: &[&str] = &[
    "font-family",
    "font-family-bold",
    "font-family-italic",
    "font-size",
    "font-feature",
    "font-variation",
    "font-codepoint-map",
    "font-style",
    "font-synthetic-style",
    "font-thicken",
    "theme",
    "background",
    "foreground",
    "cursor-color",
    "cursor-text",
    "selection-background",
    "selection-foreground",
    "palette",
    "window-padding-x",
    "window-padding-y",
    "window-padding-balance",
    "window-padding-color",
    "window-decoration",
    "window-theme",
    "window-title-font-family",
    "window-width",
    "window-height",
    "window-position-x",
    "window-position-y",
    "window-save-state",
    "window-step-resize",
    "window-new-tab-position",
    "fullscreen",
    "maximize",
    "keybind",
    "mouse-hide-while-typing",
    "copy-on-select",
    "click-repeat-interval",
    "cursor-style",
    "cursor-style-blink",
    "cursor-opacity",
    "shell-integration",
    "shell-integration-features",
    "term",
    "confirm-close-surface",
    "quit-after-last-window-closed",
    "scrollback-limit",
    "scrollback",
    "scrollbar",
    "scroll-to-bottom",
    "link",
    "link-url",
    "link-color",
    "split-divider-color",
    "unfocused-split-opacity",
    "unfocused-split-fill",
    "working-directory",
    "command",
    "initial-command",
    "wait-after-command",
    "env",
    "title",
    "title-report",
    "class",
    "x11-instance-name",
    "gtk-single-instance",
    "gtk-tabs-location",
    "gtk-wide-tabs",
    "adw-toolbar-style",
    "linux-cgroup",
    "linux-cgroup-memory-limit",
    "linux-cgroup-processes-limit",
    "linux-cgroup-hard-factory-reset",
    "auto-update",
    "auto-update-channel",
    "clipboard-read",
    "clipboard-write",
    "clipboard-trim-trailing-spaces",
    "image-storage-limit",
    "minimum-contrast",
    "focus-follows-mouse",
    "click-to-focus",
    "quit-after-last-window-closed",
    "vte-fast-path",
    "custom-shader",
    "custom-shader-animation",
    "macos-titlebar-style",
    "macos-titlebar-proxy-icon",
    "macos-option-as-alt",
    "macos-non-native-fullscreen",
    "macos-window-shadow",
    "macos-auto-secure-input",
    "macos-icon",
    "macos-icon-frame",
    "macos-icon-ghost-color",
    "macos-icon-screen-color",
    "macos-shortcut",
    "app-notifications",
    "initial-input",
    "background-opacity",
    "background-blur-radius",
    "window-colorspace",
    "window-vsync",
    "resize-overlay",
    "resize-overlay-position",
    "resize-overlay-duration",
    "config-file",
];

/// `ghostty config` らしさを簡易判定する。
pub fn detect(b: &[u8]) -> bool {
    let c = parse(b);
    match c {
        Some(c) => c.entries >= 3 && c.known_keys >= 3,
        None => false,
    }
}

/// 行種別を計数する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    if !b.is_ascii() {
        return None;
    }
    let s = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        entries: 0,
        known_keys: 0,
        keybinds: 0,
    };
    for l in s.lines() {
        let t = l.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let Some(eq) = t.find('=') else {
            continue;
        };
        let key = t[..eq].trim().to_ascii_lowercase();
        if key.is_empty() || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
            continue;
        }
        c.entries += 1;
        if KNOWN_KEYS.contains(&key.as_str()) {
            c.known_keys += 1;
        }
        if key == "keybind" {
            c.keybinds += 1;
        }
    }
    (c.entries > 0).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"font-family = \"JetBrains Mono\"\nfont-size = 13\ntheme = catppuccin-mocha\nwindow-padding-x = 4\nwindow-padding-y = 4\ncursor-style = block\nkeybind = ctrl+shift+t=new_tab\nkeybind = ctrl+shift+q=quit\nscrollback-limit = 100000\n";

    #[test]
    fn detects_ghostty() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.entries, 9);
        assert_eq!(c.known_keys, 9);
        assert_eq!(c.keybinds, 2);
    }

    #[test]
    fn rejects_other() {
        let t = b"foo = bar\nbaz = 1\nqux = yes\n";
        assert!(!detect(t));
    }
}
