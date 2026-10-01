//! xmodmap 設定ファイル (`.Xmodmap`) パーサ。
//!
//! `keycode N = keysym ...`、`keysym a = ...`、`pointer = ...`、
//! `clear`/`add`/`remove` モディファイア文を計数する。
//!
//! ```
//! use izanagi_kit::xmodmap;
//! let mm = b"keycode 24 = q Q at Greek_omega\nkeysym a = b c\nadd Mod1 = Alt_L\n";
//! assert!(xmodmap::detect(mm));
//! let c = xmodmap::parse(mm).unwrap();
//! assert_eq!(c.statements, 3);
//! assert_eq!(c.keycodes, 1);
//! ```

/// 判定結果の計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 有効ステートメント総数。
    pub statements: usize,
    /// `keycode` 文数。
    pub keycodes: usize,
    /// `keysym` 文数。
    pub keysyms: usize,
    /// `pointer` 文数。
    pub pointer: usize,
    /// `clear` 文数。
    pub clears: usize,
    /// `add` 文数。
    pub adds: usize,
    /// `remove` 文数。
    pub removes: usize,
}

/// 簡易判定 (`keycode`/`keysym`/`pointer`/`clear`/`add`/`remove` 文)。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Some(c) = parse(b) else {
        return false;
    };
    c.statements >= 3 && (c.keycodes + c.keysyms + c.pointer) >= 1
}

/// パースして計数を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let s = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        statements: 0,
        keycodes: 0,
        keysyms: 0,
        pointer: 0,
        clears: 0,
        adds: 0,
        removes: 0,
    };
    for line in s.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('!') {
            continue;
        }
        let Some(first) = t.split_whitespace().next() else {
            continue;
        };
        match first {
            "keycode" => c.keycodes += 1,
            "keysym" => c.keysyms += 1,
            "pointer" => c.pointer += 1,
            "clear" => c.clears += 1,
            "add" => c.adds += 1,
            "remove" => c.removes += 1,
            _ => continue,
        }
        c.statements += 1;
    }
    (c.statements > 0).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::{detect, parse};

    const SAMPLE: &[u8] = b"! swap caps and escape\nkeycode 24 = q Q at Greek_omega\nkeycode 66 = Mode_switch NoSymbol Mode_switch\nkeysym less = less less\npointer = 1 2 3 5 4\nclear Lock\nclear Mod1\nadd Mod1 = Alt_L Meta_L\nadd Mod4 = Super_L Super_R\nremove Shift = Shift_R\n";

    #[test]
    fn detects_xmodmap() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.statements, 9);
        assert_eq!(c.keycodes, 2);
        assert_eq!(c.keysyms, 1);
        assert_eq!(c.pointer, 1);
        assert_eq!(c.clears, 2);
        assert_eq!(c.adds, 2);
        assert_eq!(c.removes, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"#!/bin/sh\nexec i3\n"));
        assert!(!detect(b"keycode 24 = q\n"));
    }
}
