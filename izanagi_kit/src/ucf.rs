//! `.ucf` (Xilinx User Constraints File) 検出モジュール。
//!
//! 旧世代 Xilinx (ISE) の制約ファイルは `NET "name" LOC = "Pnn";`、
//! `TIMESPEC`、`TIMEGRP`、`PIN`、`AREA_GROUP` 等の制約行で
//! 構成される。
//!
//! ```
//! let b = br#"NET "clk" LOC = "E3" | IOSTANDARD = "LVCMOS33";
//! NET "rst_n" LOC = "B8";
//! NET "led<0>" LOC = "T9" | IOSTANDARD = "LVCMOS33";
//! TIMESPEC "TS_clk" = PERIOD "clk" 10 ns HIGH 50%;
//! "#;
//! let c = izanagi_kit::ucf::parse(b);
//! assert!(izanagi_kit::ucf::detect(b));
//! assert_eq!(c.net_lines, 3);
//! assert_eq!(c.timespecs, 1);
//! ```

const KEYWORDS: &[&str] = &[
    "NET",
    "PIN",
    "INST",
    "TIMESPEC",
    "TIMEGRP",
    "AREA_GROUP",
    "OFFSET",
    "FROM",
    "TO",
    "PERIOD",
];

fn is_comment(t: &str) -> bool {
    t.starts_with('#')
}

fn constraint_line(t: &str) -> bool {
    let head = t.split_whitespace().next().unwrap_or("");
    KEYWORDS.contains(&head) && t.contains('=')
}

/// `b` が .ucf に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut nets = 0usize;
    let mut total = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || is_comment(tr) {
            continue;
        }
        if constraint_line(tr) {
            total += 1;
            if tr.starts_with("NET ") || tr == "NET" {
                nets += 1;
            }
        }
    }
    (nets >= 1 && total >= 2) || total >= 4
}

/// .ucf の統計。
#[derive(Debug, Default, Clone)]
pub struct Ucf {
    /// `NET` 制約行数。
    pub net_lines: usize,
    /// `TIMESPEC` 行数。
    pub timespecs: usize,
    /// `TIMEGRP`/`AREA_GROUP`/`INST`/`PIN`/`OFFSET` 行数。
    pub other_constraints: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を .ucf として統計する。
pub fn parse(b: &[u8]) -> Ucf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = Ucf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if is_comment(tr) {
            c.comments += 1;
            continue;
        }
        if !constraint_line(tr) {
            continue;
        }
        if tr.starts_with("NET ") || tr == "NET" {
            c.net_lines += 1;
        } else if tr.starts_with("TIMESPEC") {
            c.timespecs += 1;
        } else {
            c.other_constraints += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"NET "clk" LOC = "E3";
NET "rst" LOC = "B8";
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.net_lines, 2);
    }

    #[test]
    fn detects_mixed() {
        let b = br#"NET "clk" LOC = "E3" | IOSTANDARD = "LVCMOS33";
TIMESPEC "TS_clk" = PERIOD "clk" 10 ns HIGH 50%;
TIMEGRP "fast" = "clk";
PIN "u1/clk" LOC = "E3";
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.timespecs, 1);
        assert_eq!(c.other_constraints, 2);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"NET x = 1\n"));
        assert!(!detect(b"foo = 1\nbar = 2\nbaz = 3\nqux = 4\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.net_lines, 0);
    }
}
