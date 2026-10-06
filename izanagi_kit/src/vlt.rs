//! `.vlt` (Verilator lint configuration) 検出モジュール。
//!
//! Verilator の lint 設定は `` `verilator_config `` ヘッダと
//! `lint_off`/`lint_on`/`coverage_*`/`profile_*` ディレクティブで
//! 構成される。
//!
//! ```
//! let b = br#"`verilator_config
//! lint_off -rule WIDTH -file "top.v"
//! lint_off -rule UNUSED
//! lint_on -rule DECLFILENAME
//! coverage_off -file "tb.v"
//! "#;
//! let c = izanagi_kit::vlt::parse(b);
//! assert!(izanagi_kit::vlt::detect(b));
//! assert!(c.has_header);
//! assert_eq!(c.directives, 4);
//! ```

fn is_comment(t: &str) -> bool {
    t.starts_with("//") || t.starts_with("/*") || t.starts_with('*')
}

fn directive_hit(t: &str) -> bool {
    t.starts_with("lint_off")
        || t.starts_with("lint_on")
        || t.starts_with("coverage_off")
        || t.starts_with("coverage_on")
        || t.starts_with("profile_off")
        || t.starts_with("profile_on")
        || t.starts_with("hier_block")
        || t.starts_with("public_flat")
        || t.starts_with("sc_bv")
        || t.starts_with("unroll")
}

/// `b` が Verilator lint 設定に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut header = false;
    let mut dirs = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || is_comment(tr) {
            continue;
        }
        if tr == "`verilator_config" || tr.starts_with("`verilator_config ") {
            header = true;
            continue;
        }
        if directive_hit(tr) {
            dirs += 1;
        }
    }
    header && dirs >= 1
}

/// .vlt の統計。
#[derive(Debug, Default, Clone)]
pub struct Vlt {
    /// `` `verilator_config `` 行があったか。
    pub has_header: bool,
    /// lint_off/lint_on/coverage_* 等ディレクティブ行数。
    pub directives: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を .vlt として統計する。
pub fn parse(b: &[u8]) -> Vlt {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = Vlt::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if is_comment(tr) {
            c.comments += 1;
            continue;
        }
        if tr == "`verilator_config" || tr.starts_with("`verilator_config ") {
            c.has_header = true;
            continue;
        }
        if directive_hit(tr) {
            c.directives += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"`verilator_config
lint_off -rule WIDTH -file "top.v"
"#;
        assert!(detect(b));
        let c = parse(b);
        assert!(c.has_header);
        assert_eq!(c.directives, 1);
    }

    #[test]
    fn detects_full() {
        let b = br#"`verilator_config
lint_off -rule WIDTH
lint_on -rule DECLFILENAME
coverage_off -file "tb.v"
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.directives, 3);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"lint_off -rule WIDTH\nlint_off -rule UNUSED\n"));
        assert!(!detect(b"`verilator_config\n"));
        assert!(!detect(b"module x; endmodule\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert!(!c.has_header);
    }
}
