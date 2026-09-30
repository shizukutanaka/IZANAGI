//! CVE JSON 5 record scanner (cve.org CVE Services format).
//!
//! A CVE 5.x record is a JSON object with `"dataType": "CVE_RECORD"`,
//! `"dataVersion": "5.x"`, `cveMetadata` (`cveId`, `state`,
//! `assignerOrgId`, …) and `containers` (`cna` / optional `adp`).
//!
//! ```
//! let d = b"{\"dataType\":\"CVE_RECORD\",\"dataVersion\":\"5\x2e0\",
//!   \"cveMetadata\":{\"cveId\":\"CVE-2026-1234\",\"state\":\"PUBLISHED\"},
//!   \"containers\":{\"cna\":{\"descriptions\":[{\"lang\":\"en\",\"value\":\"x\"}]}}}";
//! let c = izanagi_kit::cve::parse(d).unwrap();
//! assert_eq!(c.cve_id.as_deref(), Some("CVE-2026-1234"));
//! assert_eq!(c.state.as_deref(), Some("PUBLISHED"));
//! ```
//!
//! Reference: CVE Record Format schema (cve-services / CVEProject
//! cvelistV5 `CVE-*.json` records).

use crate::json::{self, Json};

/// Parsed CVE record fields.
#[derive(Debug, Clone, PartialEq)]
pub struct Cve {
    /// `dataVersion`, e.g. `"5\x2e0"`.
    pub data_version: String,
    /// `cveMetadata.cveId` (`CVE-YYYY-NNNN+`).
    pub cve_id: Option<String>,
    /// `cveMetadata.state` — `PUBLISHED` / `REJECTED` / `RESERVED`.
    pub state: Option<String>,
    /// `cveMetadata.assignerOrgId`.
    pub assigner: Option<String>,
    /// Number of `containers` keys (`cna`, `adp`, …).
    pub containers: usize,
    /// Total `descriptions` entries across containers.
    pub descriptions: usize,
    /// Total `references` entries across containers.
    pub references: usize,
    /// Total `affected` product entries across containers.
    pub affected: usize,
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

fn arr_len(m: &std::collections::BTreeMap<String, Json>, k: &str) -> usize {
    match m.get(k) {
        Some(Json::Arr(a)) => a.len(),
        _ => 0,
    }
}

/// Parse a CVE JSON 5 record; `None` for other JSON.
pub fn parse(d: &[u8]) -> Option<Cve> {
    let root = json::parse(d).ok()?;
    let m = obj(&root)?;
    if str_of(m, "dataType")? != "CVE_RECORD" {
        return None;
    }
    let data_version = str_of(m, "dataVersion")?;
    if !data_version.starts_with("5\x2e") {
        return None;
    }
    let meta = m.get("cveMetadata").and_then(obj)?;
    let mut containers = 0usize;
    let (mut descriptions, mut references, mut affected) = (0, 0, 0);
    if let Some(Json::Obj(cs)) = m.get("containers") {
        for c in cs.values() {
            // `adp` is an array of ADP containers; `cna` is an object.
            let list: &[Json] = match c {
                Json::Arr(a) => a.as_slice(),
                other => core::slice::from_ref(other),
            };
            for cm in list.iter().filter_map(obj) {
                containers += 1;
                descriptions += arr_len(cm, "descriptions");
                references += arr_len(cm, "references");
                affected += arr_len(cm, "affected");
            }
        }
    }
    Some(Cve {
        data_version: data_version.to_string(),
        cve_id: str_of(meta, "cveId").map(|s| s.to_string()),
        state: str_of(meta, "state").map(|s| s.to_string()),
        assigner: str_of(meta, "assignerOrgId").map(|s| s.to_string()),
        containers,
        descriptions,
        references,
        affected,
    })
}

/// `true` if the buffer looks like a CVE 5.x record.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b"{\"dataType\":\"CVE_RECORD\",\"dataVersion\":\"5\x2e0\",\"cveMetadata\":{\"cveId\":\"CVE-2026-1234\",\"state\":\"PUBLISHED\",\"assignerOrgId\":\"aaaa\"},\"containers\":{\"cna\":{\"descriptions\":[{\"lang\":\"en\",\"value\":\"x\"}],\"references\":[{},{}],\"affected\":[{\"vendor\":\"v\"}]},\"adp\":[{\"references\":[{}]}]}}";

    #[test]
    fn parses() {
        let c = parse(DOC).unwrap();
        assert_eq!(c.data_version, "5\x2e0");
        assert_eq!(c.cve_id.as_deref(), Some("CVE-2026-1234"));
        assert_eq!(c.state.as_deref(), Some("PUBLISHED"));
        assert_eq!(c.assigner.as_deref(), Some("aaaa"));
        assert_eq!(c.containers, 2);
        assert_eq!(c.descriptions, 1);
        assert_eq!(c.references, 3);
        assert_eq!(c.affected, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"{}").is_none());
        assert!(parse(b"{\"dataType\":\"CVE_RECORD\",\"dataVersion\":\"4\x2e0\"}").is_none());
        assert!(parse(b"{\"dataType\":\"CVE_RECORD\",\"dataVersion\":\"5\x2e0\"}").is_none()); // no cveMetadata
        assert!(parse(b"[1]").is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(DOC));
        assert!(!detect(b"{}"));
    }
}
