//! `bspwmrc` 検出モジュール。
//!
//! bspwm の設定は `bspc` コマンド呼び出しで構成されるシェルスクリプト。
//! `bspc config <key> <value>`、`bspc rule -a <class>`、
//! `bspc monitor <name> -d <desktops>` が主要パターン。
//!
//! ```
//! let b = br#"#! /bin/sh
//! bspc monitor -d I II III IV
//! bspc config border_width 2
//! bspc config window_gap 12
//! bspc config split_ratio 0.52
//! bspc config borderless_monocle true
//! bspc rule -a Gimp desktop='^8'
//! "#;
//! let c = izanagi_kit::bspwmrc::parse(b);
//! assert!(izanagi_kit::bspwmrc::detect(b));
//! assert_eq!(c.bspc_lines, 6);
//! ```

fn is_comment(t: &str) -> bool {
    t.starts_with('#')
}

fn bspc_line(t: &str) -> bool {
    t.starts_with("bspc ") || t == "bspc"
}

/// `b` が bspwmrc に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut calls = 0usize;
    let mut non_bspc = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || is_comment(tr) || tr.starts_with("#!") {
            continue;
        }
        if bspc_line(tr) {
            calls += 1;
        } else {
            non_bspc += 1;
        }
    }
    (calls >= 3 && non_bspc <= calls) || calls >= 6
}

/// bspwmrc の統計。
#[derive(Debug, Default, Clone)]
pub struct Bspwmrc {
    /// `bspc` 呼び出し行数。
    pub bspc_lines: usize,
    /// `bspc config` 行数。
    pub config_lines: usize,
    /// `bspc rule` 行数。
    pub rule_lines: usize,
    /// `bspc monitor` 行数。
    pub monitor_lines: usize,
    /// `bspc` 以外の行数（シェルコマンド等）。
    pub shell_lines: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を bspwmrc として統計する。
pub fn parse(b: &[u8]) -> Bspwmrc {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = Bspwmrc::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with("#!") {
            continue;
        }
        if is_comment(tr) {
            c.comments += 1;
            continue;
        }
        if bspc_line(tr) {
            c.bspc_lines += 1;
            if tr.starts_with("bspc config") {
                c.config_lines += 1;
            } else if tr.starts_with("bspc rule") {
                c.rule_lines += 1;
            } else if tr.starts_with("bspc monitor") {
                c.monitor_lines += 1;
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
        let b = br#"#!/bin/sh
bspc config border_width 2
bspc config window_gap 12
bspc config split_ratio 0.52
bspc config borderless_monocle true
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.bspc_lines, 4);
        assert_eq!(c.config_lines, 4);
    }

    #[test]
    fn detects_mixed_shell() {
        let b = br#"#!/bin/sh
sxhkd &
bspc monitor -d I II III
bspc rule -a Gimp desktop='^8' state=floating
bspc config border_width 2
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.shell_lines, 1);
        assert_eq!(c.rule_lines, 1);
        assert_eq!(c.monitor_lines, 1);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"ls -la\ncd /tmp\necho hi\n"));
        assert!(!detect(b"bspc config border_width 2\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.bspc_lines, 0);
    }
}
