//! `kdeglobals` (KDE Plasma グローバル設定) 検出モジュール。
//!
//! kdeglobals は INI 形式で、`[General]`/`[KDE]`/`[Icons]`/
//! `[WM]`/`[ColorEffects:Disabled]`/`[ColorEffects:Inactive]`/
//! `[Colors:Button]`/`[Colors:Complementary]`/`[Colors:Header]`/
//! `[Colors:Header][Inactive]`/`[Colors:Link]`/`[Colors:Selection]`/
//! `[Colors:Tooltip]`/`[Colors:View]`/`[Colors:Window]`/
//! `[KFileDialog Settings]`/`[Shortcuts]`/`[Translations]` 等の
//! セクションと `ColorScheme`/`widgetStyle`/`font`/`fixed`/
//! `menuFont`/`toolBarFont`/`smallestReadableFont`/`activeFont`/
//! `desktopFont`/`taskbarFont`/`singleClick`/`ShowDeleteCommand`/
//! `XftHintStyle`/`XftSubPixel`/`XftAntialias`/`contrast`/
//! `shadeSortColumn`/`TerminalApplication`/`BrowserApplication`/
//! `Theme`/`immutable` 等のキーで構成される。
//!
//! ```
//! let b = b"[General]\n\
//!           ColorScheme=BreezeDark\n\
//!           widgetStyle=kvantum\n\
//!           [KDE]\n\
//!           ShowDeleteCommand=false\n\
//!           singleClick=true\n\
//!           [Colors:Window]\n\
//!           BackgroundNormal=35,38,41\n";
//! let c = izanagi_kit::kdeglobals::parse(b);
//! assert!(izanagi_kit::kdeglobals::detect(b));
//! assert_eq!(c.sections, 3);
//! ```

const KEYS: &[&str] = &[
    "activeFont",
    "accent",
    "AnimationDurationFactor",
    "ApplicationStylePreference",
    "BrowserApplication",
    "ColorScheme",
    "contrast",
    "DecorationFocus",
    "DecorationHover",
    "desktopFont",
    "fileDialogGadget",
    "fixed",
    "font",
    "FontChangeTrigger",
    "iconsAndMimes",
    "immutable",
    "IntensityEffect",
    "menuFont",
    "Name",
    "NetCursorShape",
    "shadeSortColumn",
    "shadeSortColumnLow",
    "ShowDeleteCommand",
    "singleClick",
    "smallestReadableFont",
    "TerminalApplication",
    "taskbarFont",
    "Theme",
    "toolBarFont",
    "widgetStyle",
    "XftAntialias",
    "XftHintStyle",
    "XftSubPixel",
];

fn is_section(t: &str) -> bool {
    t == "[General]"
        || t == "[KDE]"
        || t == "[Icons]"
        || t == "[WM]"
        || t == "[Shortcuts]"
        || t == "[Translations]"
        || t == "[KFileDialog Settings]"
        || t == "[Settings]"
        || t.starts_with("[Colors:")
        || t.starts_with("[ColorEffects:")
}

/// `b` が kdeglobals に見えるかを返す。
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
        } else if tr.contains('=') {
            let k = tr.split('=').next().unwrap_or("").trim();
            if KEYS.contains(&k) {
                keys += 1;
            }
        }
    }
    (secs >= 1 && keys >= 2) || keys >= 3 || secs >= 3
}

/// kdeglobals の統計。
#[derive(Debug, Default, Clone)]
pub struct Kdeglobals {
    /// 既知セクション数。
    pub sections: usize,
    /// 既知キー行数。
    pub keys: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を kdeglobals として統計する。
pub fn parse(b: &[u8]) -> Kdeglobals {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = Kdeglobals::default();
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
        } else if tr.contains('=') {
            let k = tr.split('=').next().unwrap_or("").trim();
            if KEYS.contains(&k) {
                c.keys += 1;
            }
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"[General]\nColorScheme=Breeze\nwidgetStyle=breeze\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.sections, 1);
    }

    #[test]
    fn detects_color_sections() {
        let b = b"[Colors:Button]\n[Colors:View]\n[Colors:Window]\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"[General]\n"));
        assert!(!detect(b"[section]\nkey = x\nfoo = y\n"));
        assert!(!detect(b"ColorScheme=x\nwidgetStyle=y\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.sections, 0);
    }
}
