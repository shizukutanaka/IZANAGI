//! `.qsf` (Quartus Settings File) 検出モジュール。
//!
//! Intel/Altera Quartus のプロジェクト設定は Tcl ライクな
//! `set_global_assignment -name <KEY> <value>`、
//! `set_instance_assignment -name <KEY> -to <node> <value>` 行で
//! 構成される。
//!
//! ```
//! let b = br#"set_global_assignment -name FAMILY "Cyclone V"
//! set_global_assignment -name DEVICE 5CGXFC7C7F23C8
//! set_global_assignment -name TOP_LEVEL_ENTITY top
//! set_global_assignment -name VERILOG_FILE top.v
//! set_instance_assignment -name IO_STANDARD "3.3-V LVTTL" -to clk
//! "#;
//! let c = izanagi_kit::qsf::parse(b);
//! assert!(izanagi_kit::qsf::detect(b));
//! assert_eq!(c.global_assignments, 4);
//! assert_eq!(c.instance_assignments, 1);
//! ```

fn is_comment(t: &str) -> bool {
    t.starts_with('#')
}

/// `b` が Quartus Settings File に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut globals = 0usize;
    let mut total = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || is_comment(tr) {
            continue;
        }
        if tr.starts_with("set_global_assignment") {
            globals += 1;
            total += 1;
        } else if tr.starts_with("set_instance_assignment")
            || tr.starts_with("set_location_assignment")
            || tr.starts_with("set_parameter")
            || tr.starts_with("set_assignment_group")
        {
            total += 1;
        }
    }
    globals >= 2 && total >= 3
}

/// .qsf の統計。
#[derive(Debug, Default, Clone)]
pub struct Qsf {
    /// `set_global_assignment` 行数。
    pub global_assignments: usize,
    /// `set_instance_assignment` 行数。
    pub instance_assignments: usize,
    /// `set_location_assignment` 行数。
    pub location_assignments: usize,
    /// その他の `set_*` Tcl 行数。
    pub other_tcl: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を .qsf として統計する。
pub fn parse(b: &[u8]) -> Qsf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = Qsf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if is_comment(tr) {
            c.comments += 1;
            continue;
        }
        if tr.starts_with("set_global_assignment") {
            c.global_assignments += 1;
        } else if tr.starts_with("set_instance_assignment") {
            c.instance_assignments += 1;
        } else if tr.starts_with("set_location_assignment") {
            c.location_assignments += 1;
        } else if tr.starts_with("set_") {
            c.other_tcl += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"set_global_assignment -name FAMILY "Cyclone V"
set_global_assignment -name DEVICE 5CGXFC7C7F23C8
set_global_assignment -name TOP_LEVEL_ENTITY top
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.global_assignments, 3);
    }

    #[test]
    fn detects_mixed() {
        let b = br#"set_global_assignment -name FAMILY "Cyclone V"
set_global_assignment -name DEVICE 5CGXFC7C7F23C8
set_instance_assignment -name IO_STANDARD "3.3-V LVTTL" -to clk
set_location_assignment PIN_A5 -to clk
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.instance_assignments, 1);
        assert_eq!(c.location_assignments, 1);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"set_global_assignment -name A 1\n"));
        assert!(!detect(b"set foo 1\nset bar 2\n"));
        assert!(!detect(b"proc x {} {}\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.global_assignments, 0);
    }
}
