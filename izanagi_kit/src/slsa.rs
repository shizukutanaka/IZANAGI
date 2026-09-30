//! SLSA provenance (slsa.dev): an in-toto Statement whose
//! `predicateType` is `https://slsa.dev/provenance/vN[.m]`.
//! Extracts the statement version, predicate version, `buildType`
//! and `builder.id` (v1: `predicate.buildDefinition.buildType`;
//! v0.x: `predicate.buildType` / `predicate.builder.id`).
//!
//! ```
//! let d = izanagi_kit::slsa::parse(br#"{"_type":"https://in-toto.io/Statement-v1",
//!   "subject":[{"name":"a"}],"predicateType":"https://slsa.dev/provenance/v1",
//!   "predicate":{"buildDefinition":{"buildType":"bt","externalParameters":{}}}}"#)
//!   .unwrap();
//! assert_eq!(d.predicate_version, "v1");
//! assert_eq!(d.build_type.as_deref(), Some("bt"));
//! ```

use crate::json::{self, Json};
use std::string::{String, ToString};
use std::vec::Vec;

/// A parsed SLSA provenance statement.
#[derive(Clone, Debug)]
pub struct Slsa {
    /// `_type` version suffix (e.g. `v1` from `…/Statement-v1`).
    pub statement_version: String,
    /// `predicateType` tail after `…/provenance/` (e.g. `v1`, `v0.2`).
    pub predicate_version: String,
    /// Build type URI when present.
    pub build_type: Option<String>,
    /// `builder.id` when present.
    pub builder: Option<String>,
    /// `subject[].name` list.
    pub subjects: Vec<String>,
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

/// Parse a SLSA provenance statement; `None` when the envelope is
/// not an in-toto Statement or `predicateType` is not slsa.dev's.
pub fn parse(d: &[u8]) -> Option<Slsa> {
    let v = json::parse(d).ok()?;
    let m = obj(&v)?;
    let ty = s(m.get("_type")?)?;
    let statement_version = ty
        .strip_prefix("https://in-toto\x2eio/Statement-")?
        .to_string();
    let pt = s(m.get("predicateType")?)?;
    let predicate_version = pt
        .strip_prefix("https://slsa\x2edev/provenance/")?
        .to_string();
    let predicate = m.get("predicate").and_then(obj);
    // v1 nests under buildDefinition; v0.x is flat.
    let (build_type, builder) = if let Some(bd) = predicate
        .and_then(|p| p.get("buildDefinition"))
        .and_then(obj)
    {
        (
            bd.get("buildType").and_then(s).map(ToString::to_string),
            predicate
                .and_then(|p| p.get("builder"))
                .and_then(obj)
                .and_then(|b| b.get("id"))
                .and_then(s)
                .map(ToString::to_string),
        )
    } else {
        (
            predicate
                .and_then(|p| p.get("buildType"))
                .and_then(s)
                .map(ToString::to_string),
            predicate
                .and_then(|p| p.get("builder"))
                .and_then(obj)
                .and_then(|b| b.get("id"))
                .and_then(s)
                .map(ToString::to_string),
        )
    };
    let subjects = m.get("subject").and_then(arr).map_or(Vec::new(), |a| {
        a.iter()
            .filter_map(|e| obj(e).and_then(|e| e.get("name")).and_then(s))
            .map(ToString::to_string)
            .collect()
    });
    Some(Slsa {
        statement_version,
        predicate_version,
        build_type,
        builder,
        subjects,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1() {
        let d = parse(
            br#"{"_type":"https://in-toto.io/Statement-v1",
            "subject":[{"name":"pkg:x"}],
            "predicateType":"https://slsa.dev/provenance/v1",
            "predicate":{"buildDefinition":{"buildType":"https://bt"},
                         "builder":{"id":"https://ci"},"runDetails":{}}}"#,
        )
        .unwrap();
        assert_eq!(d.statement_version, "v1");
        assert_eq!(d.predicate_version, "v1");
        assert_eq!(d.build_type.as_deref(), Some("https://bt"));
        assert_eq!(d.builder.as_deref(), Some("https://ci"));
        assert_eq!(d.subjects, ["pkg:x"]);
    }

    #[test]
    fn v02_flat() {
        let d = parse(
            br#"{"_type":"https://in-toto.io/Statement-v0.1",
            "predicateType":"https://slsa.dev/provenance/v0.2",
            "predicate":{"buildType":"t","builder":{"id":"b"}}}"#,
        )
        .unwrap();
        assert_eq!(d.statement_version, "v0\x2e1");
        assert_eq!(d.predicate_version, "v0\x2e2");
        assert_eq!(d.build_type.as_deref(), Some("t"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"{}").is_none());
        // wrong predicateType
        assert!(parse(
            br#"{"_type":"https://in-toto.io/Statement-v1",
            "predicateType":"https://other/x"}"#
        )
        .is_none());
        // not an in-toto statement
        assert!(parse(
            br#"{"_type":"x",
            "predicateType":"https://slsa.dev/provenance/v1"}"#
        )
        .is_none());
    }
}
