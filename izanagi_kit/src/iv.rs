//! Open Inventor `.iv` — SGI Open Inventor scene-graph text.
//!
//! Files open with `#Inventor V2.0 ascii` (also `V2.1`, `V1.0`).
//! The body is a node tree: `Separator { ... }`, `DEF name Node`,
//! `USE name`, with field lines inside braces. Quoted strings and
//! `#` comments are skipped during the census.
//!
//! ```
//! let d = b"#Inventor V2\x2e0 ascii\nSeparator {\n  DEF C Cube { }\n  USE C\n}\n";
//! let f = izanagi_kit::iv::parse(d).unwrap();
//! assert_eq!(f.version, "2\x2e0");
//! assert_eq!(f.grouping, 1);
//! assert_eq!(f.shapes, 1);
//! assert_eq!(f.defs, 1);
//! assert_eq!(f.uses, 1);
//! ```
//!
//! Reference: Open Inventor file format (SGI inventor documentation);
//! Coin3D `SoDB` importer. Integer-only.

/// Parsed `.iv` header + node census.
#[derive(Debug, Clone, PartialEq)]
pub struct Iv {
    /// Header version digits, e.g. `2.0`, `2.1`.
    pub version: String,
    /// `DEF name` bindings.
    pub defs: u32,
    /// `USE name` references.
    pub uses: u32,
    /// Grouping nodes: `Separator`, `Group`, `TransformSeparator`,
    /// `Switch`, `LevelOfDetail`, `Array`, `MultipleCopy`,
    /// `PathSwitch`, `WWWAnchor`, `Annotation`.
    pub grouping: u32,
    /// Shape nodes: `Cube`, `Sphere`, `Cone`, `Cylinder`,
    /// `IndexedFaceSet`, `IndexedLineSet`, `PointSet`, `FaceSet`,
    /// `LineSet`, `QuadMesh`, `TriangleStripSet`, `Text3`,
    /// `IndexedTriangleStripSet`, `IndexedNurbsSurface`.
    pub shapes: u32,
    /// Total `{}` nodes counted.
    pub total_nodes: u32,
}

const GROUPING: &[&str] = &[
    "Separator",
    "Group",
    "TransformSeparator",
    "Switch",
    "LevelOfDetail",
    "Array",
    "MultipleCopy",
    "PathSwitch",
    "WWWAnchor",
    "Annotation",
];

const SHAPES: &[&str] = &[
    "Cube",
    "Sphere",
    "Cone",
    "Cylinder",
    "IndexedFaceSet",
    "IndexedLineSet",
    "PointSet",
    "FaceSet",
    "LineSet",
    "QuadMesh",
    "TriangleStripSet",
    "Text3",
    "IndexedTriangleStripSet",
    "IndexedNurbsSurface",
];

/// Scan for `#Inventor` in the first line and return its version.
fn header_version(s: &str) -> Option<&str> {
    let line = s.lines().next()?.trim_start();
    let h = line.strip_prefix("#Inventor")?;
    let h = h.trim_start();
    let h = h.strip_prefix('V')?;
    let e = h
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(h.len());
    let v = &h[..e];
    if v.is_empty() || !v.bytes().any(|b| b == b'.') {
        return None;
    }
    Some(v)
}

/// Parse the header and census nodes. `None` without a valid
/// `#Inventor V…` first line or with zero nodes.
pub fn parse(d: &[u8]) -> Option<Iv> {
    let s = core::str::from_utf8(d).ok()?;
    let version = header_version(s)?.to_string();
    let b = s.as_bytes();
    let n = b.len();
    let mut f = Iv {
        version,
        defs: 0,
        uses: 0,
        grouping: 0,
        shapes: 0,
        total_nodes: 0,
    };
    let mut i = 0;
    while i < n {
        match b[i] {
            b'#' => {
                while i < n && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'"' => {
                i += 1;
                while i < n && b[i] != b'"' {
                    i += 1;
                }
            }
            c if c.is_ascii_alphabetic() => {
                let st = i;
                while i < n && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                    i += 1;
                }
                let word = &s[st..i];
                let mut j = i;
                while j < n && b[j].is_ascii_whitespace() {
                    j += 1;
                }
                match word {
                    "DEF" => {
                        f.defs += 1;
                        i = j;
                    }
                    "USE" => {
                        f.uses += 1;
                        i = j;
                    }
                    _ => {
                        if j < n && b[j] == b'{' {
                            f.total_nodes += 1;
                            if GROUPING.contains(&word) {
                                f.grouping += 1;
                            }
                            if SHAPES.contains(&word) {
                                f.shapes += 1;
                            }
                            i = j + 1;
                        } else {
                            i = j;
                        }
                    }
                }
            }
            _ => i += 1,
        }
    }
    if f.total_nodes == 0 {
        return None;
    }
    Some(f)
}

/// `true` when the first bytes start an `#Inventor` file.
pub fn detect(d: &[u8]) -> bool {
    d.starts_with(b"#Inventor")
        || (d.len() > 4
            && d[..4].iter().all(|b| b.is_ascii_whitespace())
            && d[4..].starts_with(b"#Inventor"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b"#Inventor V2\x2e0 ascii\n# comment with Separator {\nSeparator {\n\
        DEF M Material { diffuseColor 1 0 0 }\n  Cube { width 2 }\n  USE M\n}\n";

    #[test]
    fn parses() {
        let f = parse(DOC).unwrap();
        assert_eq!(f.version, "2\x2e0");
        assert_eq!(f.defs, 1);
        assert_eq!(f.uses, 1);
        assert_eq!(f.grouping, 1); // Separator only; comment not counted
        assert_eq!(f.shapes, 1); // Cube
        assert_eq!(f.total_nodes, 3);
    }

    #[test]
    fn versions_and_strings() {
        let f = parse(b"#Inventor V2\x2e1 ascii\nSeparator { Text3 { string \"{\" } }\n").unwrap();
        assert_eq!(f.version, "2\x2e1");
        assert_eq!(f.shapes, 1);
        assert_eq!(f.total_nodes, 2);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"#VRML V2\x2e0 utf8\nGroup{}\n").is_none()); // VRML, not Inventor
        assert!(parse(b"#Inventor V2\x2e0 ascii\nno nodes here\n").is_none());
        assert!(parse(&[0xff]).is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(DOC));
        assert!(!detect(b"#VRML V2\x2e0 utf8"));
    }
}
