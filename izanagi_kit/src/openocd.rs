//! `openocd.cfg` 検出モジュール。
//!
//! OpenOCD の設定は Tcl スクリプトで、`source [find ...]`、
//! `adapter_khz`、`jtag newtap`、`target create`、`init`、
//! `reset_config` 等のコマンドが特徴。
//!
//! ```
//! let b = br#"source [find interface/stlink.cfg]
//! source [find target/stm32f4x.cfg]
//! adapter_khz 1800
//! reset_config srst_only
//! init
//! "#;
//! let c = izanagi_kit::openocd::parse(b);
//! assert!(izanagi_kit::openocd::detect(b));
//! assert_eq!(c.command_lines, 3);
//! assert_eq!(c.source_find_lines, 2);
//! ```

const COMMANDS: &[&str] = &[
    "adapter",
    "adapter_khz",
    "adapter_nsrst_delay",
    "arm",
    "cortex_a",
    "cortex_m",
    "dap",
    "echo",
    "exit",
    "exit_error",
    "halt",
    "init",
    "interface",
    "interface_name",
    "irscan",
    "jtag",
    "mww",
    "nand",
    "nucleus",
    "plld",
    "poll",
    "reg",
    "reset",
    "reset_config",
    "resume",
    "rtos",
    "scan_chain",
    "shutdown",
    "sleep",
    "soft_reset_halt",
    "source",
    "startup",
    "step",
    "target",
    "tcl_port",
    "telnet_port",
    "transport",
    "wait_halt",
];

fn is_comment(t: &str) -> bool {
    t.starts_with('#')
}

fn source_find(t: &str) -> bool {
    t.starts_with("source") && t.contains("[find")
}

fn command_hit(t: &str) -> bool {
    if source_find(t) {
        return true;
    }
    let head = t.split_whitespace().next().unwrap_or("");
    COMMANDS
        .iter()
        .any(|c| head == *c || head.starts_with(&format!("{c}_")))
}

/// `b` が openocd.cfg に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut srcs = 0usize;
    let mut cmds = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || is_comment(tr) {
            continue;
        }
        if source_find(tr) {
            srcs += 1;
        } else if command_hit(tr) {
            cmds += 1;
        }
    }
    srcs >= 1 && cmds >= 1 || cmds >= 3
}

/// openocd.cfg の統計。
#[derive(Debug, Default, Clone)]
pub struct OpenocdConf {
    /// `source [find ...]` 行数。
    pub source_find_lines: usize,
    /// その他の既知コマンド行数。
    pub command_lines: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を openocd.cfg として統計する。
pub fn parse(b: &[u8]) -> OpenocdConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = OpenocdConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if is_comment(tr) {
            c.comments += 1;
            continue;
        }
        if source_find(tr) {
            c.source_find_lines += 1;
        } else if command_hit(tr) {
            c.command_lines += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"source [find interface/stlink.cfg]
adapter_khz 1800
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.source_find_lines, 1);
        assert_eq!(c.command_lines, 1);
    }

    #[test]
    fn detects_commands() {
        let b = br#"adapter_khz 1800
reset_config srst_only
init
"#;
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"source foo\n"));
        assert!(!detect(b"echo hi\nls -la\nmkdir x\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.command_lines, 0);
    }
}
