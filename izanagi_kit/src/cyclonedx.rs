//! CycloneDX SBOM (OWASP CycloneDX, Ecma-424): JSON form is
//! `{"bomFormat":"CycloneDX","specVersion":"1.x",...}`; the XML form
//! is a `<bom xmlns="http://cyclonedx.org/schema/bom/1.x">` root.
//! Components are collected as `name@version` strings.
//!
//! ```
//! let j = br#"{"bomFormat":"CycloneDX","specVersion":"1.5",
//!   "serialNumber":"urn:uuid:1","components":[{"name":"a","version":"1"}]}"#;
//! let d = izanagi_kit::cyclonedx::parse(j).unwrap();
//! assert_eq!(d.spec_version, "1\x2e5");
//! assert_eq!(d.components, ["a@1"]);
//! ```

use crate::json::{self, Json};
use std::string::{String, ToString};
use std::vec::Vec;

/// A parsed CycloneDX document.
#[derive(Clone, Debug)]
pub struct CycloneDx {
    /// `specVersion` string (or the version tail of the XML namespace).
    pub spec_version: String,
    /// `serialNumber` (`urn:uuid:…`), when present.
    pub serial: Option<String>,
    /// `components[]` as `name@version` (`name` alone when unversioned).
    pub components: Vec<String>,
    /// `metadata.component.name`, when present.
    pub root_component: Option<String>,
    /// True when the input was the XML document form.
    pub is_xml: bool,
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

/// Parse a CycloneDX SBOM (JSON or XML root); `None` when the
/// `bomFormat`/`xmlns` markers are absent or wrong.
pub fn parse(d: &[u8]) -> Option<CycloneDx> {
    let src = std::str::from_utf8(d).ok()?;
    let t = src.trim_start();
    if t.starts_with('<') {
        return parse_xml(t);
    }
    let v = json::parse(d).ok()?;
    let m = obj(&v)?;
    if s(m.get("bomFormat")?)? != "CycloneDX" {
        return None;
    }
    let spec_version = s(m.get("specVersion")?)?.to_string();
    let serial = m.get("serialNumber").and_then(s).map(ToString::to_string);
    let root_component = m
        .get("metadata")
        .and_then(obj)
        .and_then(|md| md.get("component"))
        .and_then(obj)
        .and_then(|c| c.get("name"))
        .and_then(s)
        .map(ToString::to_string);
    let mut components = Vec::new();
    if let Some(list) = m.get("components").and_then(arr) {
        for c in list {
            let c = obj(c)?;
            let name = s(c.get("name")?)?;
            let ver = c.get("version").and_then(s).unwrap_or("");
            components.push(if ver.is_empty() {
                name.to_string()
            } else {
                format!("{name}@{ver}")
            });
        }
    }
    Some(CycloneDx {
        spec_version,
        serial,
        components,
        root_component,
        is_xml: false,
    })
}

fn parse_xml(src: &str) -> Option<CycloneDx> {
    let gt = src.find('>')?;
    let open = &src[..gt];
    if !open.contains("<bom") || open.contains("</") {
        return None;
    }
    let ns = {
        let pat = "xmlns=\"";
        let at = open.find(pat)? + pat.len();
        let end = open[at..].find('"')? + at;
        &open[at..end]
    };
    let rest = ns.strip_prefix("http://cyclonedx\x2eorg/schema/bom/")?;
    Some(CycloneDx {
        spec_version: rest.to_string(),
        serial: attr(open, "serialNumber"),
        components: Vec::new(),
        root_component: None,
        is_xml: true,
    })
}

fn attr(tag: &str, key: &str) -> Option<String> {
    let pat = format!("{key}=\"");
    let at = tag.find(&pat)? + pat.len();
    let end = tag[at..].find('"')? + at;
    Some(tag[at..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json() {
        let d = parse(
            br#"{"bomFormat":"CycloneDX","specVersion":"1.6",
            "metadata":{"component":{"name":"app"}},
            "components":[{"name":"a","version":"1"},{"name":"b"}]}"#,
        )
        .unwrap();
        assert_eq!(d.spec_version, "1\x2e6");
        assert_eq!(d.root_component.as_deref(), Some("app"));
        assert_eq!(d.components, ["a@1", "b"]);
        assert!(!d.is_xml);
    }

    #[test]
    fn xml() {
        let d = parse(b"<bom xmlns=\"http://cyclonedx.org/schema/bom/1.5\" serialNumber=\"urn:uuid:x\"></bom>").unwrap();
        assert_eq!(d.spec_version, "1\x2e5");
        assert_eq!(d.serial.as_deref(), Some("urn:uuid:x"));
        assert!(d.is_xml);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(br#"{"bomFormat":"SPDX"}"#).is_none());
        assert!(parse(br#"{"bomFormat":"CycloneDX"}"#).is_none()); // no specVersion
        assert!(parse(b"<html></html>").is_none());
        assert!(parse(b"<bom xmlns=\"http://other\x2eorg/\"></bom>").is_none());
    }
}
