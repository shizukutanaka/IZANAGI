//! Parser for Sublime Text project/settings/keymap files
//! (`.sublime-project`, `.sublime-settings`, `.sublime-keymap`).
//!
//! Counts `folders`/`path`/`file_exclude_patterns`/`folder_exclude_patterns`,
//! `settings`/`build_systems`/`variants`, keymap `keys`/`command`/`args`,
//! and common settings keys.
//!
//! ```
//! let b = b"{\n  \"folders\": [\n    { \"path\": \".\" }\n  ],\n  \"settings\": {\n    \"tab_size\": 2\n  }\n}\n";
//! assert!(izanagi_kit::sublime::detect(b));
//! let c = izanagi_kit::sublime::Sublime::parse(b).unwrap();
//! assert_eq!(c.folders, 1);
//! assert_eq!(c.settings_keys, 1);
//! ```

/// Parsed Sublime config summary.
#[derive(Debug, Clone)]
pub struct Sublime {
    /// `path`/`name` folder entries in `folders`.
    pub folders: usize,
    /// `file_exclude_patterns`/`folder_exclude_patterns`/`file_include_patterns`/`folder_include_patterns`/`binary_file_patterns`/`index_exclude_patterns`.
    pub exclude_patterns: usize,
    /// `settings` block + settings keys (tab_size/word_wrap/font_size/color_scheme/theme/ignored_packages…).
    pub settings_keys: usize,
    /// `build_systems`/`variants` entries.
    pub build_systems: usize,
    /// Keymap `keys`/`command`/`args`/`context` lines.
    pub keymap_keys: usize,
    /// `menu`/`caption`/`id`/`mnemonic` menu entries.
    pub menu_entries: usize,
    /// `extensions`/`auto_complete*`/`trim_trailing_white_space*`/`ensure_newline_at_eof*`/`translate_tabs_to_spaces` editor keys.
    pub editor_keys: usize,
    /// `//`/`/*` comment lines.
    pub comments: usize,
}

const EDITOR_KEYS: &[&str] = &[
    "tab_size",
    "word_wrap",
    "font_size",
    "color_scheme",
    "theme",
    "ignored_packages",
    "translate_tabs_to_spaces",
    "trim_trailing_white_space_on_save",
    "ensure_newline_at_eof_on_save",
    "auto_complete",
    "detect_indentation",
    "draw_white_space",
    "rulers",
    "spell_check",
];

fn key(l: &str) -> Option<&str> {
    let tr = l.trim();
    let q = tr
        .trim_start_matches(['{', '[', ',', ' '])
        .strip_prefix('"')?;
    let end = q.find('"')?;
    if q[end + 1..].trim_start().starts_with(':') {
        Some(&q[..end])
    } else {
        None
    }
}

/// Returns `true` when the bytes look like a Sublime config file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let keys = [
        "\"folders\"",
        "\"file_exclude_patterns\"",
        "\"folder_exclude_patterns\"",
        "\"build_systems\"",
        "\"ignored_packages\"",
        "\"keys\"",
        "\"caption\"",
    ];
    let hits = keys.iter().filter(|k| t.contains(**k)).count();
    hits >= 1
}

impl Sublime {
    /// Parses a Sublime config file, returning a summary or `None`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let mut c = Self {
            folders: 0,
            exclude_patterns: 0,
            settings_keys: 0,
            build_systems: 0,
            keymap_keys: 0,
            menu_entries: 0,
            editor_keys: 0,
            comments: 0,
        };
        let mut depth = 0usize;
        let mut in_folders = false;
        let mut in_settings = false;
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with("//") || tr.starts_with("/*") {
                c.comments += 1;
                continue;
            }
            if let Some(k) = key(l) {
                match k {
                    "folders" => {
                        in_folders = true;
                    }
                    "settings" => {
                        in_settings = true;
                    }
                    "build_systems" | "variants" => c.build_systems += 1,
                    "path" | "name" if in_folders => c.folders += 1,
                    "file_exclude_patterns"
                    | "folder_exclude_patterns"
                    | "file_include_patterns"
                    | "folder_include_patterns"
                    | "binary_file_patterns"
                    | "index_exclude_patterns" => c.exclude_patterns += 1,
                    "keys" | "command" | "args" | "context" => c.keymap_keys += 1,
                    "caption" | "mnemonic" | "id" | "menu" => c.menu_entries += 1,
                    _ => {
                        if in_settings || EDITOR_KEYS.contains(&k) {
                            c.settings_keys += 1;
                        }
                        if EDITOR_KEYS.contains(&k) {
                            c.editor_keys += 1;
                        }
                    }
                }
            }
            depth += tr.matches('{').count() + tr.matches('[').count();
            depth = depth.saturating_sub(tr.matches('}').count() + tr.matches(']').count());
            if depth == 0 {
                in_folders = false;
                in_settings = false;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONF: &[u8] = b"{\n  \"folders\": [\n    {\n      \"path\": \".\",\n      \"folder_exclude_patterns\": [\"target\"]\n    }\n  ],\n  \"settings\": {\n    \"tab_size\": 2,\n    \"translate_tabs_to_spaces\": true\n  },\n  \"build_systems\": []\n}\n";

    #[test]
    fn parses_sublime() {
        let c = Sublime::parse(CONF).unwrap();
        assert_eq!(c.folders, 1);
        assert_eq!(c.exclude_patterns, 1);
        assert_eq!(c.settings_keys, 2);
        assert_eq!(c.build_systems, 1);
        assert_eq!(c.editor_keys, 2);
    }

    #[test]
    fn rejects_non_sublime() {
        assert!(!detect(b"{\"a\": 1}"));
        assert!(Sublime::parse(b"x").is_none());
    }
}
