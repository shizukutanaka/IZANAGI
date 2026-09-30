//! VTK XML image/unstructured files (`.vtu` `.vti` `.vtp` `.vtr`
//! `.vts`): an XML document whose root is `<VTKFile
//! type="…" version="…" byte_order="…">` containing one matching
//! `<UnstructuredGrid>`/etc. element.
//!
//! ```
//! let d = br#"<VTKFile type="UnstructuredGrid" version="2.2" byte_order="LittleEndian"><UnstructuredGrid/></VTKFile>"#;
//! let v = izanagi_kit::vtu::parse(d).unwrap();
//! assert_eq!(v.kind, izanagi_kit::vtu::Kind::UnstructuredGrid);
//! ```

use std::string::String;

/// VTK XML dataset kind (from the `type` attribute).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `.vtu` unstructured grid.
    UnstructuredGrid,
    /// `.vti` image data.
    ImageData,
    /// `.vtp` polydata.
    PolyData,
    /// `.vtr` rectilinear grid.
    RectilinearGrid,
    /// `.vts` structured grid.
    StructuredGrid,
    /// Any other type attribute.
    Other(String),
}

/// A parsed VTK XML file.
#[derive(Clone, Debug)]
pub struct Vtu {
    /// Dataset kind.
    pub kind: Kind,
    /// `version` attribute verbatim.
    pub version: Option<String>,
    /// `byte_order` attribute verbatim.
    pub byte_order: Option<String>,
}

fn attr(tag: &str, name: &str) -> Option<String> {
    let pat = format!("{}=\"", name);
    let i = tag.find(&pat)?;
    let rest = &tag[i + pat.len()..];
    let j = rest.find('"')?;
    Some(rest[..j].to_string())
}

/// Parse a VTK XML file; `None` without `<VTKFile type="…">`.
pub fn parse(d: &[u8]) -> Option<Vtu> {
    let s = std::str::from_utf8(d).ok()?;
    let s = s.trim_start_matches(|c: char| c == '\u{feff}' || c.is_whitespace());
    // tolerate an optional <?xml prolog
    let s = s
        .strip_prefix("<?xml")
        .map(|r| r.split_once("?>").map(|(_, r)| r).unwrap_or(""))
        .unwrap_or(s);
    let s = s.trim_start();
    let rest = s.strip_prefix("<VTKFile")?;
    let end = rest.find('>')?;
    let open = &rest[..end];
    let ty = attr(open, "type")?;
    let kind = match ty.as_str() {
        "UnstructuredGrid" => Kind::UnstructuredGrid,
        "ImageData" => Kind::ImageData,
        "PolyData" => Kind::PolyData,
        "RectilinearGrid" => Kind::RectilinearGrid,
        "StructuredGrid" => Kind::StructuredGrid,
        other => Kind::Other(other.to_string()),
    };
    Some(Vtu {
        kind,
        version: attr(open, "version"),
        byte_order: attr(open, "byte_order"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let d = br#"<?xml version="1.0"?><VTKFile type="ImageData" version="1.0" byte_order="LittleEndian"><ImageData/></VTKFile>"#;
        let v = parse(d).unwrap();
        assert_eq!(v.kind, Kind::ImageData);
        assert_eq!(v.version.as_deref(), Some("1.0"));
        assert_eq!(v.byte_order.as_deref(), Some("LittleEndian"));
    }

    #[test]
    fn unstructured() {
        let d = br#"<VTKFile type="UnstructuredGrid"><UnstructuredGrid/></VTKFile>"#;
        assert_eq!(parse(d).unwrap().kind, Kind::UnstructuredGrid);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<UnstructuredGrid/>").is_none());
        assert!(parse(br#"<VTKFile version="1.0"/>"#).is_none()); // no type
    }
}
