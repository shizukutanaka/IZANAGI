//! CSAF — Common Security Advisory Framework (OASIS CSAF 2.0):
//! a JSON document whose `document.category` starts with `csaf_`
//! (`csaf_base`, `csaf_security_advisory`, `csaf_vex`, …) and whose
//! `document.tracking.id` is required.
//!
//! ```
//! let d = izanagi_kit::csaf::parse(br#"{"document":{"category":"csaf_vex",
//!   "tracking":{"id":"2024-001","version":"1"}},
//!   "vulnerabilities":[{"cve":"CVE-1"},{"cve":"CVE-2"}]}"#).unwrap();
//! assert_eq!(d.category, "csaf_vex");
//! assert_eq!(d.vulnerabilities, 2);
//! ```

use crate::json::{self, Json};
use std::string::{String, ToString};

/// A parsed CSAF document.
#[derive(Clone, Debug)]
pub struct Csaf {
    /// `document.category` (a `csaf_*` value).
    pub category: String,
    /// `document.tracking.id`.
    pub tracking_id: String,
    /// `document.title` when present.
    pub title: Option<String>,
    /// `document.tracking.version` when present.
    pub track_version: Option<String>,
    /// Count of `vulnerabilities[]` entries.
    pub vulnerabilities: usize,
    /// Count of `product_tree.branches[]` roots (0 when absent).
    pub product_branches: usize,
    /// Count of `notes[]` entries.
    pub notes: usize,
}

fn s(v: &Json) -> Option<&str> {
    match v {
        Json::Str(t) => Some(t.as_str()),
        _ => None,
    }
}
fn obj(v: &Json) -> Option<&std::collections::BTreeMap<String, Json>> {
    match v {
        Json::Obj(m) => Some(m),
        _ => None,
    }
}
fn arr(v: &Json) -> Option<&[Json]> {
    match v {
        Json::Arr(a) => Some(a.as_slice()),
        _ => None,
    }
}

/// Parse a CSAF document; `None` when `document.category` is missing
/// or not a `csaf_*` value, or `document.tracking.id` is absent.
pub fn parse(d: &[u8]) -> Option<Csaf> {
    let v = json::parse(d).ok()?;
    let m = obj(&v)?;
    let doc = m.get("document").and_then(obj)?;
    let category = s(doc.get("category")?)?;
    if !category.starts_with("csaf_") {
        return None;
    }
    let tracking = doc.get("tracking").and_then(obj)?;
    let tracking_id = s(tracking.get("id")?)?.to_string();
    Some(Csaf {
        category: category.to_string(),
        tracking_id,
        title: doc.get("title").and_then(s).map(ToString::to_string),
        track_version: tracking.get("version").and_then(s).map(ToString::to_string),
        vulnerabilities: m
            .get("vulnerabilities")
            .and_then(arr)
            .map_or(0, |a| a.len()),
        product_branches: m
            .get("product_tree")
            .and_then(obj)
            .and_then(|t| t.get("branches"))
            .and_then(arr)
            .map_or(0, |a| a.len()),
        notes: doc.get("notes").and_then(arr).map_or(0, |a| a.len()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vex() {
        let d = parse(
            br#"{"document":{"category":"csaf_vex","title":"T",
            "tracking":{"id":"ID-1","version":"2"},"notes":[{"x":1}]},
            "product_tree":{"branches":[{},{},{}]},
            "vulnerabilities":[{"cve":"CVE-1"}]}"#,
        )
        .unwrap();
        assert_eq!(d.tracking_id, "ID-1");
        assert_eq!(d.title.as_deref(), Some("T"));
        assert_eq!(d.track_version.as_deref(), Some("2"));
        assert_eq!(d.product_branches, 3);
        assert_eq!(d.notes, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"{}").is_none());
        // category must be csaf_*
        assert!(parse(
            br#"{"document":{"category":"other",
            "tracking":{"id":"x"}}}"#
        )
        .is_none());
        // tracking.id required
        assert!(parse(
            br#"{"document":{"category":"csaf_base",
            "tracking":{}}}"#
        )
        .is_none());
        // document object required
        assert!(parse(br#"{"category":"csaf_vex"}"#).is_none());
    }
}
