//! SPDX tag-value SBOM (Linux Foundation SPDX 2.x, ISO/IEC 5962):
//! flat `Tag: value` lines; `SPDXVersion: SPDX-x.y` must come first,
//! `DataLicense`, `DocumentName`, `DocumentNamespace` and a document
//! `SPDXID` are required. `PackageName:` lines are collected as the
//! package list.
//!
//! ```
//! let s = "SPDXVersion: SPDX-2\x2e3\n\
//!          DataLicense: CC0-1\x2e0\n\
//!          SPDXID: SPDXRef-DOCUMENT\n\
//!          DocumentName: demo\n\
//!          DocumentNamespace: https://e\x2ex/sbom-1\n\
//!          PackageName: zlib\nPackageName: libc\n";
//! let d = izanagi_kit::spdx::parse(s).unwrap();
//! assert_eq!(d.name, "demo");
//! assert_eq!(d.packages, ["zlib", "libc"]);
//! ```

use std::string::{String, ToString};
use std::vec::Vec;

/// A parsed SPDX tag-value document.
#[derive(Clone, Debug)]
pub struct Spdx {
    /// `SPDXVersion` (e.g. `SPDX-2.3`).
    pub version: String,
    /// `DataLicense` (always `CC0-1.0` per spec).
    pub data_license: String,
    /// `DocumentName`.
    pub name: String,
    /// `DocumentNamespace`.
    pub namespace: String,
    /// `Creator` line(s), verbatim.
    pub creators: Vec<String>,
    /// Every `PackageName` in order.
    pub packages: Vec<String>,
    /// `Relationship:` lines, verbatim.
    pub relationships: Vec<String>,
    /// All tags as ordered `(name, value)` pairs.
    pub tags: Vec<(String, String)>,
}

/// Parse an SPDX tag-value document; `None` when the required
/// header tags (`SPDXVersion` first, then `DataLicense`,
/// `SPDXID: SPDXRef-DOCUMENT`, `DocumentName`, `DocumentNamespace`)
/// are absent.
pub fn parse(s: &str) -> Option<Spdx> {
    let mut tags: Vec<(String, String)> = Vec::new();
    for line in s.lines() {
        let line = line.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once(':') {
            let k = k.trim();
            if !k.is_empty() {
                tags.push((k.to_string(), v.trim().to_string()));
            }
        }
    }
    // SPDXVersion must be the first tag.
    let (k, v) = tags.first()?;
    if k != "SPDXVersion" || !v.starts_with("SPDX-") {
        return None;
    }
    let version = v.clone();
    let get = |name: &str| -> Option<&str> {
        tags.iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    };
    let data_license = get("DataLicense")?.to_string();
    // The document's own SPDXID must be SPDXRef-DOCUMENT.
    if get("SPDXID")? != "SPDXRef-DOCUMENT" {
        return None;
    }
    let name = get("DocumentName")?.to_string();
    let namespace = get("DocumentNamespace")?.to_string();
    let collect = |name: &str| -> Vec<String> {
        tags.iter()
            .filter(|(k, _)| k == name)
            .map(|(_, v)| v.clone())
            .collect()
    };
    Some(Spdx {
        version,
        data_license,
        name,
        namespace,
        creators: collect("Creator"),
        packages: collect("PackageName"),
        relationships: collect("Relationship"),
        tags,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "SPDXVersion: SPDX-2\x2e3\nDataLicense: CC0-1\x2e0\n\
                       SPDXID: SPDXRef-DOCUMENT\nDocumentName: app\n\
                       DocumentNamespace: https://e\x2ex/1\n\
                       Creator: Tool: t\nPackageName: a\nPackageName: b\n\
                       Relationship: SPDXRef-DOCUMENT DESCRIBES SPDXRef-Package-a\n";

    #[test]
    fn parses() {
        let d = parse(DOC).unwrap();
        assert_eq!(d.version, "SPDX-2\x2e3");
        assert_eq!(d.data_license, "CC0-1\x2e0");
        assert_eq!(d.name, "app");
        assert_eq!(d.packages.len(), 2);
        assert_eq!(d.creators, ["Tool: t"]);
        assert_eq!(d.relationships.len(), 1);
    }

    #[test]
    fn rejects() {
        assert!(parse("").is_none());
        // SPDXVersion not first
        assert!(parse("DocumentName: x\nSPDXVersion: SPDX-2\x2e3\nDataLicense: C\nSPDXID: SPDXRef-DOCUMENT\nDocumentNamespace: n").is_none());
        // wrong doc SPDXID
        assert!(parse("SPDXVersion: SPDX-2\x2e3\nDataLicense: C\nSPDXID: SPDXRef-Package-x\nDocumentName: x\nDocumentNamespace: n").is_none());
        // missing namespace
        assert!(parse(
            "SPDXVersion: SPDX-2\x2e3\nDataLicense: C\nSPDXID: SPDXRef-DOCUMENT\nDocumentName: x"
        )
        .is_none());
        // non-SPDX version
        assert!(parse("SPDXVersion: 2\x2e3\nDataLicense: C\nSPDXID: SPDXRef-DOCUMENT\nDocumentName: x\nDocumentNamespace: n").is_none());
    }
}
