//! OpenStreetMap XML (`<osm>`): top-level `<node>`/`<way>`/`<relation>`
//! elements with `id` attributes and nested `<tag k="" v=""/>` metadata.
//! Self-closing elements produce empty tag lists.
//!
//! ```
//! use izanagi_kit::osm::parse;
//!
//! let src = "<osm version=\"0.6\"><node id=\"1\" lat=\"0\" lon=\"0\">\
//!            <tag k=\"name\" v=\"X\"/></node><way id=\"2\"/></osm>";
//! let o = parse(src).unwrap();
//! assert_eq!(o.elements.len(), 2);
//! assert_eq!(o.elements[0].id, 1);
//! assert_eq!(o.elements[0].tag("name"), Some("X"));
//! ```

use std::string::String;
use std::vec::Vec;

/// Element kind byte: `n` node, `w` way, `r` relation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `<node>`
    Node,
    /// `<way>`
    Way,
    /// `<relation>`
    Relation,
}

/// One top-level OSM element.
#[derive(Debug, Clone)]
pub struct Element {
    /// Node/way/relation.
    pub kind: Kind,
    /// `id` attribute.
    pub id: i64,
    /// `(k, v)` tag pairs.
    pub tags: Vec<(String, String)>,
}

/// Parsed `<osm>` document.
#[derive(Debug, Clone)]
pub struct Osm {
    /// `version` attribute of `<osm>`.
    pub version: Option<String>,
    /// Elements in document order.
    pub elements: Vec<Element>,
}

/// `name="v"` (double or single quoted) inside a tag header slice.
fn attr(tag: &str, name: &str) -> Option<String> {
    let mut rest = tag;
    let pat = String::from(name);
    while let Some(i) = rest.find(pat.as_str()) {
        rest = &rest[i + pat.len()..];
        let rest = rest.trim_start();
        if !rest.starts_with('=') {
            continue;
        }
        let rest = rest[1..].trim_start();
        let q = rest.chars().next()?;
        if q != '"' && q != '\'' {
            return None;
        }
        let end = rest[1..].find(q)? + 1;
        return Some(String::from(&rest[1..end]));
    }
    None
}

impl Element {
    /// Look up a tag value by key.
    pub fn tag(&self, k: &str) -> Option<&str> {
        self.tags
            .iter()
            .find(|(kk, _)| kk == k)
            .map(|(_, v)| v.as_str())
    }
}

/// Parse `<osm>` — walks `<node`/`<way`/`<relation` openers in order.
pub fn parse(src: &str) -> Option<Osm> {
    let i = src.find("<osm")?;
    let version = {
        let gt = src[i..].find('>')? + i;
        attr(&src[i..gt], "version")
    };
    let mut elements = Vec::new();
    let mut at = i;
    while let Some(lt) = src[at..].find('<').map(|p| p + at) {
        if src[lt..].starts_with("</osm") || src[lt..].starts_with("<?") {
            break;
        }
        let kind = if src[lt..].starts_with("<node") {
            Kind::Node
        } else if src[lt..].starts_with("<way") {
            Kind::Way
        } else if src[lt..].starts_with("<relation") {
            Kind::Relation
        } else {
            at = lt + 1;
            continue;
        };
        let gt = src[lt..].find('>').map(|p| p + lt)?;
        let tag = &src[lt..gt];
        let id = attr(tag, "id").and_then(|s| s.parse().ok()).unwrap_or(0);
        let mut el = Element {
            kind,
            id,
            tags: Vec::new(),
        };
        at = gt + 1;
        if !tag.ends_with('/') {
            // scan children until `</node|way|relation>`; collect <tag>
            let close = match kind {
                Kind::Node => "</node>",
                Kind::Way => "</way>",
                Kind::Relation => "</relation>",
            };
            let end = src[at..].find(close).map(|p| p + at).unwrap_or(src.len());
            let mut t = at;
            while let Some(lt2) = src[t..end].find('<').map(|p| p + t) {
                if src[lt2..].starts_with("<tag") {
                    if let Some(gt2) = src[lt2..end].find('>').map(|p| p + lt2) {
                        let h = &src[lt2..gt2];
                        if let (Some(k), Some(v)) = (attr(h, "k"), attr(h, "v")) {
                            el.tags.push((k, v));
                        }
                        t = gt2 + 1;
                        continue;
                    }
                }
                t = lt2 + 1;
            }
            at = end + close.len();
        }
        elements.push(el);
    }
    Some(Osm { version, elements })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "<osm version='0.6' generator='x'>\
        <node id='1' lat='35' lon='139'><tag k='name' v='A'/><tag k='amenity' v='cafe'/></node>\
        <way id='7'><nd ref='1'/><tag k='highway' v='path'/></way>\
        <relation id='9'/></osm>";

    #[test]
    fn fields() {
        let o = parse(DOC).unwrap();
        assert_eq!(o.version.as_deref(), Some("0.6"));
        assert_eq!(o.elements.len(), 3);
        assert_eq!(o.elements[0].kind, Kind::Node);
        assert_eq!(o.elements[0].id, 1);
        assert_eq!(o.elements[0].tag("name"), Some("A"));
        assert_eq!(o.elements[0].tag("amenity"), Some("cafe"));
        assert_eq!(o.elements[1].kind, Kind::Way);
        assert_eq!(o.elements[1].tag("highway"), Some("path"));
        assert_eq!(o.elements[2].kind, Kind::Relation);
        assert_eq!(o.elements[2].id, 9);
    }

    #[test]
    fn rejects() {
        assert!(parse("<xml/>").is_none());
        assert_eq!(parse("<osm/>").unwrap().elements.len(), 0);
    }
}
