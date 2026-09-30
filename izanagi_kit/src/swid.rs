//! SWID tag — Software Identification (ISO/IEC 19770-2): an XML
//! document rooted at `<SoftwareIdentity name=… tagId=… version=…>`
//! with optional `<Entity name=… role=…>` and `<Link href=… rel=…>`
//! children. Namespace prefixes are tolerated.
//!
//! ```
//! let s = "<SoftwareIdentity name=\"App\" tagId=\"com.e/app/1\" version=\"1\">\
//!          <Entity name=\"E\" role=\"softwareCreator\"/></SoftwareIdentity>";
//! let d = izanagi_kit::swid::parse(s).unwrap();
//! assert_eq!(d.name, "App");
//! assert_eq!(d.entities[0].0, "E");
//! ```

use std::string::{String, ToString};
use std::vec::Vec;

/// A parsed SWID tag.
#[derive(Clone, Debug)]
pub struct Swid {
    /// `SoftwareIdentity@name`.
    pub name: String,
    /// `SoftwareIdentity@tagId` — globally unique tag identifier.
    pub tag_id: String,
    /// `SoftwareIdentity@version` (string form; may be `0` in `versionScheme`).
    pub version: Option<String>,
    /// `SoftwareIdentity@versionScheme` when present.
    pub version_scheme: Option<String>,
    /// `SoftwareIdentity@xml:lang` when present.
    pub lang: Option<String>,
    /// `(name, role)` of each `<Entity>` in order.
    pub entities: Vec<(String, String)>,
    /// `href` of each `<Link>` in order.
    pub links: Vec<String>,
}

fn attr(tag: &str, key: &str) -> Option<String> {
    for q in ['"', '\''] {
        let pat = format!("{key}={q}");
        if let Some(at) = tag.find(&pat) {
            let at = at + pat.len();
            if let Some(end) = tag[at..].find(q) {
                return Some(tag[at..at + end].to_string());
            }
        }
    }
    None
}

/// Collect the inner text of every `<…SoftwareIdentity …>`-style
/// open tag matching `local` (with or without a `ns:` prefix).
fn tags<'a>(src: &'a str, local: &str) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut rest = src;
    while let Some(a) = rest.find('<') {
        let gt = match rest[a..].find('>') {
            Some(g) => a + g,
            None => break,
        };
        let tag = &rest[a..=gt];
        let inner = &tag[1..tag.len() - 1];
        let inner = inner.trim_start_matches('/');
        let name_end = inner
            .find(|c: char| c.is_whitespace() || c == '/' || c == '>')
            .unwrap_or(inner.len());
        let name = &inner[..name_end];
        let local_name = name.rsplit_once(':').map(|(_, l)| l).unwrap_or(name);
        if local_name == local && !rest[a..].starts_with("</") {
            out.push(tag);
        }
        rest = &rest[gt + 1..];
    }
    out
}

/// Parse a SWID tag; `None` without a `<SoftwareIdentity>` root
/// carrying both `name` and `tagId`.
pub fn parse(s: &str) -> Option<Swid> {
    let ids = tags(s, "SoftwareIdentity");
    let root = *ids.first()?;
    // An optional `<?xml …?>` prolog may precede the root; nothing else.
    let head = s[..s.find(root)?].trim();
    if !head.is_empty() && !(head.starts_with("<?xml") && head.ends_with("?>")) {
        return None;
    }
    let name = attr(root, "name")?;
    let tag_id = attr(root, "tagId")?;
    let entities = tags(s, "Entity")
        .iter()
        .filter_map(|t| Some((attr(t, "name")?, attr(t, "role").unwrap_or_default())))
        .collect();
    let links = tags(s, "Link")
        .iter()
        .filter_map(|t| attr(t, "href"))
        .collect();
    Some(Swid {
        name,
        tag_id,
        version: attr(root, "version"),
        version_scheme: attr(root, "versionScheme"),
        lang: attr(root, "xml:lang"),
        entities,
        links,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let d = parse(
            "<?xml version=\"1.0\"?><swid:SoftwareIdentity xmlns:swid=\"x\" \
             name=\"App\" tagId=\"id1\" version=\"2\x2e0\" versionScheme=\"semver\">\
             <Entity name=\"Me\" role=\"softwareCreator\"/>\
             <Link href=\"h\" rel=\"supplemental\"/></swid:SoftwareIdentity>",
        )
        .unwrap();
        assert_eq!(d.tag_id, "id1");
        assert_eq!(d.version.as_deref(), Some("2\x2e0"));
        assert_eq!(d.version_scheme.as_deref(), Some("semver"));
        assert_eq!(
            d.entities,
            [("Me".to_string(), "softwareCreator".to_string())]
        );
        assert_eq!(d.links, ["h"]);
    }

    #[test]
    fn rejects() {
        assert!(parse("").is_none());
        assert!(parse("<Other name=\"x\" tagId=\"t\"/>").is_none());
        // tagId required
        assert!(parse("<SoftwareIdentity name=\"x\"/>").is_none());
        // name required
        assert!(parse("<SoftwareIdentity tagId=\"t\"/>").is_none());
    }
}
