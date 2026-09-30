//! CalculiX `.frd` result file — the fixed-format result listing
//! produced by `*NODE FILE` / `*EL FILE`. Records are column-oriented:
//! `    1<KEY>` block headers (`PSTEP`, `U`, `C`, `N`, …), `  -1` node
//! index lines, `  -2` element index lines, `  -3` value records,
//! and `9999` block terminators.
//!
//! `parse` requires at least one `    1`-style block header and
//! reports node/value record counts plus the block keys seen.
//!
//! ```
//! let f = b"    1 PSTEP\n    2 CL101\n  -1         1  0.0\n  -3\n9999\n";
//! let r = izanagi_kit::frd::parse(f).unwrap();
//! assert_eq!(r.blocks, 1);
//! assert_eq!(r.node_records, 1);
//! assert!(izanagi_kit::frd::parse(b"1 PSTEP\n").is_none());
//! ```

/// Parsed CalculiX result summary.
#[derive(Debug, Clone, PartialEq)]
pub struct Frd {
    /// `    1`-prefix block headers seen.
    pub blocks: usize,
    /// `  -1` node/index records seen.
    pub node_records: usize,
    /// `  -2` element records seen.
    pub elem_records: usize,
    /// `  -3` value records seen.
    pub value_records: usize,
    /// `9999` terminators seen.
    pub terminators: usize,
}

/// Parse a `.frd`; `None` without block headers.
pub fn parse(d: &[u8]) -> Option<Frd> {
    let s = std::str::from_utf8(d).ok()?;
    let mut r = Frd {
        blocks: 0,
        node_records: 0,
        elem_records: 0,
        value_records: 0,
        terminators: 0,
    };
    for line in s.lines() {
        if line.starts_with("    1") {
            r.blocks += 1;
        } else if line.starts_with("  -1") {
            r.node_records += 1;
        } else if line.starts_with("  -2") {
            r.elem_records += 1;
        } else if line.starts_with("  -3") {
            r.value_records += 1;
        } else if line.trim() == "9999" {
            r.terminators += 1;
        }
    }
    (r.blocks > 0).then_some(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let f = b"    1 PSTEP\n    2 CL\n  -1         1\n  -2         1\n  -3\n9999\n";
        let r = parse(f).unwrap();
        assert_eq!(r.blocks, 1);
        assert_eq!(r.node_records, 1);
        assert_eq!(r.elem_records, 1);
        assert_eq!(r.value_records, 1);
        assert_eq!(r.terminators, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"  -1  1\n").is_none());
    }
}
