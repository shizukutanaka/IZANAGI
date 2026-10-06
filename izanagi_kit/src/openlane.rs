//! OpenLane 設定ファイル検出モジュール。
//!
//! OpenLane (ASIC フロー) の `config.tcl`/`config.json` は
//! `set ::env(VAR) value` 形式の Tcl 代入が主体。
//!
//! ```
//! let b = br#"set ::env(DESIGN_NAME) "spm"
//! set ::env(VERILOG_FILES) "dir::src/spm.v"
//! set ::env(CLOCK_PORT) "clk"
//! set ::env(CLOCK_PERIOD) "10.000"
//! set ::env(FP_CORE_UTIL) "50"
//! "#;
//! let c = izanagi_kit::openlane::parse(b);
//! assert!(izanagi_kit::openlane::detect(b));
//! assert_eq!(c.env_assignments, 5);
//! ```

const ENV_PREFIXES: &[&str] = &[
    "CLOCK_",
    "CORE_",
    "DESIGN_",
    "DIODE_",
    "DPL_",
    "EXTRA_",
    "FILL_",
    "FP_",
    "GDS_",
    "GLB_",
    "GND_",
    "IO_",
    "KLAYOUT_",
    "LEC_",
    "LIB_",
    "LVS_",
    "MAGIC_",
    "MAX_",
    "MERGED_",
    "NETLIST_",
    "PDK",
    "PLACE_",
    "PL_",
    "PNR_",
    "POWER_",
    "RCX_",
    "ROUTING_",
    "RSZ_",
    "SAVE_",
    "SCL_",
    "SPEF_",
    "STD_CELL_",
    "SYNTH_",
    "TECH_",
    "TRACKS_",
    "VDD_",
    "VERILOG_",
    "WELL_",
];

fn is_comment(t: &str) -> bool {
    t.starts_with('#')
}

fn env_var(t: &str) -> Option<&str> {
    let rest = t.strip_prefix("set ::env(")?;
    let end = rest.find(')')?;
    Some(&rest[..end])
}

/// `b` が OpenLane 設定に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut envs = 0usize;
    let mut known = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || is_comment(tr) {
            continue;
        }
        if let Some(v) = env_var(tr) {
            envs += 1;
            if ENV_PREFIXES.iter().any(|p| v.starts_with(p)) {
                known += 1;
            }
        }
    }
    known >= 2 || envs >= 4
}

/// OpenLane 設定の統計。
#[derive(Debug, Default, Clone)]
pub struct OpenlaneConf {
    /// `set ::env(...)` 行数。
    pub env_assignments: usize,
    /// 既知プレフィックスの env 代入数。
    pub known_envs: usize,
    /// その他の `set` 行数。
    pub other_sets: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を OpenLane 設定として統計する。
pub fn parse(b: &[u8]) -> OpenlaneConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = OpenlaneConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if is_comment(tr) {
            c.comments += 1;
            continue;
        }
        if let Some(v) = env_var(tr) {
            c.env_assignments += 1;
            if ENV_PREFIXES.iter().any(|p| v.starts_with(p)) {
                c.known_envs += 1;
            }
        } else if tr.starts_with("set ") {
            c.other_sets += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"set ::env(DESIGN_NAME) "spm"
set ::env(CLOCK_PORT) "clk"
set ::env(CLOCK_PERIOD) "10.000"
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.env_assignments, 3);
        assert_eq!(c.known_envs, 3);
    }

    #[test]
    fn detects_generic_envs() {
        let b = br#"set ::env(FOO) 1
set ::env(BAR) 2
set ::env(BAZ) 3
set ::env(QUX) 4
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.env_assignments, 4);
        assert_eq!(c.known_envs, 0);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"set ::env(DESIGN_NAME) \"x\"\n"));
        assert!(!detect(b"set foo 1\nset bar 2\n"));
        assert!(!detect(b"env(VAR) = 1\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.env_assignments, 0);
    }
}
