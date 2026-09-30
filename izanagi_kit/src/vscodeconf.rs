//! VS Code `settings.json` (JSONC) census.
//!
//! Top-level `"key":` census over `editor.*` / `workbench.*` /
//! `files.*` / `terminal.*` / `git.*` / `extensions.*` /
//! `window.*` / `search.*` / `telemetry.*` / `security.*` /
//! `debug.*` / `scm.*` / `notebook.*` / `remote.*` /
//! `problems.*` / `update.*` / `http.*` / `emmet.*` /
//! `markdown.*` / `diffEditor.*` / `accessibility.*` /
//! `zenMode.*` namespaces; `"[language]": { }` override
//! sections; `keybindings.json`-style arrays not covered.
//!
//! ```rust
//! let v = "{\n  \"editor.tabSize\": 4,\n  \"files.autoSave\": \"onFocusChange\",\n  \"[python]\": {\n    \"editor.formatOnSave\": true\n  }\n}\n";
//! let c = izanagi_kit::vscodeconf::Vscodeconf::parse(v.as_bytes()).unwrap();
//! assert_eq!(c.lang_sections, 1);
//! ```

/// VS Code settings census.
#[derive(Debug, Clone)]
pub struct Vscodeconf {
    /// `"key":` entries.
    pub settings: usize,
    /// `"[lang]":` override sections.
    pub lang_sections: usize,
    /// `editor.`/`workbench.`/`files.`/`terminal.`/`git.`/`window.`/`search.`/`debug.`/`scm.`/`notebook.`/`remote.`/`security.`/`telemetry.`/`problems.`/`update.`/`http.`/`emmet.`/`markdown.`/`diffEditor.`/`accessibility.`/`zenMode.`/`task.`/`explorer.`/`scm.` prefixed keys.
    pub editor_keys: usize,
    /// `//` or `/*` comment lines.
    pub comments: usize,
}

const PREFIXES: &[&str] = &[
    "editor.",
    "workbench.",
    "files.",
    "terminal.",
    "git.",
    "extensions.",
    "window.",
    "search.",
    "telemetry.",
    "security.",
    "debug.",
    "scm.",
    "notebook.",
    "remote.",
    "problems.",
    "update.",
    "http.",
    "emmet.",
    "markdown.",
    "diffEditor.",
    "accessibility.",
    "zenMode.",
    "task.",
    "explorer.",
    "launch.",
    "javascript.",
    "typescript.",
    "python.",
    "go.",
    "rust.",
    "rust-analyzer.",
    "java.",
    "clangd.",
    "cmake.",
    "docker.",
    "kubernetes.",
    "yaml.",
    "json.",
    "html.",
    "css.",
    "less.",
    "scss.",
    "php.",
    "sql.",
    "vim.",
    "errorLens.",
    "cSpell.",
    "hediet.",
    "eslint.",
    "prettier.",
    "github.",
    "githubPullRequests.",
    "liveServer.",
    "todohighlight.",
    "peacock.",
    "material-icon-theme.",
    "remoteHub.",
    "codespaces.",
    "github.copilot.",
];

/// Detect settings.json content.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    if !t.contains('{') {
        return false;
    }
    let mut hits = 0usize;
    for p in [
        "\"editor.",
        "\"workbench.",
        "\"files.",
        "\"terminal.",
        "\"[",
        "\"window.",
    ] {
        if t.contains(p) {
            hits += 1;
        }
    }
    hits >= 2
}

impl Vscodeconf {
    /// Census a settings.json buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            settings: 0,
            lang_sections: 0,
            editor_keys: 0,
            comments: 0,
        };
        for seg in t.split(',') {
            let mut inner = seg.trim().trim_start_matches('{').trim();
            while inner.starts_with("//") || inner.starts_with("/*") {
                match inner.find('\n') {
                    Some(i) => {
                        inner = inner[i + 1..].trim().trim_start_matches('{').trim();
                    }
                    None => {
                        inner = "";
                        break;
                    }
                }
            }
            if inner.is_empty() || inner == "{" || inner.starts_with('}') {
                continue;
            }
            if let Some(rest) = inner.strip_prefix('"') {
                if let Some(end) = rest.find('"') {
                    if rest.len() > end + 1 && rest[end + 1..].trim_start().starts_with(':') {
                        let key = &rest[..end];
                        if key.is_empty() {
                            continue;
                        }
                        c.settings += 1;
                        if key.starts_with('[') && key.ends_with(']') {
                            c.lang_sections += 1;
                        }
                        if PREFIXES.iter().any(|p| key.starts_with(p)) {
                            c.editor_keys += 1;
                        }
                    }
                }
            }
        }
        for line in t.lines() {
            let s = line.trim();
            if s.starts_with("//") || s.starts_with("/*") {
                c.comments += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_conf() {
        let b = br#"{ "editor.tabSize": 4, "files.autoSave": "onFocusChange" }"#;
        assert!(detect(b));
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[x]\ny=1\n"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses_conf() {
        let b = concat!(
            "{\n",
            "  // editor\n",
            "  \"editor.tabSize\": 4,\n",
            "  \"editor.formatOnSave\": true,\n",
            "  \"editor.fontSize\": 13,\n",
            "  \"editor.minimap.enabled\": false,\n",
            "  \"workbench.colorTheme\": \"Default Dark+\",\n",
            "  \"workbench.startupEditor\": \"none\",\n",
            "  \"files.autoSave\": \"onFocusChange\",\n",
            "  \"files.exclude\": {\n",
            "    \"**/node_modules\": true\n",
            "  },\n",
            "  \"terminal.integrated.fontSize\": 12,\n",
            "  \"git.autofetch\": true,\n",
            "  \"window.zoomLevel\": 1,\n",
            "  \"search.exclude\": {},\n",
            "  \"telemetry.telemetryLevel\": \"off\",\n",
            "  \"[python]\": {\n",
            "    \"editor.formatOnSave\": true\n",
            "  },\n",
            "  \"[rust]\": {\n",
            "    \"editor.tabSize\": 4\n",
            "  },\n",
            "  \"rust-analyzer.checkOnSave.command\": \"clippy\"\n",
            "}\n",
        );
        let c = Vscodeconf::parse(b.as_bytes()).unwrap();
        assert!(c.settings >= 16);
        assert_eq!(c.lang_sections, 2);
        assert!(c.editor_keys >= 12);
        assert_eq!(c.comments, 1);
    }
}
