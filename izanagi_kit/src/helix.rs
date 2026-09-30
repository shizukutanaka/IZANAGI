//! Helix `config.toml` / `languages.toml` census.
//!
//! `config.toml`: `[editor]` + `[editor.cursor-shape]` /
//! `[editor.file-picker]` / `[editor.statusline]` /
//! `[editor.lsp]` / `[editor.indent-guides]` /
//! `[editor.soft-wrap]` / `[editor.auto-pairs]` /
//! `[editor.search]` / `[editor.whitespace]` /
//! `[editor.gutter]` / `[editor.inline-diagnostics]` /
//! `[editor.end-of-line-diagnostics]` /
//! `[editor.popup-border]` / `[editor.smart-tab]` +
//! `[keys.normal]` / `[keys.insert]` / `[keys.select]` +
//! `[keys.normal.g]`/`[keys.normal.m]`/`[keys.normal.z]`/
//! `[keys.normal.C-w]` + `theme = "…"` top keys.
//!
//! `languages.toml`: `[[language]]` entries (`name`,
//! `scope`, `file-types`, `roots`, `indent`,
//! `language-servers`, `grammar`, `injection-regex`,
//! `comment-token`, `block-comment-tokens`,
//! `auto-format`, `formatter`, `diagnostic-severity`,
//! `persistent-diagnostic-sources`, `text-width`,
//! `workspace-lsp-roots`, `ruler`, `soft-wrap`) and
//! `[[grammar]]`/`[[language-server]]` tables.
//!
//! ```rust
//! let h = "theme = \"catppuccin_mocha\"\n[editor]\nline-number = \"relative\"\n[keys.normal]\nspace.f = \"file_picker\"\n";
//! let c = izanagi_kit::helix::Helix::parse(h.as_bytes()).unwrap();
//! assert_eq!(c.sections, 2);
//! ```

/// Helix config census.
#[derive(Debug, Clone)]
pub struct Helix {
    /// `[…]`/`[[…]]` tables.
    pub sections: usize,
    /// `[keys.*]` tables.
    pub key_tables: usize,
    /// `[[language]]`/`[[grammar]]`/`[[language-server]]` entries.
    pub languages: usize,
    /// `key = value` pairs.
    pub settings: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Detect helix config content.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    (t.contains("[editor]") && t.contains("theme"))
        || t.contains("[keys.normal]")
        || (t.contains("[[language]]") && t.contains("file-types"))
        || t.contains("[editor.cursor-shape]")
}

impl Helix {
    /// Census a helix config buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            key_tables: 0,
            languages: 0,
            settings: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if s.starts_with("[[") {
                c.sections += 1;
                if s.starts_with("[[language]]")
                    || s.starts_with("[[grammar]]")
                    || s.starts_with("[[language-server]]")
                {
                    c.languages += 1;
                }
                continue;
            }
            if s.starts_with('[') {
                c.sections += 1;
                if s.starts_with("[keys.") || s == "[keys]" {
                    c.key_tables += 1;
                }
                continue;
            }
            if let Some(eq) = s.find('=') {
                let key = s[..eq].trim();
                if !key.is_empty() {
                    c.settings += 1;
                }
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_helix() {
        let b = b"theme = \"x\"\n[editor]\nline-number = \"relative\"\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[x]\ny=1\n"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses_helix() {
        let b = concat!(
            "# helix\n",
            "theme = \"catppuccin_mocha\"\n",
            "[editor]\n",
            "line-number = \"relative\"\n",
            "cursorline = true\n",
            "auto-save = true\n",
            "idle-timeout = 400\n",
            "completion-trigger-len = 1\n",
            "[editor.cursor-shape]\n",
            "normal = \"block\"\n",
            "insert = \"bar\"\n",
            "select = \"underline\"\n",
            "[editor.statusline]\n",
            "left = [\"mode\", \"spinner\", \"file-name\"]\n",
            "[editor.lsp]\n",
            "enable = true\n",
            "display-messages = true\n",
            "auto-signature-help = false\n",
            "[editor.indent-guides]\n",
            "render = true\n",
            "character = \"|\"\n",
            "[editor.file-picker]\n",
            "hidden = false\n",
            "[keys.normal]\n",
            "space.f = \"file_picker\"\n",
            "space.b = \"buffer_picker\"\n",
            "C-s = \":w\"\n",
            "[keys.insert]\n",
            "j.k = \"normal_mode\"\n",
            "[keys.select]\n",
            "space = \"keep_selections\"\n",
        );
        let c = Helix::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 9);
        assert_eq!(c.key_tables, 3);
        assert!(c.settings >= 20);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn parses_languages_toml() {
        let b = concat!(
            "[[language]]\n",
            "name = \"rust\"\n",
            "auto-format = true\n",
            "[[language]]\n",
            "name = \"python\"\n",
            "file-types = [\"py\"]\n",
            "scope = \"source.python\"\n",
        );
        let c = Helix::parse(b.as_bytes()).unwrap();
        assert_eq!(c.languages, 2);
        assert!(c.settings >= 5);
    }
}
