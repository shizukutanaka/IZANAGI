//! Contour ターミナル `contour.yml` パーサ。
//!
//! `profiles:`/`color_schemes:`/`word_delimiters`/`bypass_mouse_protocol_modifier`/
//! `spawn_new_process` 等の既知トップキーと `main:` プロファイルキーを計数する。
//!
//! ```
//! use izanagi_kit::contourconf;
//! let conf = b"word_delimiters: \" /\\'\"()[]{}<>|\\\"\nprofiles:\n  main:\n    shell: /bin/zsh\n    terminal_size: { columns: 100, lines: 30 }\n";
//! assert!(contourconf::detect(conf));
//! let c = contourconf::parse(conf).unwrap();
//! assert_eq!(c.known_tops, 2);
//! ```

/// 判定結果の計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// col0 `key:` トップキー数。
    pub top_keys: usize,
    /// 既知トップキー数。
    pub known_tops: usize,
    /// インデント `key:` 行数。
    pub nested_keys: usize,
    /// `-` リスト項目数。
    pub list_items: usize,
}

const KNOWN_TOPS: &[&str] = &[
    "word_delimiters",
    "read_buffer_size",
    "pty_buffer_size",
    "pty_read_buffer_size",
    "spawn_new_process",
    "reflow_on_resize",
    "bypass_mouse_protocol_modifier",
    "on_mouse_select",
    "mouse_block_selection_modifier",
    "profiles",
    "color_schemes",
    "input_mapping",
    "platform_plugin",
    "renderer",
    "text_shaper",
    "font_locator",
    "persistent_logging",
    "file_logging",
    "file_logging_path",
    "file_logging_level",
    "logging",
    "notifications",
    "bar_position",
    "background",
    "scrollbar",
    "permissions",
    "sixel",
    "vi_scrolling",
    "images",
    "bypass_mouse_protocol",
    "early_log_threshold",
    "keyboard_layout",
];

/// `contour.yml` らしさを簡易判定する。
pub fn detect(b: &[u8]) -> bool {
    let c = parse(b);
    match c {
        Some(c) => c.known_tops >= 2 || (c.known_tops >= 1 && c.nested_keys >= 4),
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
        top_keys: 0,
        known_tops: 0,
        nested_keys: 0,
        list_items: 0,
    };
    for l in s.lines() {
        let t = l.trim_end();
        let tr = t.trim_start();
        if tr.is_empty() || tr.starts_with('#') {
            continue;
        }
        if tr.starts_with("- ") || tr == "-" {
            c.list_items += 1;
            continue;
        }
        let Some(colon) = tr.find(':') else {
            continue;
        };
        let key = tr[..colon].trim();
        if key.is_empty()
            || !key
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'-'))
        {
            continue;
        }
        if t.starts_with(tr) {
            c.top_keys += 1;
            if KNOWN_TOPS.contains(&key) {
                c.known_tops += 1;
            }
        } else {
            c.nested_keys += 1;
        }
    }
    (c.top_keys > 0 || c.nested_keys > 0).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"word_delimiters: \" /\\'\"()[]{}<>|\\\"\nspawn_new_process: false\nprofiles:\n  main:\n    shell: /bin/zsh\n    terminal_size: { columns: 100, lines: 30 }\n    font:\n      size: 12\ncolor_schemes:\n  - name: nord\n";

    #[test]
    fn detects_contour() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.top_keys, 4);
        assert_eq!(c.known_tops, 4);
        assert_eq!(c.list_items, 1);
    }

    #[test]
    fn rejects_other_yaml() {
        let y = b"name: x\nversion: 1\nservices:\n  a:\n    image: img\n";
        assert!(!detect(y));
    }
}
