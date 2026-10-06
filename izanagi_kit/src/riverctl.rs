//! river `init` 検出モジュール。
//!
//! river コンポジタの init ファイルは `riverctl` コマンド呼び出しで
//! 構成されるシェルスクリプト。`riverctl map`/`spawn`/
//! `set-option`-系のサブコマンドが特徴。
//!
//! ```
//! let b = br#"#!/bin/sh
//! riverctl map normal Super Return spawn foot
//! riverctl map normal Super+Shift Q close
//! riverctl border-width 2
//! riverctl keyboard-layout us
//! riverctl spawn "waybar"
//! riverctl default-layout rivertile
//! "#;
//! let c = izanagi_kit::riverctl::parse(b);
//! assert!(izanagi_kit::riverctl::detect(b));
//! assert_eq!(c.riverctl_lines, 6);
//! ```

fn is_comment(t: &str) -> bool {
    t.starts_with('#')
}

fn call_line(t: &str) -> bool {
    t.starts_with("riverctl ") || t == "riverctl"
}

/// `b` が river init に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut calls = 0usize;
    let mut other = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || is_comment(tr) || tr.starts_with("#!") {
            continue;
        }
        if call_line(tr) {
            calls += 1;
        } else {
            other += 1;
        }
    }
    (calls >= 3 && other <= calls) || calls >= 6
}

/// river init の統計。
#[derive(Debug, Default, Clone)]
pub struct RiverCtl {
    /// `riverctl` 呼び出し行数。
    pub riverctl_lines: usize,
    /// `riverctl map` 行数。
    pub map_lines: usize,
    /// `riverctl spawn` 行数。
    pub spawn_lines: usize,
    /// `riverctl` 以外の行数。
    pub shell_lines: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を river init として統計する。
pub fn parse(b: &[u8]) -> RiverCtl {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = RiverCtl::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with("#!") {
            continue;
        }
        if is_comment(tr) {
            c.comments += 1;
            continue;
        }
        if call_line(tr) {
            c.riverctl_lines += 1;
            if tr.starts_with("riverctl map") {
                c.map_lines += 1;
            } else if tr.starts_with("riverctl spawn") {
                c.spawn_lines += 1;
            }
        } else {
            c.shell_lines += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"riverctl map normal Super Return spawn foot
riverctl map normal Super Q close
riverctl border-width 2
riverctl default-layout rivertile
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.riverctl_lines, 4);
        assert_eq!(c.map_lines, 2);
    }

    #[test]
    fn detects_mixed_shell() {
        let b = br#"#!/bin/sh
rivertile -view-padding 6 &
riverctl spawn waybar
riverctl map normal Super Return spawn foot
riverctl map normal Super+Shift E exit
riverctl set-repeat 50 300
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.spawn_lines, 1);
        assert_eq!(c.shell_lines, 1);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"ls -la\ncd /tmp\necho hi\n"));
        assert!(!detect(b"riverctl map normal Super Q close\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.riverctl_lines, 0);
    }
}
