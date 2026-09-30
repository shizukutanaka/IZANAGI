//! in-toto attestation envelopes (in-toto Attestation Framework):
//! a JSON Statement `{"_type":"https://in-toto.io/Statement-vN",
//! "subject":[{name,digest}],"predicateType":"…","predicate":{…}}`,
//! or the legacy layout `"_type":"link"`/`"statement"`.
//!
//! ```
//! let d = izanagi_kit::intoto::parse(br#"{"_type":"https://in-toto.io/Statement-v1",
//!   "subject":[{"name":"a","digest":{"sha256":"00"}}],
//!   "predicateType":"https://slsa.dev/provenance/v1","predicate":{}}"#).unwrap();
//! assert_eq!(d.kind, izanagi_kit::intoto::Kind::Statement);
//! assert_eq!(d.subjects, ["a"]);
//! ```

use crate::json::{self, Json};
use std::string::{String, ToString};
use std::vec::Vec;

/// Envelope kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `_type` is a `…/Statement-vN` IRI or the legacy `"statement"`.
    Statement,
    /// `_type` is a `…/Link` IRI or the legacy `"link"`.
    Link,
}

/// A parsed in-toto envelope.
#[derive(Clone, Debug)]
pub struct InToto {
    /// Statement vs Link.
    pub kind: Kind,
    /// `_type` verbatim.
    pub type_iri: String,
    /// `subject[].name` in order.
    pub subjects: Vec<String>,
    /// `predicateType` IRI when present.
    pub predicate_type: Option<String>,
    /// True when a `predicate` object is present.
    pub has_predicate: bool,
    /// Count of `materials[]` / `products[]` (Link layouts).
    pub materials: usize,
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

/// Parse an in-toto envelope; `None` when `_type` is not an
/// in-toto Statement/Link form.
pub fn parse(d: &[u8]) -> Option<InToto> {
    let v = json::parse(d).ok()?;
    let m = obj(&v)?;
    let t = s(m.get("_type")?)?;
    let kind = if t == "statement" || t.starts_with("https://in-toto\x2eio/Statement-v") {
        Kind::Statement
    } else if t == "link" || t.starts_with("https://in-toto\x2eio/Link") {
        Kind::Link
    } else {
        return None;
    };
    let subjects = m.get("subject").and_then(arr).map_or(Vec::new(), |a| {
        a.iter()
            .filter_map(|e| obj(e).and_then(|e| e.get("name")).and_then(s))
            .map(ToString::to_string)
            .collect()
    });
    if kind == Kind::Statement && subjects.is_empty() {
        return None; // statements must carry at least one subject
    }
    Some(InToto {
        kind,
        type_iri: t.to_string(),
        subjects,
        predicate_type: m.get("predicateType").and_then(s).map(ToString::to_string),
        has_predicate: m.get("predicate").is_some(),
        materials: m
            .get("materials")
            .or_else(|| m.get("products"))
            .and_then(arr)
            .map_or(0, |a| a.len()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statement_v1() {
        let d = parse(
            br#"{"_type":"https://in-toto.io/Statement-v1",
            "subject":[{"name":"a"},{"name":"b"}],
            "predicateType":"https://x/y","predicate":{"k":1}}"#,
        )
        .unwrap();
        assert_eq!(d.kind, Kind::Statement);
        assert_eq!(d.subjects, ["a", "b"]);
        assert_eq!(d.predicate_type.as_deref(), Some("https://x/y"));
        assert!(d.has_predicate);
    }

    #[test]
    fn legacy_link() {
        let d = parse(
            br#"{"_type":"link","name":"step",
            "materials":[{"a":1},{"b":2}]}"#,
        )
        .unwrap();
        assert_eq!(d.kind, Kind::Link);
        assert_eq!(d.materials, 2);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"{}").is_none());
        assert!(parse(br#"{"_type":"other"}"#).is_none());
        // Statement without subject
        assert!(parse(br#"{"_type":"https://in-toto.io/Statement-v1"}"#).is_none());
        assert!(parse(b"").is_none());
    }
}
