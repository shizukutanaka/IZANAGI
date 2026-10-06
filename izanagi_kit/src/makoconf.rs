//! `mako` 設定ファイル検出モジュール。
//!
//! mako (Wayland 通知デーモン) の設定はフラットな `key=value` 行で、
//! `max-visible`/`sort`/`layer`/`anchor`/`font`/`background-color`
//! 等のオプションと `[criteria]` ブロックが使われる。
//!
//! ```
//! let b = br#"max-visible=5
//! sort=-time
//! layer=overlay
//! anchor=top-right
//! font=monospace 10
//! background-color=#285577
//! "#;
//! let c = izanagi_kit::makoconf::parse(b);
//! assert!(izanagi_kit::makoconf::detect(b));
//! assert_eq!(c.keys, 6);
//! ```

const KEYS: &[&str] = &[
    "anchor",
    "background-color",
    "border-color",
    "border-radius",
    "border-size",
    "button-font",
    "default-timeout",
    "font",
    "format",
    "group-by",
    "height",
    "hidden-format",
    "history",
    "icons",
    "icon-path",
    "icon-border-radius",
    "ignore-timeout",
    "invisible",
    "layer",
    "margin",
    "max-history",
    "max-icon-size",
    "max-visible",
    "on-button-left",
    "on-button-middle",
    "on-button-right",
    "on-touch",
    "output",
    "padding",
    "progress-color",
    "sort",
    "text-color",
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

/// `b` が mako 設定に見えるかを返す。
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

/// mako 設定の統計。
#[derive(Debug, Default, Clone)]
pub struct MakoConf {
    /// 既知オプションの代入行数。
    pub keys: usize,
    /// 全代入行数。
    pub assignments: usize,
    /// `[criteria]` ブロック数。
    pub criteria: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を mako 設定として統計する。
pub fn parse(b: &[u8]) -> MakoConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = MakoConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if is_comment(tr) {
            c.comments += 1;
            continue;
        }
        if tr.starts_with('[') && tr.ends_with(']') {
            c.criteria += 1;
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
        let b = br#"max-visible=5
sort=-time
layer=overlay
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 3);
    }

    #[test]
    fn detects_criteria() {
        let b = br#"max-visible=5
font=sans-serif 11
background-color=#222
[app-name=firefox]
default-timeout=15000
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.criteria, 1);
        assert_eq!(c.keys, 4);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"max-visible=5\n"));
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
