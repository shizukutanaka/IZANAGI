//! OSV vulnerability record (Open Source Vulnerabilities schema):
//! JSON object with a required `id` (`GHSA-…`, `CVE-…`, `OSV-…` …),
//! optional `schema_version` (`1.x`), `affected[]` entries each with
//! `package.ecosystem`, and `published`/`modified` timestamps.
//!
//! ```
//! let d = izanagi_kit::osv::parse(br#"{"id":"GHSA-x","schema_version":"1.6",
//!   "affected":[{"package":{"ecosystem":"crates.io","name":"a"}}]}"#).unwrap();
//! assert_eq!(d.id, "GHSA-x");
//! assert_eq!(d.ecosystems, ["crates.io"]);
//! ```

use crate::json::{self, Json};
use std::string::{String, ToString};
use std::vec::Vec;

/// A parsed OSV record.
#[derive(Clone, Debug)]
pub struct Osv {
    /// `id` — the vulnerability identifier.
    pub id: String,
    /// `schema_version` when present.
    pub schema_version: Option<String>,
    /// `summary` when present.
    pub summary: Option<String>,
    /// `published` timestamp when present.
    pub published: Option<String>,
    /// `modified` timestamp when present.
    pub modified: Option<String>,
    /// Distinct `affected[].package.ecosystem` values in order.
    pub ecosystems: Vec<String>,
    /// Count of `affected[]` entries.
    pub affected: usize,
    /// Count of `references[]` entries.
    pub references: usize,
    /// `aliases` (e.g. CVE ids) when present.
    pub aliases: Vec<String>,
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

/// Parse an OSV record; `None` without a string `id`.
pub fn parse(d: &[u8]) -> Option<Osv> {
    let v = json::parse(d).ok()?;
    let m = obj(&v)?;
    let id = s(m.get("id")?)?.to_string();
    let affected_list = m.get("affected").and_then(arr).unwrap_or(&[]);
    let mut ecosystems = Vec::new();
    for a in affected_list {
        if let Some(eco) = obj(a)
            .and_then(|a| a.get("package"))
            .and_then(obj)
            .and_then(|p| p.get("ecosystem"))
            .and_then(s)
        {
            if !ecosystems.iter().any(|e| e == eco) {
                ecosystems.push(eco.to_string());
            }
        }
    }
    Some(Osv {
        id,
        schema_version: m.get("schema_version").and_then(s).map(ToString::to_string),
        summary: m.get("summary").and_then(s).map(ToString::to_string),
        published: m.get("published").and_then(s).map(ToString::to_string),
        modified: m.get("modified").and_then(s).map(ToString::to_string),
        ecosystems,
        affected: affected_list.len(),
        references: m.get("references").and_then(arr).map_or(0, |r| r.len()),
        aliases: m.get("aliases").and_then(arr).map_or(Vec::new(), |a| {
            a.iter().filter_map(s).map(ToString::to_string).collect()
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let d = parse(
            br#"{"id":"OSV-2024-1","schema_version":"1.6","summary":"s",
               "published":"2024-01-01T00:00:00Z","modified":"2024-01-02T00:00:00Z",
               "aliases":["CVE-2024-1"],
               "affected":[{"package":{"ecosystem":"crates.io"}},
                           {"package":{"ecosystem":"PyPI"}},
                           {"package":{"ecosystem":"crates.io"}}],
               "references":[{"type":"WEB","url":"https://e"}]}"#,
        )
        .unwrap();
        assert_eq!(d.id, "OSV-2024-1");
        assert_eq!(d.schema_version.as_deref(), Some("1\x2e6"));
        assert_eq!(d.ecosystems, ["crates.io", "PyPI"]);
        assert_eq!(d.affected, 3);
        assert_eq!(d.references, 1);
        assert_eq!(d.aliases, ["CVE-2024-1"]);
    }

    #[test]
    fn minimal() {
        let d = parse(br#"{"id":"X"}"#).unwrap();
        assert_eq!(d.affected, 0);
        assert!(d.ecosystems.is_empty());
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"{}").is_none());
        assert!(parse(br#"{"id":1}"#).is_none()); // id must be a string
        assert!(parse(b"[]").is_none());
    }
}
