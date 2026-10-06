//! `wayfire.ini` 検出モジュール。
//!
//! Wayfire コンポジタの INI 形式設定。`[core]`/`[command]`/
//! `[autostart]`/`[input]`/`[output]` セクションと `plugins = ` 等の
//! `key = value` 行が特徴。
//!
//! ```
//! let b = br#"[core]
//! plugins = autostart command cube expo move vswitch wrot
//! close_top_view = <super> KEY_Q
//! [command]
//! binding_terminal = <super> KEY_ENTER
//! [autostart]
//! panel = wf-panel
//! "#;
//! let c = izanagi_kit::wayfire::parse(b);
//! assert!(izanagi_kit::wayfire::detect(b));
//! assert_eq!(c.sections, 3);
//! ```

const SECTIONS: &[&str] = &[
    "animate",
    "autostart",
    "command",
    "core",
    "cube",
    "decoration",
    "expo",
    "extra-commands",
    "grid",
    "idle",
    "input",
    "invert",
    "move",
    "output",
    "place",
    "scale",
    "simple-tile",
    "switcher",
    "vswitch",
    "vswipe",
    "window-rules",
    "wobbly",
    "workarounds",
    "wrot",
];

fn is_comment(t: &str) -> bool {
    t.starts_with('#')
}

fn section_name(t: &str) -> Option<&str> {
    if !(t.starts_with('[') && t.ends_with(']')) {
        return None;
    }
    Some(&t[1..t.len() - 1])
}

/// `b` が wayfire.ini に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut secs = 0usize;
    let mut in_sec = false;
    let mut assigns = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || is_comment(tr) {
            continue;
        }
        if let Some(n) = section_name(tr) {
            in_sec = SECTIONS.contains(&n);
            if in_sec {
                secs += 1;
            }
            continue;
        }
        if in_sec && tr.contains('=') {
            assigns += 1;
        }
    }
    secs >= 1 && assigns >= 1
}

/// wayfire.ini の統計。
#[derive(Debug, Default, Clone)]
pub struct WayfireIni {
    /// 既知セクション数。
    pub sections: usize,
    /// セクション内 `key = value` 行数。
    pub assignments: usize,
    /// `<modifier> KEY` バインディング行数。
    pub bindings: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を wayfire.ini として統計する。
pub fn parse(b: &[u8]) -> WayfireIni {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = WayfireIni::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if is_comment(tr) {
            c.comments += 1;
            continue;
        }
        if let Some(n) = section_name(tr) {
            if SECTIONS.contains(&n) {
                c.sections += 1;
            }
            continue;
        }
        if tr.contains('=') {
            c.assignments += 1;
            if tr.contains('<') && tr.contains('>') {
                c.bindings += 1;
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
        let b = br#"[core]
plugins = autostart command cube
close_top_view = <super> KEY_Q
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.sections, 1);
        assert_eq!(c.assignments, 2);
        assert_eq!(c.bindings, 1);
    }

    #[test]
    fn detects_autostart() {
        let b = br#"[autostart]
panel = wf-panel
background = wf-background
"#;
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"[foo]\nbar = 1\n"));
        assert!(!detect(b"[core]\n"));
        assert!(!detect(b"key = value\nother = 2\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.sections, 0);
    }
}
