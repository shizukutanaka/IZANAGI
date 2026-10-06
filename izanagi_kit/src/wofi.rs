//! `wofi` 設定ファイル検出モジュール。
//!
//! wofi (Wayland ランチャ) の設定はフラットな `key=value` 行で、
//! `show`/`prompt`/`location`/`orientation`/`width`/`height` 等の
//! オプションが使われる。
//!
//! ```
//! let b = br#"show=drun
//! prompt=run
//! location=center
//! width=500
//! height=400
//! insensitivity=true
//! "#;
//! let c = izanagi_kit::wofi::parse(b);
//! assert!(izanagi_kit::wofi::detect(b));
//! assert_eq!(c.keys, 6);
//! ```

const KEYS: &[&str] = &[
    "allow_images",
    "allow_markup",
    "cache_file",
    "columns",
    "content_halign",
    "display_generic",
    "exec_search",
    "filter_rate",
    "hide_scroll",
    "height",
    "halign",
    "image_size",
    "insensitive",
    "insensitivity",
    "key_expand",
    "key_down",
    "key_hide",
    "key_left",
    "key_right",
    "key_up",
    "lines",
    "line_wrap",
    "location",
    "matching",
    "no_actions",
    "normal_window",
    "orientation",
    "password_char",
    "password_mode",
    "print_command",
    "prompt",
    "show",
    "term",
    "width",
];

fn is_comment(t: &str) -> bool {
    t.starts_with('#')
}

fn key_present(t: &str, k: &str) -> bool {
    if !t.starts_with(k) {
        return false;
    }
    let rest = t[k.len()..].trim_start();
    rest.starts_with('=')
}

/// `b` が wofi 設定に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut keys = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || is_comment(tr) {
            continue;
        }
        if KEYS.iter().any(|k| key_present(tr, k)) {
            keys += 1;
        }
    }
    keys >= 3
}

/// wofi 設定の統計。
#[derive(Debug, Default, Clone)]
pub struct WofiConf {
    /// 既知オプションの代入行数。
    pub keys: usize,
    /// 全代入行数。
    pub assignments: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を wofi 設定として統計する。
pub fn parse(b: &[u8]) -> WofiConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = WofiConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if is_comment(tr) {
            c.comments += 1;
            continue;
        }
        if tr.contains('=') {
            c.assignments += 1;
            if KEYS.iter().any(|k| key_present(tr, k)) {
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
        let b = br#"show=drun
prompt=apps
location=center
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 3);
    }

    #[test]
    fn detects_more() {
        let b = br#"show=drun
width=600
height=400
orientation=vertical
lines=5
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 5);
        assert_eq!(c.assignments, 5);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"show=drun\n"));
        assert!(!detect(b"foo=1\nbar=2\nbaz=3\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.keys, 0);
    }
}
