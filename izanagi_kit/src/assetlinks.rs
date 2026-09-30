//! Digital Asset Links — `/.well-known/assetlinks.json` (Android /
//! Google App Links): a JSON array of statements
//! `[{"relation":["delegate_permission/common.handle_all_urls",…],
//! "target":{"namespace":"android_app","package_name":"…",
//! "sha256_cert_fingerprints":["…"]}}]` — web targets use
//! `namespace:"web"` + `site:`.
//!
//! ```
//! let d = izanagi_kit::assetlinks::parse(br#"[{"relation":["r"],
//!   "target":{"namespace":"android_app","package_name":"com.e",
//!   "sha256_cert_fingerprints":["AA"]}}]"#).unwrap();
//! assert_eq!(d.entries[0].package.as_deref(), Some("com.e"));
//! ```

use crate::json::{self, Json};
use std::string::{String, ToString};
use std::vec::Vec;

/// One asset-links statement.
#[derive(Clone, Debug)]
pub struct Entry {
    /// `relation` strings (e.g. `delegate_permission/common.handle_all_urls`).
    pub relations: Vec<String>,
    /// `target.namespace` (`android_app` or `web`).
    pub namespace: String,
    /// `target.package_name` for `android_app` targets.
    pub package: Option<String>,
    /// `target.site` for `web` targets.
    pub site: Option<String>,
    /// `target.sha256_cert_fingerprints` list.
    pub fingerprints: Vec<String>,
}

/// A parsed assetlinks.json document.
#[derive(Clone, Debug)]
pub struct Assetlinks {
    /// Statements in order.
    pub entries: Vec<Entry>,
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
fn str_arr(v: Option<&Json>) -> Vec<String> {
    v.and_then(arr).map_or(Vec::new(), |a| {
        a.iter().filter_map(s).map(ToString::to_string).collect()
    })
}

/// Parse assetlinks.json; `None` unless a non-empty JSON array of
/// statements with `relation` + `target.namespace` is present.
pub fn parse(d: &[u8]) -> Option<Assetlinks> {
    let v = json::parse(d).ok()?;
    let items = arr(&v)?;
    if items.is_empty() {
        return None;
    }
    let mut entries = Vec::new();
    for it in items {
        let it = obj(it)?;
        let relations = str_arr(it.get("relation"));
        if relations.is_empty() {
            return None;
        }
        let target = it.get("target").and_then(obj)?;
        let namespace = s(target.get("namespace")?)?.to_string();
        entries.push(Entry {
            relations,
            namespace,
            package: target
                .get("package_name")
                .and_then(s)
                .map(ToString::to_string),
            site: target.get("site").and_then(s).map(ToString::to_string),
            fingerprints: str_arr(target.get("sha256_cert_fingerprints")),
        });
    }
    Some(Assetlinks { entries })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn android_app() {
        let d = parse(
            br#"[{"relation":["delegate_permission/common.handle_all_urls"],
            "target":{"namespace":"android_app","package_name":"com.e.app",
            "sha256_cert_fingerprints":["AB","CD"]}},
            {"relation":["r2"],"target":{"namespace":"web","site":"https://e"}}]"#,
        )
        .unwrap();
        assert_eq!(d.entries.len(), 2);
        assert_eq!(d.entries[0].fingerprints, ["AB", "CD"]);
        assert_eq!(d.entries[1].site.as_deref(), Some("https://e"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"[]").is_none());
        assert!(parse(b"{}").is_none());
        // relation required
        assert!(parse(br#"[{"target":{"namespace":"web"}}]"#).is_none());
        // namespace required
        assert!(parse(br#"[{"relation":["r"],"target":{}}]"#).is_none());
    }
}
