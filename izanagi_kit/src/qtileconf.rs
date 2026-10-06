//! qtile `config.py` 検出モジュール。
//!
//! Qtile ウィンドウマネージャの設定は Python スクリプトで、
//! `from libqtile import ...` や `keys = [...]` / `groups = [...]` /
//! `layouts = [...]` / `screens = [...]` のトップレベル代入が特徴。
//!
//! ```
//! let b = br#"from libqtile import bar, layout, widget
//! from libqtile.config import Key, Group, Screen
//! keys = [
//!     Key([mod], "Return", lazy.spawn("alacritty")),
//! ]
//! groups = [Group(i) for i in "1234"]
//! layouts = [layout.Columns()]
//! screens = [Screen(top=bar.Bar([widget.Clock()], 24))]
//! "#;
//! let c = izanagi_kit::qtileconf::parse(b);
//! assert!(izanagi_kit::qtileconf::detect(b));
//! assert!(c.list_assignments >= 4);
//! ```

const TOP_VARS: &[&str] = &[
    "auto_fullscreen",
    "bring_front_click",
    "cursor_warp",
    "dgroups_app_rules",
    "dgroups_key_binder",
    "extension_defaults",
    "floating_layout",
    "focus_on_window_activation",
    "follow_mouse_focus",
    "groups",
    "keys",
    "layouts",
    "mouse",
    "reconfigure_screens",
    "screens",
    "widget_defaults",
    "wl_input_rules",
    "wmname",
];

fn is_comment(t: &str) -> bool {
    t.starts_with('#')
}

fn libqtile_ref(t: &str) -> bool {
    t.contains("libqtile") || t.contains("from libqtile")
}

fn top_assign(t: &str, k: &str) -> bool {
    if !t.starts_with(k) {
        return false;
    }
    let rest = t[k.len()..].trim_start();
    rest.starts_with('=')
}

/// `b` が qtile config.py に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut qt_ref = false;
    let mut assigns = 0usize;
    for l in t.lines() {
        let tr = l.trim_start();
        if tr.is_empty() || is_comment(tr) {
            continue;
        }
        if libqtile_ref(tr) {
            qt_ref = true;
        }
        if TOP_VARS.iter().any(|k| top_assign(tr, k)) {
            assigns += 1;
        }
    }
    (qt_ref && assigns >= 1) || assigns >= 3
}

/// qtile config.py の統計。
#[derive(Debug, Default, Clone)]
pub struct QtileConf {
    /// libqtile を参照する行数（import 等）。
    pub libqtile_refs: usize,
    /// 既知トップレベル変数への代入行数。
    pub list_assignments: usize,
    /// `lazy.` 呼び出しを含む行数。
    pub lazy_calls: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を qtile config.py として統計する。
pub fn parse(b: &[u8]) -> QtileConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = QtileConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if is_comment(tr) {
            c.comments += 1;
            continue;
        }
        if libqtile_ref(tr) {
            c.libqtile_refs += 1;
        }
        if TOP_VARS.iter().any(|k| top_assign(l.trim_start(), k)) {
            c.list_assignments += 1;
        }
        if tr.contains("lazy.") {
            c.lazy_calls += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"from libqtile import bar, layout, widget
from libqtile.config import Key, Group, Screen
keys = [
    Key([mod], "Return", lazy.spawn("alacritty")),
]
"#;
        assert!(detect(b));
        let c = parse(b);
        assert!(c.libqtile_refs >= 2);
        assert!(c.lazy_calls >= 1);
    }

    #[test]
    fn detects_by_assigns() {
        let b = br#"keys = []
groups = []
layouts = []
screens = []
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.list_assignments, 4);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"keys = []\nprint(1)\n"));
        assert!(!detect(b"import os\nimport sys\nprint('hi')\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.list_assignments, 0);
    }
}
