//! `.octaverc` 検出モジュール。
//!
//! Octave 起動スクリプトは `addpath`/`pkg load`/`more off`/`PS1`/
//! `format`/`cd`/`setdefaultfigure`/`suppress_verbose_help_message`
//! 等のコマンドで構成される。
//!
//! ```
//! let b = br#"addpath ~/octave/lib
//! addpath ~/octave/scripts
//! pkg load io
//! more off
//! format compact
//! PS1('>> ')
//! "#;
//! let c = izanagi_kit::octaverc::parse(b);
//! assert!(izanagi_kit::octaverc::detect(b));
//! assert_eq!(c.command_lines, 6);
//! ```

const COMMANDS: &[&str] = &[
    "addpath",
    "ans",
    "cd",
    "chdir",
    "clear",
    "commandhistory",
    "diary",
    "dir",
    "disp",
    "echo",
    "edit",
    "eval",
    "exist",
    "exit",
    "format",
    "genpath",
    "global",
    "graphics_toolkit",
    "help",
    "history",
    "isguirunning",
    "ls",
    "more",
    "num2str",
    "pkg",
    "printf",
    "pwd",
    "quit",
    "rehash",
    "restoredefaultpath",
    "rmpath",
    "run",
    "save",
    "savepath",
    "set",
    "setdefaultfigure",
    "source",
    "suppress_verbose_help_message",
    "typeinfo",
    "uicontrol",
    "warning",
    "which",
    "who",
    "x11",
];

fn is_comment(t: &str) -> bool {
    t.starts_with('#') || t.starts_with('%')
}

fn command_hit(t: &str) -> bool {
    let head = t.split_whitespace().next().unwrap_or("");
    let head = head.trim_end_matches(['(', ';']);
    COMMANDS.contains(&head) || head == "PS1" || head.starts_with("PS1")
}

/// `b` が .octaverc に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut cmds = 0usize;
    let mut oct = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || is_comment(tr) {
            continue;
        }
        if command_hit(tr) {
            cmds += 1;
            if tr.starts_with("addpath")
                || tr.starts_with("pkg ")
                || tr.starts_with("rmpath")
                || tr.starts_with("more ")
                || tr.starts_with("PS1")
                || tr.starts_with("setdefaultfigure")
            {
                oct += 1;
            }
        }
    }
    (oct >= 1 && cmds >= 3) || cmds >= 5
}

/// .octaverc の統計。
#[derive(Debug, Default, Clone)]
pub struct Octaverc {
    /// 既知コマンド行数。
    pub command_lines: usize,
    /// addpath/pkg load/more off/PS1 等 Octave 固有行数。
    pub octave_lines: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を .octaverc として統計する。
pub fn parse(b: &[u8]) -> Octaverc {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = Octaverc::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if is_comment(tr) {
            c.comments += 1;
            continue;
        }
        if command_hit(tr) {
            c.command_lines += 1;
            if tr.starts_with("addpath")
                || tr.starts_with("pkg ")
                || tr.starts_with("rmpath")
                || tr.starts_with("more ")
                || tr.starts_with("PS1")
                || tr.starts_with("setdefaultfigure")
            {
                c.octave_lines += 1;
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
        let b = br#"addpath ~/octave/lib
pkg load io
more off
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.octave_lines, 3);
    }

    #[test]
    fn detects_general() {
        let b = br#"format compact
cd ~
clear all
warning off
exit
"#;
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"addpath x\n"));
        assert!(!detect(b"cd /tmp\n"));
        assert!(!detect(b"foo bar\nbaz qux\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.command_lines, 0);
    }
}
