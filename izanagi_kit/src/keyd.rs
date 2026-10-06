//! `/etc/keyd/*.conf` (keyd キーボードデーモン) 検出モジュール。
//!
//! keyd の設定は INI 風で、`[ids]`(デバイスID)/`[main]`/
//! `[<layer>]`/`[<layer>:<type>]` セクションと、`key = action` の
//! 割当で構成される。アクションは `overload(<layer>, <key>)`/
//! `oneshot(<layer>)`/`layer(<layer>)`/`toggle(<layer>)`/
//! `setlayout(...)`/`macro(...)`/`command(...)`/`overloadt(...)`/
//! `overloadi(...)`/`timeout(...)`/`swap(...)`/`clear()`/
//! `block()`/`noop` 等。
//!
//! ```
//! let b = b"[ids]\n\
//!           *\n\
//!           [main]\n\
//!           capslock = overload(control, esc)\n\
//!           leftshift = oneshot(shift)\n\
//!           [control]\n\
//!           h = left\n\
//!           j = down\n";
//! let c = izanagi_kit::keyd::parse(b);
//! assert!(izanagi_kit::keyd::detect(b));
//! assert_eq!(c.sections, 3);
//! ```

const ACTIONS: &[&str] = &[
    "block",
    "clear",
    "command",
    "compose",
    "keyd-gadget",
    "layer",
    "layout",
    "lettermod",
    "macro",
    "noop",
    "oneshot",
    "oneshotm",
    "overload",
    "overloaded",
    "overloadi",
    "overloadt",
    "overloadt2",
    "setlayout",
    "setmarks",
    "swap",
    "timeout",
    "toggle",
    "togglem",
];

fn is_section(t: &str) -> bool {
    t.starts_with('[') && t.ends_with(']') && !t.contains(' ') && t.len() > 2
}

fn has_action(t: &str) -> bool {
    let v = t.split('=').nth(1).unwrap_or("").trim();
    ACTIONS
        .iter()
        .any(|a| v == *a || v.starts_with(&format!("{a}(")))
}

/// `b` が keyd 設定に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut secs = 0usize;
    let mut acts = 0usize;
    let mut keyd_secs = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') || tr.starts_with(';') {
            continue;
        }
        if is_section(tr) {
            secs += 1;
            if tr == "[ids]"
                || tr == "[main]"
                || tr.starts_with("[layer")
                || tr == "[global]"
                || tr == "[aliases]"
                || tr.contains(':')
            {
                keyd_secs += 1;
            }
            continue;
        }
        if tr.contains('=') && has_action(tr) {
            acts += 1;
        }
    }
    (keyd_secs >= 1 && acts >= 1) || acts >= 3 || (keyd_secs >= 2 && secs >= 2)
}

/// keyd 設定の統計。
#[derive(Debug, Default, Clone)]
pub struct Keyd {
    /// セクション数。
    pub sections: usize,
    /// 既知アクション割当数。
    pub actions: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を keyd 設定として統計する。
pub fn parse(b: &[u8]) -> Keyd {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = Keyd::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') || tr.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if is_section(tr) {
            c.sections += 1;
            continue;
        }
        if tr.contains('=') && has_action(tr) {
            c.actions += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"[main]\ncapslock = overload(control, esc)\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.sections, 1);
        assert_eq!(c.actions, 1);
    }

    #[test]
    fn detects_actions() {
        let b = b"a = overload(ctrl, x)\nb = oneshot(shift)\nc = layer(nav)\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"[main]\nkey = x\n"));
        assert!(!detect(b"key = value\nfoo = bar\nbaz = quux\n"));
        assert!(!detect(b"[section]\n[other]\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.sections, 0);
    }
}
