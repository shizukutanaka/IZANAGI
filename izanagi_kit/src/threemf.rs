//! Minimal reader for 3MF containers: an OPC zip package whose model part
//! lives at `/3D/3dmodel.model`. Built on [`crate::zip`]; reports the part
//! list, the model XML's `<object>` count, and `<build><item objectid>`
//! references.
//!
//! ```
//! use izanagi_kit::threemf::parse;
//!
//! // build a stored (uncompressed) zip with the model part
//! let mut w = izanagi_kit::zip::ZipWriter::new();
//! w.add_stored(
//!     "3D/3dmodel.model",
//!     b"<model><resources><object id=\"1\"/><object id=\"2\"/></resources>\
//!       <build><item objectid=\"1\"/></build></model>",
//! );
//! let z = w.finish();
//! let m = parse(&z).unwrap();
//! assert_eq!(m.objects, 2);
//! assert_eq!(m.build, vec![1]);
//! ```

use crate::zip;

/// A parsed 3MF package.
#[derive(Debug)]
pub struct ThreeMf {
    /// All package part names.
    pub parts: Vec<String>,
    /// `<object>` elements in the model part.
    pub objects: usize,
    /// `objectid` values of `<build><item>` references, in order.
    pub build: Vec<u64>,
}

fn attr(tag: &str, key: &str) -> Option<String> {
    let pat = format!("{key}=\"");
    let at = tag.find(&pat)? + pat.len();
    let end = tag[at..].find('"')? + at;
    Some(tag[at..end].to_string())
}

/// Parse a 3MF package. `None` when it isn't a zip or has no
/// `3D/3dmodel.model` part (with or without the leading slash).
pub fn parse(data: &[u8]) -> Option<ThreeMf> {
    let entries = zip::list(data)?;
    let parts: Vec<String> = entries.iter().map(|e| e.name.clone()).collect();
    let model = parts
        .iter()
        .find(|n| {
            let t = n.trim_start_matches('/');
            t.eq_ignore_ascii_case("3d/3dmodel.model")
        })
        .cloned()?;
    let xml_bytes = zip::extract(data, &model)?;
    let src = std::str::from_utf8(&xml_bytes).ok()?;
    let mut objects = 0usize;
    let mut build = Vec::new();
    let rest = src;
    let mut at = 0;
    while let Some(p) = rest[at..].find('<') {
        let a = at + p;
        let name_end = rest[a..]
            .find(|c: char| [' ', '>', '/', '\t', '\n'].contains(&c))
            .map(|e| a + e)
            .unwrap_or(rest.len());
        let name = &rest[a + 1..name_end.min(rest.len())];
        if name == "object" {
            objects += 1;
        } else if name == "item" {
            let tag_end = rest[a..].find('>').map(|e| a + e + 1)?;
            if let Some(v) = attr(&rest[a..tag_end], "objectid") {
                if let Ok(id) = v.parse() {
                    build.push(id);
                }
            }
        }
        at = a + 1;
    }
    Some(ThreeMf {
        parts,
        objects,
        build,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pkg(model: &[u8]) -> Vec<u8> {
        let mut w = zip::ZipWriter::new();
        w.add_stored("[Content_Types].xml", b"<Types/>");
        w.add_stored("3D/3dmodel.model", model);
        w.finish()
    }

    #[test]
    fn parses() {
        let z = pkg(
            b"<model><resources><object id=\"1\"/><object id=\"7\"/></resources>\
                    <build><item objectid=\"1\"/><item objectid=\"7\"/></build></model>",
        );
        let m = parse(&z).unwrap();
        assert_eq!(m.parts.len(), 2);
        assert_eq!(m.objects, 2);
        assert_eq!(m.build, vec![1, 7]);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"PK not a zip").is_none());
        // a zip without the model part
        let mut w = zip::ZipWriter::new();
        w.add_stored("readme.txt", b"hi");
        assert!(parse(&w.finish()).is_none());
    }
}
