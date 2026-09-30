//! Zed `settings.json` census.
//!
//! JSON `"key":` census over Zed keys: `theme`,
//! `buffer_font_family`/`buffer_font_size`/`buffer_font_weight`/
//! `buffer_line_height`, `vim_mode`, `autosave`, `tab_size`,
//! `preferred_line_length`, `hard_tabs`, `languages`,
//! `lsp`, `assistant`, `agent`, `terminal`, `project_panel`,
//! `git`, `inlay_hints`, `features`, `journal`, `ui_font_size`/
//! `ui_font_family`, `base_keymap`, `auto_update`,
//! `confirm_quit`, `cursor_blink`, `indent_guides`,
//! `scrollbar`, `tabs`, `toolbar`, `wrap_guides`,
//! `seed_search_query_from_cursor`, `format_on_save`,
//! `formatter`, `remove_trailing_whitespace_on_save`,
//! `ensure_final_newline_on_save`, `soft_wrap`,
//! `show_whitespaces`, `proxy`, `node`, `ssh_connections`,
//! `close_on_file_delete`, `restore_on_startup`,
//! `preview_tabs`, `search`, `file_scan_exclusions`,
//! `private_files`, `collaboration_panel`, `outline_panel`,
//! `notification_panel`, `chat_panel`, `message_editor`,
//! `drop_target_size`, `when_closing_with_no_tabs`,
//! `use_on_type_format`, `active_pane_magnification`,
//! `centered_layout`, `context_servers`, `slash_commands`,
//! `inline_completions`, `edit_predictions`, `evaluatable_expr`.
//!
//! ```rust
//! let z = "{\n  \"theme\": \"One Dark\",\n  \"buffer_font_size\": 14,\n  \"vim_mode\": true\n}\n";
//! let c = izanagi_kit::zedconf::Zedconf::parse(z.as_bytes()).unwrap();
//! assert_eq!(c.settings, 3);
//! ```

/// Zed settings census.
#[derive(Debug, Clone)]
pub struct Zedconf {
    /// `"key":` entries.
    pub settings: usize,
    /// Recognised Zed keys.
    pub named: usize,
    /// `//` or `/*` comment lines.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "theme",
    "icon_theme",
    "buffer_font_family",
    "buffer_font_size",
    "buffer_font_weight",
    "buffer_line_height",
    "ui_font_family",
    "ui_font_size",
    "ui_font_weight",
    "vim_mode",
    "vim",
    "helix_mode",
    "autosave",
    "tab_size",
    "preferred_line_length",
    "hard_tabs",
    "languages",
    "lsp",
    "assistant",
    "agent",
    "terminal",
    "project_panel",
    "git",
    "inlay_hints",
    "features",
    "journal",
    "base_keymap",
    "auto_update",
    "confirm_quit",
    "cursor_blink",
    "indent_guides",
    "scrollbar",
    "tabs",
    "toolbar",
    "wrap_guides",
    "seed_search_query_from_cursor",
    "format_on_save",
    "formatter",
    "remove_trailing_whitespace_on_save",
    "ensure_final_newline_on_save",
    "soft_wrap",
    "show_whitespaces",
    "proxy",
    "node",
    "ssh_connections",
    "close_on_file_delete",
    "restore_on_startup",
    "preview_tabs",
    "search",
    "file_scan_exclusions",
    "private_files",
    "collaboration_panel",
    "outline_panel",
    "notification_panel",
    "chat_panel",
    "message_editor",
    "drop_target_size",
    "when_closing_with_no_tabs",
    "use_on_type_format",
    "active_pane_magnification",
    "centered_layout",
    "context_servers",
    "slash_commands",
    "inline_completions",
    "edit_predictions",
    "evaluatable_expr",
    "diagnostics",
    "status_bar",
    "title_bar",
    "tab_bar",
    "pane_split_direction_horizontal",
    "pane_split_direction_vertical",
    "relative_line_numbers",
    "multi_cursor_modifier",
    "redact_private_values",
    "session",
    "show_call_status_icon",
    "auto_signature_help",
    "hover_popover_enabled",
    "hover_popover_delay",
    "double_click_in_multibuffer",
    "exand_excerpt_lines",
    "expand_excerpt_lines",
    "unnecessary_code_fade",
    "current_line_highlight",
    "gutter",
    "rounded_selection",
    "custom_confs",
    "colorize_brackets",
    "go_to_definition_fallback",
    "jippy",
    "elasticsearch",
    "telltales",
    "mutable_virtual_text",
    "disable_ai",
    "debugger",
    "dap",
    "profile",
    "minimap",
    "signature_help",
    "drag_and_drop_selection",
    "auto_install_extensions",
    "auto_update_extensions",
    "version_control",
    "keymap",
    "keymap_file",
];

/// Detect Zed settings.json content.
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
        "\"buffer_font",
        "\"vim_mode\"",
        "\"base_keymap\"",
        "\"ui_font",
        "\"zed\"",
        "\"theme\"",
    ] {
        if t.contains(p) {
            hits += 1;
        }
    }
    hits >= 2
}

impl Zedconf {
    /// Census a Zed settings buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            settings: 0,
            named: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.starts_with("//") || s.starts_with("/*") {
                c.comments += 1;
            }
        }
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
                        if KEYS.contains(&key) {
                            c.named += 1;
                        }
                    }
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
    fn detects_conf() {
        let b = br#"{ "theme": "One Dark", "vim_mode": true }"#;
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
            "  // zed\n",
            "  \"theme\": \"One Dark\",\n",
            "  \"buffer_font_family\": \"JetBrains Mono\",\n",
            "  \"buffer_font_size\": 14,\n",
            "  \"buffer_line_height\": \"comfortable\",\n",
            "  \"vim_mode\": true,\n",
            "  \"base_keymap\": \"VSCode\",\n",
            "  \"tab_size\": 4,\n",
            "  \"hard_tabs\": false,\n",
            "  \"preferred_line_length\": 100,\n",
            "  \"autosave\": \"on_focus_change\",\n",
            "  \"format_on_save\": \"on\",\n",
            "  \"soft_wrap\": \"editor_width\",\n",
            "  \"show_whitespaces\": \"boundary\",\n",
            "  \"ui_font_size\": 16,\n",
            "  \"auto_update\": false,\n",
            "  \"confirm_quit\": true,\n",
            "  \"cursor_blink\": false,\n",
            "  \"telemetry\": {\n",
            "    \"metrics\": false\n",
            "  },\n",
            "  \"project_panel\": {\n",
            "    \"dock\": \"left\"\n",
            "  },\n",
            "  \"git\": {\n",
            "    \"git_gutter\": \"tracked_files\"\n",
            "  },\n",
            "  \"terminal\": {\n",
            "    \"font_size\": 13,\n",
            "    \"dock\": \"bottom\"\n",
            "  },\n",
            "  \"lsp\": {\n",
            "    \"rust-analyzer\": {\n",
            "      \"initialization_options\": {}\n",
            "    }\n",
            "  }\n",
            "}\n",
        );
        let c = Zedconf::parse(b.as_bytes()).unwrap();
        assert!(c.settings >= 18);
        assert!(c.named >= 15);
        assert_eq!(c.comments, 1);
    }
}
