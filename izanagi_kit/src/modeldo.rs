//! `.do` (ModelSim/Questa Tcl スクリプト) 検出モジュール。
//!
//! ModelSim/QuestaSim の .do ファイルは `vlib`/`vlog`/`vcom`/`vsim`/
//! `add wave`/`run`/`view`/`quit` 等のシミュレーションコマンドで
//! 構成される。
//!
//! ```
//! let b = br#"vlib work
//! vlog top.v
//! vsim work.top -novopt
//! add wave -r /*
//! run 1000ns
//! quit -f
//! "#;
//! let c = izanagi_kit::modeldo::parse(b);
//! assert!(izanagi_kit::modeldo::detect(b));
//! assert_eq!(c.command_lines, 6);
//! ```

const COMMANDS: &[&str] = &[
    "add wave",
    "add list",
    "add log",
    "alias",
    "bookmark",
    "break",
    "dataset",
    "do",
    "drivers",
    "echo",
    "examine",
    "force",
    "help",
    "log",
    "mem display",
    "noforce",
    "noview",
    "pause",
    "power add",
    "power report",
    "power reset",
    "project",
    "quit",
    "radix",
    "restart",
    "run",
    "search",
    "step",
    "transcript",
    "vcd add",
    "vcd dumpports",
    "vcd file",
    "vcd files",
    "vcom",
    "vcd2wlf",
    "vdel",
    "vdir",
    "virtual",
    "vlib",
    "vlog",
    "vmake",
    "vmap",
    "vsim",
    "view",
    "wave",
    "wlf",
    "write format",
    "write report",
];

fn is_comment(t: &str) -> bool {
    t.starts_with('#') || t.starts_with("//")
}

fn command_hit(t: &str) -> bool {
    COMMANDS
        .iter()
        .any(|c| t == *c || t.starts_with(&format!("{c} ")))
}

/// `b` が ModelSim/Questa .do に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut cmds = 0usize;
    let mut sims = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || is_comment(tr) {
            continue;
        }
        if command_hit(tr) {
            cmds += 1;
            if tr.starts_with("vlib")
                || tr.starts_with("vlog")
                || tr.starts_with("vcom")
                || tr.starts_with("vsim")
            {
                sims += 1;
            }
        }
    }
    (sims >= 1 && cmds >= 3) || cmds >= 5
}

/// .do の統計。
#[derive(Debug, Default, Clone)]
pub struct ModelDo {
    /// 既知コマンド行数。
    pub command_lines: usize,
    /// vlib/vlog/vcom/vsim 行数。
    pub sim_lines: usize,
    /// add wave/run 行数。
    pub wave_lines: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を .do として統計する。
pub fn parse(b: &[u8]) -> ModelDo {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = ModelDo::default();
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
            if tr.starts_with("vlib")
                || tr.starts_with("vlog")
                || tr.starts_with("vcom")
                || tr.starts_with("vsim")
            {
                c.sim_lines += 1;
            }
            if tr.starts_with("add wave") || tr.starts_with("run") {
                c.wave_lines += 1;
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
        let b = br#"vlib work
vlog top.v
vsim work.top
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.sim_lines, 3);
    }

    #[test]
    fn detects_wave() {
        let b = br#"vlib work
vcom top.vhd
vsim work.top
add wave -r /*
run 1000ns
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.wave_lines, 2);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"vlog top.v\n"));
        assert!(!detect(b"run\n"));
        assert!(!detect(b"echo hi\nls -la\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.command_lines, 0);
    }
}
