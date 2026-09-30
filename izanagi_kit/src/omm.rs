//! CCSDS Orbit Mean-Elements Message (OMM) scanner — KVN text form.
//!
//! An OMM is a `KEY = value` text file whose first statement is
//! `CCSDS_OMM_VERS = <version>`, followed by `META_START`/`META_STOP`
//! and `DATA` blocks of TLE-style element keywords (`OBJECT_NAME`,
//! `OBJECT_ID`, `CENTER_NAME`, `MEAN_ELEMENT_THEORY`, `EPOCH`, …).
//!
//! ```
//! let d = b"CCSDS_OMM_VERS = 2\x2e0\nCOMMENT x\nMETA_START\nOBJECT_NAME = SAT\nMETA_STOP\nEPOCH = 2026-01-01T00:00:00\n";
//! let o = izanagi_kit::omm::parse(d).unwrap();
//! assert_eq!(o.version.as_deref(), Some("2\x2e0"));
//! assert_eq!(o.object_name.as_deref(), Some("SAT"));
//! ```
//!
//! Reference: CCSDS 502.0-B Orbit Data Messages (OMM section) —
//! `CCSDS_OMM_VERS` mandatory first keyword and the `META_START`/
//! `META_STOP`/`DATA` block structure.

/// Parsed OMM fields.
#[derive(Debug, Clone, PartialEq)]
pub struct Omm {
    /// `CCSDS_OMM_VERS` value.
    pub version: Option<String>,
    /// `OBJECT_NAME`.
    pub object_name: Option<String>,
    /// `OBJECT_ID` (international designator).
    pub object_id: Option<String>,
    /// `CENTER_NAME` (e.g. `EARTH`).
    pub center_name: Option<String>,
    /// `MEAN_ELEMENT_THEORY` (e.g. `SGP4`).
    pub theory: Option<String>,
    /// `EPOCH` of the state.
    pub epoch: Option<String>,
    /// `META_START`/`META_STOP` block presence.
    pub meta_blocks: usize,
    /// `COMMENT` line count.
    pub comments: usize,
    /// Total `KEY = value` statements.
    pub entries: usize,
}

fn get<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.lines().find_map(|l| {
        l.trim()
            .strip_prefix(key)?
            .trim_start()
            .strip_prefix('=')
            .map(str::trim)
    })
}

/// Parse an OMM file; `None` unless `CCSDS_OMM_VERS` is present.
pub fn parse(d: &[u8]) -> Option<Omm> {
    let text = core::str::from_utf8(d).ok()?;
    let version = get(text, "CCSDS_OMM_VERS")?.to_string();
    let comments = text
        .lines()
        .filter(|l| l.trim_start().starts_with("COMMENT"))
        .count();
    let entries = text
        .lines()
        .filter(|l| {
            let l = l.trim();
            l.contains('=') && !l.starts_with("COMMENT") && !l.starts_with('%')
        })
        .count();
    Some(Omm {
        version: Some(version),
        object_name: get(text, "OBJECT_NAME").map(|s| s.to_string()),
        object_id: get(text, "OBJECT_ID").map(|s| s.to_string()),
        center_name: get(text, "CENTER_NAME").map(|s| s.to_string()),
        theory: get(text, "MEAN_ELEMENT_THEORY").map(|s| s.to_string()),
        epoch: get(text, "EPOCH").map(|s| s.to_string()),
        meta_blocks: text.matches("META_START").count(),
        comments,
        entries,
    })
}

/// `true` if the buffer looks like a CCSDS OMM.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b"CCSDS_OMM_VERS = 2\x2e0\nCOMMENT x\nMETA_START\nOBJECT_NAME = SAT\nOBJECT_ID = 2026-001A\nCENTER_NAME = EARTH\nMETA_STOP\nEPOCH = 2026-01-01T00:00:00\nMEAN_ELEMENT_THEORY = SGP4\n";

    #[test]
    fn parses() {
        let o = parse(DOC).unwrap();
        assert_eq!(o.version.as_deref(), Some("2\x2e0"));
        assert_eq!(o.object_name.as_deref(), Some("SAT"));
        assert_eq!(o.object_id.as_deref(), Some("2026-001A"));
        assert_eq!(o.center_name.as_deref(), Some("EARTH"));
        assert_eq!(o.theory.as_deref(), Some("SGP4"));
        assert_eq!(o.epoch.as_deref(), Some("2026-01-01T00:00:00"));
        assert_eq!(o.meta_blocks, 1);
        assert_eq!(o.comments, 1);
        assert_eq!(o.entries, 6);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"OBJECT_NAME = SAT\n").is_none());
        assert!(parse(&[0xff]).is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(DOC));
        assert!(!detect(b"CCSDS_OMM"));
    }
}
