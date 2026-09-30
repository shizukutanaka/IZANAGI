//! Gambit Neutral File (`.neu`): a text mesh format whose prolog
//! contains `CONTROL INFO` / `GAMBIT NEUTRAL FILE`, followed by
//! `NUMNP NELEM NGRPS NBSETS NDFCD NDFVL` summary numbers and blocks
//! ending in `ENDOFSECTION`.
//!
//! ```
//! let d = b"        CONTROL INFO\n**      GAMBIT NEUTRAL FILE\n10 4 1 1 2 2\n        ENDOFSECTION\n";
//! let n = izanagi_kit::neu::parse(d).unwrap();
//! assert_eq!(n.numnp, Some(10));
//! assert_eq!(n.nelem, Some(4));
//! ```

use std::vec::Vec;

/// A parsed Gambit neutral file.
#[derive(Clone, Debug)]
pub struct Neu {
    /// `NUMNP` — node count from the summary line.
    pub numnp: Option<u64>,
    /// `NELEM` — element count.
    pub nelem: Option<u64>,
    /// `NGRPS` — group count.
    pub ngrps: Option<u64>,
    /// `NBSETS` — boundary-set count.
    pub nbsets: Option<u64>,
    /// Number of `ENDOFSECTION` blocks.
    pub sections: usize,
    /// `PROGRAM` name when present.
    pub program: Option<std::string::String>,
}

/// Parse a `.neu`; `None` unless the prolog mentions `GAMBIT NEUTRAL
/// FILE` or `CONTROL INFO` and at least one `ENDOFSECTION` exists.
pub fn parse(d: &[u8]) -> Option<Neu> {
    let s = std::str::from_utf8(d).ok()?;
    let upper = s.to_uppercase();
    if !upper.contains("GAMBIT NEUTRAL FILE") && !upper.contains("CONTROL INFO") {
        return None;
    }
    let mut sections = 0usize;
    let mut program = None;
    let mut summary: Option<Vec<u64>> = None;
    for line in s.lines() {
        let t = line.trim();
        if t.eq_ignore_ascii_case("endofsection") {
            sections += 1;
            continue;
        }
        let u = t.to_uppercase();
        if u.starts_with("PROGRAM:") || u.starts_with("PROGRAM =") {
            let v = t.split(&[':', '='][..]).nth(1).unwrap_or("").trim();
            if !v.is_empty() {
                program = Some(v.to_string());
            }
            continue;
        }
        // The summary line is six whitespace-separated integers.
        if summary.is_none() {
            let vals: Vec<u64> = t
                .split_whitespace()
                .filter_map(|w| w.parse().ok())
                .collect();
            if vals.len() == 6 && t.split_whitespace().count() == 6 {
                summary = Some(vals);
            }
        }
    }
    if sections == 0 {
        return None;
    }
    let v = summary.unwrap_or_default();
    let at = |i: usize| v.get(i).copied();
    Some(Neu {
        numnp: at(0),
        nelem: at(1),
        ngrps: at(2),
        nbsets: at(3),
        sections,
        program,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const NEU: &[u8] = b"        CONTROL INFO\n**      GAMBIT NEUTRAL FILE\nPROGRAM:                Gambit\n10 4 1 1 2 2\n   NODAL COORDINATES\n     1 0 0 0\n        ENDOFSECTION\n        ELEMENTS/CELLS\n        ENDOFSECTION\n";

    #[test]
    fn basic() {
        let n = parse(NEU).unwrap();
        assert_eq!(n.numnp, Some(10));
        assert_eq!(n.nelem, Some(4));
        assert_eq!(n.ngrps, Some(1));
        assert_eq!(n.nbsets, Some(1));
        assert_eq!(n.sections, 2);
        assert_eq!(n.program.as_deref(), Some("Gambit"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"1 2 3 4 5 6\nENDOFSECTION\n").is_none()); // no header marker
        assert!(parse(b"GAMBIT NEUTRAL FILE\nno sections\n").is_none());
    }
}
