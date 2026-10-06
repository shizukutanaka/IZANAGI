//! `.ys` (Yosys synthesis script) 検出モジュール。
//!
//! Yosys スクリプトは `read_verilog`/`hierarchy`/`proc`/`opt`/
//! `techmap`/`abc`/`synth_*`/`write_*` 等のコマンド行で構成される。
//!
//! ```
//! let b = br#"read_verilog top.v
//! hierarchy -check -top top
//! proc; opt
//! techmap; opt
//! abc -g AND,NAND
//! write_json out.json
//! "#;
//! let c = izanagi_kit::yosys::parse(b);
//! assert!(izanagi_kit::yosys::detect(b));
//! assert_eq!(c.command_lines, 6);
//! ```

const COMMANDS: &[&str] = &[
    "abc",
    "abc9",
    "check",
    "clean",
    "connect",
    "convert",
    "copy",
    "delete",
    "design",
    "dump",
    "equiv_",
    "eval",
    "extract",
    "flatten",
    "formalff",
    "fsm",
    "help",
    "hierarchy",
    "json",
    "log",
    "memory",
    "opt",
    "plugin",
    "prep",
    "proc",
    "read_",
    "rename",
    "scc",
    "script",
    "select",
    "setattr",
    "setenv",
    "setparam",
    "share",
    "show",
    "smtbmc",
    "splice",
    "splitnets",
    "stat",
    "submod",
    "synth",
    "synth_",
    "tcl",
    "techmap",
    "tee",
    "test_",
    "trace",
    "verific",
    "wreduce",
    "write_",
];

fn is_comment(t: &str) -> bool {
    t.starts_with('#')
}

fn command_hit(t: &str) -> bool {
    // first whitespace-separated token must start with a known command
    let head = t.split_whitespace().next().unwrap_or("");
    let head = head.trim_end_matches(';');
    COMMANDS.iter().any(|c| head.starts_with(c))
}

/// `b` が Yosys スクリプトに見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut cmds = 0usize;
    let mut reads = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || is_comment(tr) {
            continue;
        }
        if command_hit(tr) {
            cmds += 1;
            if tr.starts_with("read_") {
                reads += 1;
            }
        }
    }
    (reads >= 1 && cmds >= 2) || cmds >= 4
}

/// Yosys スクリプトの統計。
#[derive(Debug, Default, Clone)]
pub struct YosysScript {
    /// 既知コマンド行数。
    pub command_lines: usize,
    /// `read_*` 行数。
    pub reads: usize,
    /// `write_*` 行数。
    pub writes: usize,
    /// `synth*` 行数。
    pub synths: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を Yosys スクリプトとして統計する。
pub fn parse(b: &[u8]) -> YosysScript {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = YosysScript::default();
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
            if tr.starts_with("read_") {
                c.reads += 1;
            } else if tr.starts_with("write_") {
                c.writes += 1;
            } else if tr.starts_with("synth") {
                c.synths += 1;
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
        let b = br#"read_verilog top.v
hierarchy -check -top top
proc
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.command_lines, 3);
        assert_eq!(c.reads, 1);
    }

    #[test]
    fn detects_flow() {
        let b = br#"read_verilog top.v
synth_ice40 -top top
write_json out.json
stat
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.synths, 1);
        assert_eq!(c.writes, 1);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"read_verilog x\n"));
        assert!(!detect(b"ls\ncd\nmkdir\n"));
        assert!(!detect(b"proc x {} {}\nset a 1\nputs hi\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.command_lines, 0);
    }
}
