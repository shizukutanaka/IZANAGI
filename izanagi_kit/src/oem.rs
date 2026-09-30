//! CCSDS Orbit Ephemeris Message (OEM) scanner — KVN text form.
//!
//! An OEM is a `KEY = value` text file starting with
//! `CCSDS_OEM_VERS = <version>`, then `META_START`/`META_STOP`
//! header + ephemeris `DATA` blocks of state-vector lines
//! (`YYYY-MM-DDTHH:MM:SS… x y z vx vy vz`).
//!
//! ```
//! let d = b"CCSDS_OEM_VERS = 2\x2e0\nMETA_START\nOBJECT_NAME = SAT\nMETA_STOP\n2026-01-01T00:00:00 1 2 3 4 5 6\n";
//! let o = izanagi_kit::oem::parse(d).unwrap();
//! assert_eq!(o.version.as_deref(), Some("2\x2e0"));
//! assert_eq!(o.state_lines, 1);
//! ```
//!
//! Reference: CCSDS 502.0-B Orbit Data Messages (OEM section) —
//! `CCSDS_OEM_VERS` mandatory first keyword, META/DATA blocks, and
//! the 7-field `epoch x y z vx vy vz` state line shape.

/// Parsed OEM fields.
#[derive(Debug, Clone, PartialEq)]
pub struct Oem {
    /// `CCSDS_OEM_VERS` value.
    pub version: Option<String>,
    /// `OBJECT_NAME`.
    pub object_name: Option<String>,
    /// `OBJECT_ID`.
    pub object_id: Option<String>,
    /// `CENTER_NAME` (e.g. `EARTH`).
    pub center_name: Option<String>,
    /// `REF_FRAME` (e.g. `EME2000`, `ICRF`).
    pub ref_frame: Option<String>,
    /// `TIME_SYSTEM` (e.g. `UTC`, `TAI`).
    pub time_system: Option<String>,
    /// `START_TIME` / `STOP_TIME` of the ephemeris span.
    pub start_time: Option<String>,
    /// Stop time of the ephemeris span.
    pub stop_time: Option<String>,
    /// `META_START`/`META_STOP` block count.
    pub meta_blocks: usize,
    /// State-vector line count (7 numeric fields after an epoch).
    pub state_lines: usize,
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

fn looks_like_epoch(s: &str) -> bool {
    // CCSDS time code: YYYY-MM-DD[T ]HH:MM:SS(.sss) or DOY form
    let b = s.as_bytes();
    b.len() >= 17
        && b[..4].iter().all(|c| c.is_ascii_digit())
        && b[4] == b'-'
        && b[7] == b'-'
        && (b[10] == b'T' || b[10] == b' ')
        && b[13] == b':'
        && b[16] == b':'
}

/// Parse an OEM file; `None` unless `CCSDS_OEM_VERS` is present.
pub fn parse(d: &[u8]) -> Option<Oem> {
    let text = core::str::from_utf8(d).ok()?;
    let version = get(text, "CCSDS_OEM_VERS")?.to_string();
    let mut state_lines = 0usize;
    let mut comments = 0usize;
    for l in text.lines() {
        let l = l.trim();
        if l.starts_with("COMMENT") {
            comments += 1;
            continue;
        }
        if l.contains('=') || l.is_empty() {
            continue;
        }
        if looks_like_epoch(l) {
            let fields = l.split_whitespace().count();
            if fields >= 7 {
                state_lines += 1;
            }
        }
    }
    Some(Oem {
        version: Some(version),
        object_name: get(text, "OBJECT_NAME").map(|s| s.to_string()),
        object_id: get(text, "OBJECT_ID").map(|s| s.to_string()),
        center_name: get(text, "CENTER_NAME").map(|s| s.to_string()),
        ref_frame: get(text, "REF_FRAME").map(|s| s.to_string()),
        time_system: get(text, "TIME_SYSTEM").map(|s| s.to_string()),
        start_time: get(text, "START_TIME").map(|s| s.to_string()),
        stop_time: get(text, "STOP_TIME").map(|s| s.to_string()),
        meta_blocks: text.matches("META_START").count(),
        state_lines,
        comments,
    })
}

/// `true` if the buffer looks like a CCSDS OEM.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b"CCSDS_OEM_VERS = 2\x2e0\nCOMMENT built\nMETA_START\nOBJECT_NAME = SAT\nOBJECT_ID = 2026-001A\nCENTER_NAME = EARTH\nREF_FRAME = EME2000\nTIME_SYSTEM = UTC\nSTART_TIME = 2026-01-01T00:00:00\nSTOP_TIME = 2026-01-01T01:00:00\nMETA_STOP\n2026-01-01T00:00:00 1 2 3 4 5 6\n2026-01-01T00:30:00 1 2 3 4 5 6\n";

    #[test]
    fn parses() {
        let o = parse(DOC).unwrap();
        assert_eq!(o.version.as_deref(), Some("2\x2e0"));
        assert_eq!(o.object_name.as_deref(), Some("SAT"));
        assert_eq!(o.ref_frame.as_deref(), Some("EME2000"));
        assert_eq!(o.time_system.as_deref(), Some("UTC"));
        assert_eq!(o.meta_blocks, 1);
        assert_eq!(o.state_lines, 2);
        assert_eq!(o.comments, 1);
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
        assert!(!detect(b"CCSDS_OEM"));
    }
}
