//! `.gnuplot`/`gnuplotrc` 検出モジュール。
//!
//! gnuplot の初期化スクリプトは `set`/`unset`/`plot`/`splot`/
//! `load`/`call`/`pause` コマンドで構成される。
//!
//! ```
//! let b = br#"set terminal pngcairo
//! set output 'out.png'
//! set title "demo"
//! set xrange [0:10]
//! set grid
//! unset key
//! plot sin(x)
//! "#;
//! let c = izanagi_kit::gnuplot::parse(b);
//! assert!(izanagi_kit::gnuplot::detect(b));
//! assert_eq!(c.set_lines, 6);
//! ```

const COMMANDS: &[&str] = &[
    "bind", "call", "cd", "clear", "exit", "fit", "help", "history", "if", "import", "load",
    "lower", "pause", "plot", "print", "printerr", "pwd", "quit", "raise", "refresh", "replot",
    "reread", "reset", "save", "set", "show", "splot", "stats", "system", "test", "toggle",
    "undefine", "unset", "update",
];

fn is_comment(t: &str) -> bool {
    t.starts_with('#')
}

fn command_hit(t: &str) -> bool {
    let head = t.split_whitespace().next().unwrap_or("");
    COMMANDS.contains(&head)
}

/// `b` が gnuplotrc に見えるかを返す。
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
            if tr.starts_with("set ") || tr.starts_with("unset ") {
                sets += 1;
            }
        }
    }
    (sets >= 2 && cmds >= 3) || cmds >= 6
}

/// gnuplotrc の統計。
#[derive(Debug, Default, Clone)]
pub struct GnuplotRc {
    /// `set`/`unset` 行数。
    pub set_lines: usize,
    /// `plot`/`splot`/`replot` 行数。
    pub plot_lines: usize,
    /// 既知コマンド行総数。
    pub command_lines: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を gnuplotrc として統計する。
pub fn parse(b: &[u8]) -> GnuplotRc {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = GnuplotRc::default();
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
            if tr.starts_with("set ") || tr.starts_with("unset ") {
                c.set_lines += 1;
            }
            if tr.starts_with("plot") || tr.starts_with("splot") || tr.starts_with("replot") {
                c.plot_lines += 1;
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
        let b = br#"set terminal pngcairo
set output 'out.png'
set grid
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.set_lines, 3);
    }

    #[test]
    fn detects_plot() {
        let b = br#"set terminal pngcairo
set output 'x.png'
set title "d"
set xrange [0:10]
plot sin(x)
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.plot_lines, 1);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"set x 1\n"));
        assert!(!detect(b"plot sin(x)\n"));
        assert!(!detect(b"foo bar\nbaz qux\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.set_lines, 0);
    }
}
