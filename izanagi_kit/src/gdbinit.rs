//! `.gdbinit` / `gdbinit` 検出モジュール。
//!
//! GDB の初期化ファイルは `set`/`define`/`source`/`target`/`break`/
//! `handle`/`python`/`add-auto-load-safe-path` 等のコマンドで
//! 構成される。
//!
//! ```
//! let b = br#"set pagination off
//! set print pretty on
//! set history save on
//! add-auto-load-safe-path /usr/local/lib
//! handle SIGPIPE nostop noprint
//! define hook-quit
//! end
//! "#;
//! let c = izanagi_kit::gdbinit::parse(b);
//! assert!(izanagi_kit::gdbinit::detect(b));
//! assert_eq!(c.command_lines, 7);
//! ```

const COMMANDS: &[&str] = &[
    "add-auto-load-safe-path",
    "add-demi-symbol-file",
    "alias",
    "apropos",
    "attach",
    "backtrace",
    "break",
    "catch",
    "cd",
    "commands",
    "define",
    "delete",
    "detach",
    "directory",
    "disable",
    "display",
    "document",
    "down",
    "echo",
    "enable",
    "end",
    "file",
    "finish",
    "frame",
    "handle",
    "info",
    "jump",
    "kill",
    "list",
    "load",
    "macro",
    "next",
    "print",
    "python",
    "quit",
    "run",
    "set",
    "shell",
    "source",
    "step",
    "target",
    "thread",
    "undisplay",
    "unset",
    "until",
    "up",
    "watch",
    "where",
];

fn is_comment(t: &str) -> bool {
    t.starts_with('#')
}

fn command_hit(t: &str) -> bool {
    let head = t.split_whitespace().next().unwrap_or("");
    COMMANDS.contains(&head)
}

/// `b` が .gdbinit に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut cmds = 0usize;
    let mut sets = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || is_comment(tr) {
            continue;
        }
        if command_hit(tr) {
            cmds += 1;
            if tr.starts_with("set ") || tr == "set" {
                sets += 1;
            }
        }
    }
    (sets >= 1 && cmds >= 3) || cmds >= 5
}

/// .gdbinit の統計。
#[derive(Debug, Default, Clone)]
pub struct Gdbinit {
    /// 既知コマンド行数。
    pub command_lines: usize,
    /// `set` 行数。
    pub set_lines: usize,
    /// `define`/`document`/`end` 行数。
    pub define_blocks: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を .gdbinit として統計する。
pub fn parse(b: &[u8]) -> Gdbinit {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = Gdbinit::default();
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
            if tr.starts_with("set ") || tr == "set" {
                c.set_lines += 1;
            }
            if tr.starts_with("define") || tr.starts_with("document") || tr == "end" {
                c.define_blocks += 1;
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
        let b = br#"set pagination off
set print pretty on
set history save on
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.set_lines, 3);
    }

    #[test]
    fn detects_full() {
        let b = br#"set pagination off
handle SIGPIPE nostop noprint
source /usr/share/gdb/init.py
break main
run
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.command_lines, 5);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"set x 1\n"));
        assert!(!detect(b"echo hi\nmkdir x\n"));
        assert!(!detect(b"run\nquit\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.command_lines, 0);
    }
}
