//! VRML `.wrl` — Virtual Reality Modeling Language scene text.
//!
//! First line `#VRML V1.0 ascii` (VRML 1.0) or `#VRML V2.0 utf8`
//! (VRML97 / ISO/IEC 14772-1). The body is a node tree with
//! `DEF name` bindings, `USE name` references, grouping nodes and
//! geometry nodes. `#` comments and quoted strings are skipped.
//!
//! ```
//! let d = b"#VRML V2\x2e0 utf8\nDEF S Separator {\n  Transform { translation 0 0 0 }\n\
//!   Shape { appearance Appearance { material Material { } } \
//!   geometry IndexedFaceSet { } }\n}\n";
//! let f = izanagi_kit::wrl::parse(d).unwrap();
//! assert_eq!(f.major, 2);
//! assert_eq!(f.defs, 1);
//! assert_eq!(f.grouping, 2); // Separator + Transform
//! assert_eq!(f.geometry, 1); // IndexedFaceSet
//! ```
//!
//! Reference: ISO/IEC 14772-1 (VRML97); VRML 1.0 specification.
//! Integer-only.

/// Parsed `.wrl` header + node census.
#[derive(Debug, Clone, PartialEq)]
pub struct Wrl {
    /// Major version: 1 or 2.
    pub major: u8,
    /// `DEF name` bindings.
    pub defs: u32,
    /// `USE name` references.
    pub uses: u32,
    /// Grouping nodes: `Separator`, `Group`, `Transform`,
    /// `TransformSeparator`, `Switch`, `LOD`, `Collision`,
    /// `WWWAnchor`, `Anchor`, `Inline`, `Billboard`.
    pub grouping: u32,
    /// Geometry nodes: `IndexedFaceSet`, `IndexedLineSet`,
    /// `PointSet`, `Cube`, `Sphere`, `Cone`, `Cylinder`,
    /// `ElevationGrid`, `Extrusion`, `Text`, `Box`.
    pub geometry: u32,
    /// Appearance/material nodes: `Material`, `Appearance`,
    /// `Texture2`, `ImageTexture`, `PixelTexture`, `MovieTexture`,
    /// `MaterialBinding`, `Texture2Transform`.
    pub appearance: u32,
    /// Total `{}` nodes counted.
    pub total_nodes: u32,
}

const GROUPING: &[&str] = &[
    "Separator",
    "Group",
    "Transform",
    "TransformSeparator",
    "Switch",
    "LOD",
    "Collision",
    "WWWAnchor",
    "Anchor",
    "Inline",
    "Billboard",
];

const GEOMETRY: &[&str] = &[
    "IndexedFaceSet",
    "IndexedLineSet",
    "PointSet",
    "Cube",
    "Box",
    "Sphere",
    "Cone",
    "Cylinder",
    "ElevationGrid",
    "Extrusion",
    "Text",
];

const APPEARANCE: &[&str] = &[
    "Material",
    "Appearance",
    "Texture2",
    "ImageTexture",
    "PixelTexture",
    "MovieTexture",
    "MaterialBinding",
    "Texture2Transform",
];

/// First line must be `#VRML V<major>.<minor> <encoding>`.
fn header_major(s: &str) -> Option<u8> {
    let line = s.lines().next()?.trim_start();
    let h = line.strip_prefix("#VRML")?;
    let h = h.trim_start();
    let h = h.strip_prefix('V')?;
    match h.as_bytes().first() {
        Some(b'1') => Some(1),
        Some(b'2') => Some(2),
        _ => None,
    }
}

/// Parse the header and census nodes. `None` without a valid
/// `#VRML V1|V2` first line or with zero nodes.
pub fn parse(d: &[u8]) -> Option<Wrl> {
    let s = core::str::from_utf8(d).ok()?;
    let major = header_major(s)?;
    let b = s.as_bytes();
    let n = b.len();
    let mut f = Wrl {
        major,
        defs: 0,
        uses: 0,
        grouping: 0,
        geometry: 0,
        appearance: 0,
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
                            if GEOMETRY.contains(&word) {
                                f.geometry += 1;
                            }
                            if APPEARANCE.contains(&word) {
                                f.appearance += 1;
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

/// `true` when the first bytes start a `#VRML` file.
pub fn detect(d: &[u8]) -> bool {
    d.starts_with(b"#VRML")
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] =
        b"#VRML V2\x2e0 utf8\nDEF S Separator {\n  Transform { translation 0 0 0 }\n\
        Shape { appearance Appearance { material Material { } } \
        geometry IndexedFaceSet { } }\n  USE S\n}\n";

    #[test]
    fn parses() {
        let f = parse(DOC).unwrap();
        assert_eq!(f.major, 2);
        assert_eq!(f.defs, 1);
        assert_eq!(f.uses, 1);
        assert_eq!(f.grouping, 2);
        assert_eq!(f.geometry, 1);
        assert_eq!(f.appearance, 2); // Appearance + Material
        assert_eq!(f.total_nodes, 6);
    }

    #[test]
    fn v1_and_comments_strings() {
        let d = b"#VRML V1\x2e0 ascii\n# Separator { not a node\nSeparator {\n\
            Coordinate3 { point [ 0 0 0 ] }\n  WWWAnchor { name \"a{b}\" }\n}\n";
        let f = parse(d).unwrap();
        assert_eq!(f.major, 1);
        assert_eq!(f.grouping, 2); // Separator + WWWAnchor
        assert_eq!(f.total_nodes, 3);
    }

    #[test]
    fn field_names_not_counted() {
        // lowercase field words followed by node names must not count
        let d = b"#VRML V2\x2e0 utf8\nGroup { children [ Shape { geometry Box { } } ] }\n";
        let f = parse(d).unwrap();
        assert_eq!(f.grouping, 1); // Group only
        assert_eq!(f.geometry, 1); // Box
        assert_eq!(f.total_nodes, 3);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"#Inventor V2\x2e0 ascii\nSeparator{}\n").is_none());
        assert!(parse(b"#VRML V3\x2e0 utf8\nGroup{}\n").is_none());
        assert!(parse(b"#VRML V2\x2e0 utf8\nonly words\n").is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(DOC));
        assert!(!detect(b"#Inventor V2\x2e0 ascii"));
    }
}
