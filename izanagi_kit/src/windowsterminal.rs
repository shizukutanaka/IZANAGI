//! Windows Terminal `settings.json` パーサ。
//!
//! `profiles`(list/defaults)/`schemes`/`actions`/`keybindings`/`themes`/`globals` 等の
//! 既知トップキーと `"guid"`/`"name"`/`"commandline"`/`"colorScheme"` 等キーを計数する。
//!
//! ```
//! use izanagi_kit::windowsterminal;
//! let s = b"{\"$schema\": \"https://aka.ms/terminal-profiles-schema\", \"profiles\": {\"list\": [{\"guid\": \"{a}\", \"name\": \"pwsh\", \"commandline\": \"pwsh.exe\"}]}}\n";
//! assert!(windowsterminal::detect(s));
//! let c = windowsterminal::parse(s).unwrap();
//! assert_eq!(c.known_keys, 5);
//! ```

/// 判定結果の計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `"key":` エントリ数。
    pub entries: usize,
    /// 既知キー数。
    pub known_keys: usize,
    /// `"guid"` 値を持つエントリ数。
    pub guids: usize,
    /// `"profiles"`/`"schemes"`/`"actions"`/`"keybindings"`/`"themes"` 出現数。
    pub wt_sections: usize,
}

const KNOWN_KEYS: &[&str] = &[
    "guid",
    "name",
    "commandline",
    "colorScheme",
    "fontFace",
    "fontSize",
    "icon",
    "hidden",
    "startingDirectory",
    "tabTitle",
    "historySize",
    "snapOnInput",
    "cursorShape",
    "cursorColor",
    "cursorHeight",
    "antialiasingMode",
    "useAcrylic",
    "acrylicOpacity",
    "opacity",
    "padding",
    "scrollbarState",
    "closeOnExit",
    "source",
    "suppressApplicationTitle",
    "altGrAliasing",
    "tabColor",
    "admin",
    "elevated",
    "bellStyle",
    "experimental.retroTerminalEffect",
    "intenseTextStyle",
    "adjustIndistinguishableColors",
    "pathTranslation",
    "backgroundImage",
    "backgroundImageOpacity",
    "backgroundImageStretchMode",
    "backgroundImageAlignment",
    "foreground",
    "background",
    "selectionBackground",
    "cursorColor",
    "black",
    "red",
    "green",
    "yellow",
    "blue",
    "purple",
    "cyan",
    "white",
    "brightBlack",
    "brightRed",
    "brightGreen",
    "brightYellow",
    "brightBlue",
    "brightPurple",
    "brightCyan",
    "brightWhite",
    "copyFormatting",
    "copyOnSelect",
    "trimBlockSelection",
    "trimPaste",
    "largePasteWarning",
    "multiLinePasteWarning",
    "wordDelimiters",
    "confirmCloseAllTabs",
    "showTabsInTitlebar",
    "alwaysShowTabs",
    "tabWidthMode",
    "initialCols",
    "initialRows",
    "initialPosition",
    "launchMode",
    "startupActions",
    "centerOnLaunch",
    "defaultProfile",
    "useAcrylicInTabRow",
    "showTerminalTitleInTitlebar",
    "disabledProfileSources",
    "theme",
    "themes",
    "actions",
    "keybindings",
    "schemes",
    "profiles",
    "globals",
    "list",
    "defaults",
    "sendInput",
    "command",
    "keys",
    "id",
    "commandPalette",
    "window",
    "application",
    "tab",
    "nameMinimizationStyle",
];

/// `settings.json` (Windows Terminal) らしさを簡易判定する。
pub fn detect(b: &[u8]) -> bool {
    let c = parse(b);
    match c {
        Some(c) => {
            (c.wt_sections >= 1 && c.guids >= 1)
                || (c.wt_sections >= 2 && c.known_keys >= 4)
                || (c.wt_sections >= 1 && c.known_keys >= 6)
        }
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
        entries: 0,
        known_keys: 0,
        guids: 0,
        wt_sections: 0,
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
            if k.is_empty()
                || !k
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
            {
                continue;
            }
            c.entries += 1;
            if KNOWN_KEYS.contains(&k) {
                c.known_keys += 1;
            }
            if k == "guid" {
                c.guids += 1;
            }
            if matches!(
                k,
                "profiles" | "schemes" | "actions" | "keybindings" | "themes" | "globals"
            ) {
                c.wt_sections += 1;
            }
        }
    }
    (c.entries > 0).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"{\n  \"$schema\": \"https://aka.ms/terminal-profiles-schema\",\n  \"defaultProfile\": \"{61c54bbd-c2c6-5271-96e7-009a87ff44bf}\",\n  \"profiles\": {\n    \"defaults\": { \"fontSize\": 11 },\n    \"list\": [\n      { \"guid\": \"{61c54bbd-c2c6-5271-96e7-009a87ff44bf}\", \"name\": \"Windows PowerShell\", \"commandline\": \"powershell.exe\" },\n      { \"guid\": \"{0caa0dad-35be-5f56-a8ff-afceeeaa6101}\", \"name\": \"cmd\", \"commandline\": \"cmd.exe\" }\n    ]\n  },\n  \"schemes\": [{ \"name\": \"Campbell\", \"background\": \"#0C0C0C\" }],\n  \"actions\": [{ \"command\": \"copy\", \"keys\": \"ctrl+shift+c\" }]\n}\n";

    #[test]
    fn detects_settings() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.wt_sections, 3);
        assert_eq!(c.guids, 2);
        assert_eq!(c.known_keys, 17);
    }

    #[test]
    fn rejects_generic_json() {
        let j = b"{\"a\": 1, \"b\": 2, \"c\": {\"d\": 3}}\n";
        assert!(!detect(j));
    }
}
