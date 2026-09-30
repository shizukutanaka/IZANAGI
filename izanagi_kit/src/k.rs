//! LS-DYNA keyword deck (`.k` / `.key`) — text input for the LS-DYNA
//! explicit solver. The first record is `*KEYWORD` (optionally
//! `*KEYWORD_ID`, `*KEYWORD_SECRET`, …), records are `*NODE`,
//! `*ELEMENT_SOLID`, `*PART`, `*SECTION_`, `*MAT_`, `*BOUNDARY_`,
//! `*CONTROL_`, comments start `$`, and the deck ends `*END`.
//!
//! `parse` requires a `*KEYWORD` first record and `*END`, counts
//! keyword records, and flags the common sections.
//!
//! ```
//! let f = b"*KEYWORD\n*NODE\n1 0.0 0.0 0.0\n*END\n";
//! let k = izanagi_kit::k::parse(f).unwrap();
//! assert_eq!(k.keywords, 3);
//! assert!(k.has_node);
//! assert!(k.has_end);
//! assert!(izanagi_kit::k::parse(b"*NODE\n*END\n").is_none());
//! ```

/// Parsed LS-DYNA keyword-deck summary.
#[derive(Debug, Clone, PartialEq)]
pub struct K {
    /// `*KEYWORD`-family records seen (including `*END`).
    pub keywords: usize,
    /// Deck terminated by `*END`.
    pub has_end: bool,
    /// `*NODE` section present.
    pub has_node: bool,
    /// `*ELEMENT_`/`ELEMENT` family section present.
    pub has_element: bool,
    /// `*PART` section present.
    pub has_part: bool,
    /// `*MAT_` family section present.
    pub has_mat: bool,
}

/// Parse a `.k` deck; `None` without `*KEYWORD` + `*END`.
pub fn parse(d: &[u8]) -> Option<K> {
    let s = std::str::from_utf8(d).ok()?;
    let mut lines = s.lines();
    let first = lines.find(|l| !l.trim().is_empty() && !l.trim_start().starts_with('$'))?;
    if !first.trim_start().to_uppercase().starts_with("*KEYWORD") {
        return None;
    }
    let mut keywords = 1usize;
    let mut has_end = false;
    let mut has_node = false;
    let mut has_element = false;
    let mut has_part = false;
    let mut has_mat = false;
    for line in lines {
        let l = line.trim_start();
        if !l.starts_with('*') {
            continue;
        }
        keywords += 1;
        let up = l.to_uppercase();
        if up.starts_with("*END") {
            has_end = true;
        }
        if up.starts_with("*NODE") {
            has_node = true;
        }
        if up.starts_with("*ELEMENT") {
            has_element = true;
        }
        if up.starts_with("*PART") {
            has_part = true;
        }
        if up.starts_with("*MAT") {
            has_mat = true;
        }
    }
    if !has_end {
        return None;
    }
    Some(K {
        keywords,
        has_end,
        has_node,
        has_element,
        has_part,
        has_mat,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let f = b"*KEYWORD_ID\n$ comment\n*NODE\n*ELEMENT_SOLID\n*PART\n*MAT_ELASTIC\n*END\n";
        let k = parse(f).unwrap();
        assert!(k.has_node && k.has_element && k.has_part && k.has_mat);
        assert_eq!(k.keywords, 6);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"*KEYWORD\n*NODE\n").is_none()); // no *END
        assert!(parse(b"$ only a comment\n*END\n").is_none());
    }
}
