//! iTerm2 Dynamic Profiles JSON (`*.iterm2profile` / `~/Library/Application Support/iTerm2/DynamicProfiles/*.json`) パーサ。
//!
//! `"Profiles"` 配列と iTerm 固有の PascalCase キー (`"Guid"`/`"Name"`/
//! `"Dynamic Profile"`/`"Background Color"`/`"Foreground Color"`/`"Ansi * Color"`/
//! `"Custom Command"` 等) を計数する。
//!
//! ```
//! use izanagi_kit::itermdyn;
//! let s = b"{\"Profiles\": [{\"Name\": \"Dev\", \"Guid\": \"abc-def\", \"Dynamic Profile\": true, \"Custom Command\": \"ssh dev\"}]}\n";
//! assert!(itermdyn::detect(s));
//! let c = itermdyn::parse(s).unwrap();
//! assert_eq!(c.profiles_key, 1);
//! ```

/// 判定結果の計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `"Profiles"` キー出現数。
    pub profiles_key: usize,
    /// `"key":` エントリ数。
    pub entries: usize,
    /// 既知 PascalCase キー数。
    pub known_keys: usize,
    /// `"Ansi * Color"` キー数。
    pub ansi_colors: usize,
}

const KNOWN_KEYS: &[&str] = &[
    "Name",
    "Guid",
    "Dynamic Profile",
    "Badge Text",
    "Background Color",
    "Foreground Color",
    "Bold Color",
    "Link Color",
    "Cursor Color",
    "Cursor Text Color",
    "Selection Color",
    "Selected Text Color",
    "Custom Command",
    "Command",
    "Working Directory",
    "Initial Text",
    "Custom Directory",
    "Shortcut",
    "Icon Path",
    "Tags",
    "Bound Hosts",
    "Transparency",
    "Blur",
    "Blur Radius",
    "Use Bold Font",
    "Use Italic Font",
    "Use Bright Bold",
    "Use Underline Color",
    "Use Tab Color",
    "Tab Color",
    "Title",
    "Sync Title",
    "Rows",
    "Columns",
    "Scrollback Lines",
    "Unlimited Scrollback",
    "Terminal Type",
    "Answerback",
    "Encoding",
    "Non-ASCII Anti Aliased",
    "ASCII Anti Aliased",
    "Normal Font",
    "Non Ascii Font",
    "Horizontal Spacing",
    "Vertical Spacing",
    "Keyboard Map",
    "Option Key Sends",
    "Right Option Key Sends",
    "Left Option Key Sends",
    "Send Code When Idle",
    "Idle Code",
    "Idle Period",
    "Silence Bell",
    "Visual Bell",
    "Flashing Bell",
    "Bell",
    "BM Growl",
    "Mouse Reporting",
    "Cursor Type",
    "Blinking Cursor",
    "Smart Cursor Color",
    "Smart Selection Rules",
    "Triggers",
    "Semantic History",
    "Prompt Before Closing",
    "Sync Prompt After Closing",
    "Jobs to Ignore",
    "Allow Title Reporting",
    "Allow Title Setting",
    "Disable Smcup Rmcup",
    "Has Hotkey",
    "Has HotKey",
    "HotKey Activates",
    "HotKey Shortcut",
    "HotKey Characters",
    "HotKey Characters Ignoring Modifiers",
    "HotKey Key Code",
    "HotKey Modifier Activation",
    "HotKey Modifiers",
    "Session HotKey",
    "Use Custom Window Title",
    "Custom Window Title Escapes",
    "Minimum Contrast",
    "Custom Color Presets",
    "Custom Window Title",
];

const ANSI_PREFIX: &str = "Ansi ";

/// Dynamic Profiles JSON らしさを簡易判定する。
pub fn detect(b: &[u8]) -> bool {
    let c = parse(b);
    match c {
        Some(c) => c.profiles_key >= 1 && c.known_keys >= 2,
        None => false,
    }
}

/// `"key":` エントリを計数する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    if !b.is_ascii() {
        return None;
    }
    let s = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        profiles_key: 0,
        entries: 0,
        known_keys: 0,
        ansi_colors: 0,
    };
    for l in s.lines() {
        let mut rest = l;
        while let Some(open) = rest.find('"') {
            let Some(rel) = rest[open + 1..].find('"') else {
                break;
            };
            let k = &rest[open + 1..open + 1 + rel];
            let after = &rest[open + rel + 2..];
            rest = after;
            if !after.trim_start().starts_with(':') {
                continue;
            }
            if k.is_empty() {
                continue;
            }
            c.entries += 1;
            if k == "Profiles" {
                c.profiles_key += 1;
            }
            if KNOWN_KEYS.contains(&k) {
                c.known_keys += 1;
            }
            if k.starts_with(ANSI_PREFIX) && k.ends_with(" Color") {
                c.ansi_colors += 1;
            }
        }
    }
    (c.entries > 0).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"{\n  \"Profiles\": [\n    {\n      \"Name\": \"Dev\",\n      \"Guid\": \"abc-def-123\",\n      \"Dynamic Profile\": true,\n      \"Custom Command\": \"ssh dev\",\n      \"Foreground Color\": { \"Red Component\": 0.8, \"Green Component\": 0.8, \"Blue Component\": 0.8 },\n      \"Ansi 0 Color\": { \"Red Component\": 0 }\n    },\n    {\n      \"Name\": \"Prod\",\n      \"Guid\": \"zzz-999\"\n    }\n  ]\n}\n";

    #[test]
    fn detects_iterm_dynamic() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.profiles_key, 1);
        assert_eq!(c.known_keys, 7);
        assert_eq!(c.ansi_colors, 1);
    }

    #[test]
    fn rejects_generic_json() {
        let j = b"{\"name\": \"x\", \"guid\": \"y\", \"items\": []}\n";
        assert!(!detect(j));
    }
}
