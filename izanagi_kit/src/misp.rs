//! MISP event JSON export scanner.
//!
//! A MISP export is a JSON object `"Event": { … }` (or a bare event
//! object) carrying `uuid`, `info`, `date`, `orgc`/`Orgc`,
//! `Attribute[]` and `Object[]` collections.
//!
//! ```
//! let d = br#"{"Event":{"uuid":"11111111-2222-3333-4444-555555555555",
//!   "info":"t","date":"2026-01-01","Attribute":[{},{}],"Object":[{}],
//!   "Orgc":{"name":"org"}}}"#;
//! let m = izanagi_kit::misp::parse(d).unwrap();
//! assert_eq!(m.info.as_deref(), Some("t"));
//! assert_eq!(m.attributes, 2);
//! assert_eq!(m.objects, 1);
//! ```
//!
//! Reference: MISP core format (misp-standard.org misp-core-format
//! JSON schema) — `Event` object with `uuid`/`info`/`date` required.

use crate::json::{self, Json};

/// Parsed MISP event fields.
#[derive(Debug, Clone, PartialEq)]
pub struct Misp {
    /// `Event.uuid`.
    pub uuid: Option<String>,
    /// `Event.info` free-text summary.
    pub info: Option<String>,
    /// `Event.date` (`YYYY-MM-DD`).
    pub date: Option<String>,
    /// `Event.threat_level_id` (1 = high … 4 = undefined).
    pub threat_level: Option<i64>,
    /// `Orgc.name` — creating organisation.
    pub orgc: Option<String>,
    /// `Attribute[]` count.
    pub attributes: usize,
    /// `Object[]` count.
    pub objects: usize,
    /// `Galaxy[]` count.
    pub galaxies: usize,
    /// `Tag[]` count.
    pub tags: usize,
}

fn obj(v: &Json) -> Option<&std::collections::BTreeMap<String, Json>> {
    match v {
        Json::Obj(m) => Some(m),
        _ => None,
    }
}

fn str_of<'a>(m: &'a std::collections::BTreeMap<String, Json>, k: &str) -> Option<&'a str> {
    match m.get(k) {
        Some(Json::Str(s)) => Some(s),
        _ => None,
    }
}

fn int_of(m: &std::collections::BTreeMap<String, Json>, k: &str) -> Option<i64> {
    match m.get(k) {
        Some(Json::Int(n)) => Some(*n),
        _ => None,
    }
}

fn arr_len(m: &std::collections::BTreeMap<String, Json>, k: &str) -> usize {
    match m.get(k) {
        Some(Json::Arr(a)) => a.len(),
        _ => 0,
    }
}

/// Parse a MISP event export; `None` for other JSON.
pub fn parse(d: &[u8]) -> Option<Misp> {
    let root = json::parse(d).ok()?;
    let top = obj(&root)?;
    // either {"Event": {...}} or a bare event object
    let ev = match top.get("Event") {
        Some(e) => obj(e)?,
        None => top,
    };
    if !ev.contains_key("uuid") && !ev.contains_key("Attribute") {
        return None;
    }
    if !ev.contains_key("info") && !ev.contains_key("date") && !ev.contains_key("uuid") {
        return None;
    }
    let orgc = ev
        .get("Orgc")
        .or_else(|| ev.get("orgc"))
        .and_then(obj)
        .and_then(|o| str_of(o, "name"))
        .map(|s| s.to_string());
    Some(Misp {
        uuid: str_of(ev, "uuid").map(|s| s.to_string()),
        info: str_of(ev, "info").map(|s| s.to_string()),
        date: str_of(ev, "date").map(|s| s.to_string()),
        threat_level: int_of(ev, "threat_level_id"),
        orgc,
        attributes: arr_len(ev, "Attribute"),
        objects: arr_len(ev, "Object"),
        galaxies: arr_len(ev, "Galaxy"),
        tags: arr_len(ev, "Tag"),
    })
}

/// `true` if the buffer looks like a MISP event export.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = br#"{"Event":{"uuid":"1111-2222","info":"t","date":"2026-01-01","threat_level_id":2,"Attribute":[{},{}],"Object":[{}],"Galaxy":[],"Tag":[{}],"Orgc":{"name":"org"}}}"#;

    #[test]
    fn parses() {
        let m = parse(DOC).unwrap();
        assert_eq!(m.uuid.as_deref(), Some("1111-2222"));
        assert_eq!(m.info.as_deref(), Some("t"));
        assert_eq!(m.date.as_deref(), Some("2026-01-01"));
        assert_eq!(m.threat_level, Some(2));
        assert_eq!(m.orgc.as_deref(), Some("org"));
        assert_eq!(m.attributes, 2);
        assert_eq!(m.objects, 1);
        assert_eq!(m.tags, 1);
    }

    #[test]
    fn bare_event() {
        let m = parse(br#"{"uuid":"u","info":"i","Attribute":[]}"#).unwrap();
        assert_eq!(m.uuid.as_deref(), Some("u"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"{}").is_none());
        assert!(parse(br#"{"Event":{"x":1}}"#).is_none());
        assert!(parse(b"[1]").is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(DOC));
        assert!(!detect(b"{}"));
    }
}
