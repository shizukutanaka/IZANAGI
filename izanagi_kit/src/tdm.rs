//! CCSDS Tracking Data Message (TDM) scanner — KVN text form.
//!
//! A TDM is a `KEY = value` text file starting with
//! `CCSDS_TDM_VERS = <version>` (optionally preceded by `CREATION_DATE`),
//! with `META_START`/`META_STOP` metadata and `DATA_START`/`DATA_STOP`
//! blocks of `KEY = epoch value` observation lines (`DOPPLER_COUNT`,
//! `RANGE`, `RECEIVE_FREQ`, …).
//!
//! ```
//! let d = b"CCSDS_TDM_VERS = 1\x2e0\nMETA_START\nTRACK_ID = 1\nMETA_STOP\nDATA_START\nRANGE = 2026-01-01T00:00:00 1000\nDATA_STOP\n";
//! let t = izanagi_kit::tdm::parse(d).unwrap();
//! assert_eq!(t.version.as_deref(), Some("1\x2e0"));
//! assert_eq!(t.data_lines, 1);
//! ```
//!
//! Reference: CCSDS 503.0-B Tracking Data Message — mandatory
//! `CCSDS_TDM_VERS`, META/DATA block structure, and the
//! `keyword = epoch measurement` observation line shape.

/// Parsed TDM fields.
#[derive(Debug, Clone, PartialEq)]
pub struct Tdm {
    /// `CCSDS_TDM_VERS` value.
    pub version: Option<String>,
    /// `CREATION_DATE`.
    pub creation_date: Option<String>,
    /// `PARTICIPANT_1` … — the originating agency/ground station.
    pub participant: Option<String>,
    /// `TRACK_ID`/`STEC`/`MODE` — first `TRACK_ID` seen.
    pub track_id: Option<i64>,
    /// `META_START`/`META_STOP` block count.
    pub meta_blocks: usize,
    /// `DATA_START`/`DATA_STOP` block count.
    pub data_blocks: usize,
    /// Observation lines inside DATA blocks.
    pub data_lines: usize,
    /// `COMMENT` line count.
    pub comments: usize,
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

/// Parse a TDM file; `None` unless `CCSDS_TDM_VERS` is present.
pub fn parse(d: &[u8]) -> Option<Tdm> {
    let text = core::str::from_utf8(d).ok()?;
    let version = get(text, "CCSDS_TDM_VERS")?.to_string();
    let mut data_lines = 0usize;
    let mut comments = 0usize;
    let mut in_data = false;
    for l in text.lines() {
        let l = l.trim();
        if l.starts_with("COMMENT") {
            comments += 1;
        }
        if l.starts_with("DATA_START") {
            in_data = true;
            continue;
        }
        if l.starts_with("DATA_STOP") {
            in_data = false;
            continue;
        }
        if in_data && l.contains('=') {
            data_lines += 1;
        }
    }
    Some(Tdm {
        version: Some(version),
        creation_date: get(text, "CREATION_DATE").map(|s| s.to_string()),
        participant: get(text, "PARTICIPANT_1").map(|s| s.to_string()),
        track_id: get(text, "TRACK_ID").and_then(|s| s.parse().ok()),
        meta_blocks: text.matches("META_START").count(),
        data_blocks: text.matches("DATA_START").count(),
        data_lines,
        comments,
    })
}

/// `true` if the buffer looks like a CCSDS TDM.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b"CREATION_DATE = 2026-01-01T00:00:00\nCCSDS_TDM_VERS = 1\x2e0\nCOMMENT hi\nMETA_START\nTRACK_ID = 7\nPARTICIPANT_1 = GS\nMETA_STOP\nDATA_START\nRANGE = 2026-01-01T00:00:00 1000\nDOPPLER_COUNT = 2026-01-01T00:00:01 5\nDATA_STOP\n";

    #[test]
    fn parses() {
        let t = parse(DOC).unwrap();
        assert_eq!(t.version.as_deref(), Some("1\x2e0"));
        assert_eq!(t.creation_date.as_deref(), Some("2026-01-01T00:00:00"));
        assert_eq!(t.participant.as_deref(), Some("GS"));
        assert_eq!(t.track_id, Some(7));
        assert_eq!(t.meta_blocks, 1);
        assert_eq!(t.data_blocks, 1);
        assert_eq!(t.data_lines, 2);
        assert_eq!(t.comments, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"TRACK_ID = 1\n").is_none());
        assert!(parse(&[0xff]).is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(DOC));
        assert!(!detect(b"CCSDS_TDM"));
    }
}
