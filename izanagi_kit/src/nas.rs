//! NASTRAN bulk data file (`.bdf` / `.nas` / `.dat`): `$`-comment
//! header lines, `BEGIN BULK`, then card lines (GRID, CQUAD4, …)
//! until `ENDDATA`. Continuation cards and executive sections are
//! counted, not interpreted.
//!
//! ```
//! let d = b"$ comment\nBEGIN BULK\nGRID,1,,0.,0.,0.\nENDDATA\n";
//! let n = izanagi_kit::nas::parse(d).unwrap();
//! assert!(n.bulk_seen);
//! assert_eq!(n.card_count, 1);
//! ```

/// A parsed NASTRAN deck.
#[derive(Clone, Debug)]
pub struct Nas {
    /// `BEGIN BULK` present.
    pub bulk_seen: bool,
    /// `ENDDATA` present.
    pub enddata_seen: bool,
    /// Data-card count inside the bulk section (excluding `$`
    /// comments and continuation-only accounting is left to callers).
    pub card_count: usize,
    /// Distinct card names seen (uppercase, first field).
    pub card_names: std::vec::Vec<std::string::String>,
}

/// Parse a NASTRAN deck; `None` without `BEGIN BULK`.
pub fn parse(d: &[u8]) -> Option<Nas> {
    let s = std::str::from_utf8(d).ok()?;
    let mut bulk_seen = false;
    let mut enddata_seen = false;
    let mut card_count = 0usize;
    let mut in_bulk = false;
    let mut names = std::collections::BTreeSet::new();
    for line in s.lines() {
        let t = line.trim();
        let u = t.to_uppercase();
        if t.starts_with('$') {
            continue;
        }
        if u.starts_with("BEGIN BULK") || u.starts_with("BEGINBULK") {
            bulk_seen = true;
            in_bulk = true;
            continue;
        }
        if u.starts_with("ENDDATA") {
            enddata_seen = true;
            in_bulk = false;
            continue;
        }
        if in_bulk && !t.is_empty() {
            card_count += 1;
            let name = t.split(|c: char| c == ',' || c.is_whitespace()).next()?;
            if !name.is_empty() {
                names.insert(name.trim_end_matches('*').to_uppercase());
            }
        }
    }
    if !bulk_seen {
        return None;
    }
    Some(Nas {
        bulk_seen,
        enddata_seen,
        card_count,
        card_names: names.into_iter().collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DECK: &[u8] = b"$ header\nSOL 101\nBEGIN BULK\nGRID,1,,0.,0.,0.\nGRID* this is not split\nCQUAD4,1,1,1,2,3,4\nENDDATA\n";

    #[test]
    fn basic() {
        let n = parse(DECK).unwrap();
        assert!(n.bulk_seen);
        assert!(n.enddata_seen);
        assert_eq!(n.card_count, 3);
        assert!(n.card_names.iter().any(|c| c == "GRID"));
        assert!(n.card_names.iter().any(|c| c == "CQUAD4"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"GRID,1\n").is_none()); // no BEGIN BULK
        assert_eq!(parse(b"BEGIN BULK\n").unwrap().card_count, 0);
    }
}
