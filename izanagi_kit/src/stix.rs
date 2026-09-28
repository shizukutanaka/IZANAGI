//! STIX 2.x bundle scanner (OASIS CTI, JSON serialization).
//!
//! A STIX bundle is a JSON object `"type": "bundle"` with
//! `"spec_version": "2.0"` or `"2.1"`, an `"id"` of
//! `"bundle--<uuid>"`, and an `"objects"` array of SDOs/SCOs/SROs.
//!
//! ```
//! let d = b"{\"type\":\"bundle\",\"spec_version\":\"2\x2e1\",
//!   \"id\":\"bundle--00000000-0000-4000-8000-000000000000\",
//!   \"objects\":[{\"type\":\"indicator\"},{\"type\":\"malware\"},{\"type\":\"indicator\"}]}";
//! let s = izanagi_kit::stix::parse(d).unwrap();
//! assert_eq!(s.spec_version, "2\x2e1");
//! assert_eq!(s.objects, 3);
//! assert_eq!(s.count("indicator"), 2);
//! ```
//!
//! Reference: OASIS STIX 2\x2e0/2\x2e1 specification — bundle object
//! shape and the `type` discriminator shared by all SDOs/SCOs/SROs.

use crate::json::{self, Json};

/// Parsed STIX bundle statistics.
#[derive(Debug, Clone, PartialEq)]
pub struct Stix {
    /// `"2\x2e0"` or `"2\x2e1"`.
    pub spec_version: String,
    /// Bundle `id` (`bundle--uuid`).
    pub id: Option<String>,
    /// Number of entries in `objects`.
    pub objects: usize,
    /// `type` strings of the bundle members, in order.
    pub types: Vec<String>,
}

fn str_field<'a>(m: &'a std::collections::BTreeMap<String, Json>, k: &str) -> Option<&'a str> {
    match m.get(k) {
        Some(Json::Str(s)) => Some(s),
        _ => None,
    }
}

impl Stix {
    /// Count of objects whose `type` equals `t`.
    pub fn count(&self, t: &str) -> usize {
        self.types.iter().filter(|x| x.as_str() == t).count()
    }
}

/// Parse a STIX 2.x bundle; `None` for non-STIX JSON.
pub fn parse(d: &[u8]) -> Option<Stix> {
    let root = json::parse(d).ok()?;
    let Json::Obj(m) = &root else {
        return None;
    };
    if str_field(m, "type")? != "bundle" {
        return None;
    }
    let spec = str_field(m, "spec_version")?;
    if spec != "2\x2e0" && spec != "2\x2e1" {
        return None;
    }
    let mut types = Vec::new();
    if let Some(Json::Arr(objs)) = m.get("objects") {
        for o in objs {
            if let Json::Obj(om) = o {
                if let Some(t) = str_field(om, "type") {
                    types.push(t.to_string());
                }
            }
        }
    }
    Some(Stix {
        spec_version: spec.to_string(),
        id: str_field(m, "id").map(|s| s.to_string()),
        objects: types.len(),
        types,
    })
}

/// `true` if the buffer looks like a STIX bundle.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b"{\"type\":\"bundle\",\"spec_version\":\"2\x2e1\",\"id\":\"bundle--00000000-0000-4000-8000-000000000000\",\"objects\":[{\"type\":\"indicator\"},{\"type\":\"malware\"},{\"type\":\"indicator\"},{\"type\":\"x\"}]}";

    #[test]
    fn parses() {
        let s = parse(DOC).unwrap();
        assert_eq!(s.spec_version, "2\x2e1");
        assert_eq!(s.objects, 4);
        assert_eq!(s.count("indicator"), 2);
        assert_eq!(s.count("malware"), 1);
        assert!(s.id.unwrap().starts_with("bundle--"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"{}").is_none());
        assert!(parse(b"{\"type\":\"bundle\",\"spec_version\":\"1\x2e0\"}").is_none());
        assert!(parse(b"{\"type\":\"indicator\",\"spec_version\":\"2\x2e1\"}").is_none());
        assert!(parse(b"[1]").is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(DOC));
        assert!(!detect(b"{}"));
    }
}
