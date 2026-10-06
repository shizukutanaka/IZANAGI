//! `.lldbinit` 検出モジュール。
//!
//! LLDB の初期化ファイルは `settings set`/`command alias`/
//! `command script`/`target create`/`breakpoint set`/`plugin load`
//! 等のコマンドで構成される。
//!
//! ```
//! let b = br#"settings set target.inline-breakpoint-strategy always
//! settings set symbols.enable-external-lookup false
//! command alias fn frame variable
//! command script import ~/lldb.py
//! breakpoint set -n main
//! "#;
//! let c = izanagi_kit::lldbinit::parse(b);
//! assert!(izanagi_kit::lldbinit::detect(b));
//! assert_eq!(c.command_lines, 5);
//! ```

const COMMANDS: &[&str] = &[
    "apropos",
    "breakpoint",
    "command",
    "disassemble",
    "expression",
    "frame",
    "help",
    "image",
    "language",
    "log",
    "memory",
    "platform",
    "plugin",
    "process",
    "quit",
    "register",
    "script",
    "settings",
    "source",
    "statistics",
    "target",
    "thread",
    "type",
    "version",
    "watchpoint",
    "x",
];

fn is_comment(t: &str) -> bool {
    t.starts_with('#') || t.starts_with("//") || t.starts_with(';')
}

fn command_hit(t: &str) -> bool {
    let head = t.split_whitespace().next().unwrap_or("");
    COMMANDS.contains(&head)
}

/// `b` が .lldbinit に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut cmds = 0usize;
    let mut settings = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || is_comment(tr) {
            continue;
        }
        if command_hit(tr) {
            cmds += 1;
            if tr.starts_with("settings ") || tr == "settings" {
                settings += 1;
            }
        }
    }
    (settings >= 1 && cmds >= 3) || cmds >= 5
}

/// .lldbinit の統計。
#[derive(Debug, Default, Clone)]
pub struct Lldbinit {
    /// 既知コマンド行数。
    pub command_lines: usize,
    /// `settings` 行数。
    pub settings_lines: usize,
    /// `command` 行数（alias/script）。
    pub command_aliases: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を .lldbinit として統計する。
pub fn parse(b: &[u8]) -> Lldbinit {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = Lldbinit::default();
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
            if tr.starts_with("settings ") || tr == "settings" {
                c.settings_lines += 1;
            }
            if tr.starts_with("command ") || tr == "command" {
                c.command_aliases += 1;
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
        let b = br#"settings set target.inline-breakpoint-strategy always
settings set symbols.enable-external-lookup false
command alias fn frame variable
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.settings_lines, 2);
        assert_eq!(c.command_aliases, 1);
    }

    #[test]
    fn detects_many() {
        let b = br#"command script import ~/lldb.py
breakpoint set -n main
target create app
process launch
frame variable
"#;
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"settings set x y\n"));
        assert!(!detect(b"echo hi\nmkdir x\n"));
        assert!(!detect(b"quit\nhelp\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.command_lines, 0);
    }
}
