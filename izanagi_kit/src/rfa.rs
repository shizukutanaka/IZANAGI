//! Autodesk Revit `.rfa` family files — an OLE2 compound document
//! whose directory contains Revit-specific storages/streams such as
//! `Family`, `PartAtom`, `FamilyTypeInformation` and `Form4`.
//!
//! `parse` reuses `crate::ole`, then requires at least one
//! Revit-flavoured entry name; plain CFB files (MSI, .doc, …) are
//! rejected.
//!
//! ```
//! assert!(izanagi_kit::rfa::parse(b"not a compound file").is_none());
//! assert!(izanagi_kit::rfa::marks().contains(&"Family"));
//! ```

/// Parsed Revit family summary.
#[derive(Debug, Clone, PartialEq)]
pub struct Rfa {
    /// OLE directory entries found.
    pub entries: usize,
    /// True when a `Family`-named stream/storage exists.
    pub has_family: bool,
    /// True when a `PartAtom` storage exists (geometry).
    pub has_partatom: bool,
}

const MARKS: &[&str] = &[
    "Family",
    "PartAtom",
    "FamilyTypeInformation",
    "Form4",
    "Global",
];

/// Parse an `.rfa`; `None` for non-CFB input or CFB without Revit
/// marks.
pub fn parse(d: &[u8]) -> Option<Rfa> {
    let ole = crate::ole::parse(d)?;
    let mut has_family = false;
    let mut has_partatom = false;
    for e in &ole.entries {
        let n = &e.name;
        if n.contains("Family") {
            has_family = true;
        }
        if n.contains("PartAtom") {
            has_partatom = true;
        }
    }
    if !(has_family || has_partatom) {
        return None;
    }
    Some(Rfa {
        entries: ole.entries.len(),
        has_family,
        has_partatom,
    })
}

/// Marker table (also exercised by tests).
pub fn marks() -> &'static [&'static str] {
    MARKS
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn marks_table() {
        assert!(marks().contains(&"Family"));
        assert!(marks().contains(&"PartAtom"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"not a cfb").is_none());
    }
}
