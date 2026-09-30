//! Quake `.map` text format — entities as `{` … `}` blocks of
//! `"key" "value"` pairs and nested brush solids, each face a
//! `( x y z ) ( x y z ) ( x y z )` plane definition followed by a
//! texture name.
//!
//! ```
//! let d = b"{\n\"classname\" \"worldspawn\"\n{\n( 0 0 0 ) ( 1 0 0 ) ( 0 1 0 ) TEX 0 0 0 1 1\n}\n}\n";
//! let q = izanagi_kit::qmap::parse(d).unwrap();
//! assert_eq!(q.entities, 1);
//! assert_eq!(q.brushes, 1);
//! assert_eq!(q.planes, 1);
//! assert!(q.worldspawn);
//! assert!(izanagi_kit::qmap::detect(d));
//! ```

/// Census of a Quake `.map` file.
#[derive(Debug, Clone)]
pub struct Qmap {
    /// Top-level `{` entity blocks.
    pub entities: usize,
    /// Nested `{` brush blocks.
    pub brushes: usize,
    /// `( x y z ) ( x y z ) ( x y z )` plane face lines.
    pub planes: usize,
    /// `"key" "value"` property lines.
    pub props: usize,
    /// `"classname" "worldspawn"` present.
    pub worldspawn: bool,
    /// `//` comment lines.
    pub comments: usize,
    /// Deepest brace nesting depth.
    pub max_depth: usize,
}

/// Detects a `.map`: brace blocks plus `classname` or plane lines.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains('{') && t.contains('}') && (t.contains("classname") || t.contains("( "))
}

/// Parses a `.map`; `None` on non-UTF-8 or unbalanced braces.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Qmap> {
    let t = std::str::from_utf8(b).ok()?;
    if !detect(b) {
        return None;
    }
    let mut q = Qmap {
        entities: 0,
        brushes: 0,
        planes: 0,
        props: 0,
        worldspawn: false,
        comments: 0,
        max_depth: 0,
    };
    let mut depth = 0usize;
    let mut saw_entity = false;
    for raw in t.lines() {
        let l = raw.trim();
        match l {
            "{" => {
                depth += 1;
                if depth == 1 {
                    q.entities += 1;
                    saw_entity = true;
                } else {
                    q.brushes += 1;
                }
                if depth > q.max_depth {
                    q.max_depth = depth;
                }
            }
            "}" => {
                if depth == 0 {
                    return None;
                }
                depth -= 1;
            }
            _ => {
                if l.starts_with("//") {
                    q.comments += 1;
                } else if l.starts_with('(') {
                    q.planes += 1;
                } else if l.starts_with('"') {
                    q.props += 1;
                    if l.contains("worldspawn") {
                        q.worldspawn = true;
                    }
                }
            }
        }
    }
    (saw_entity && depth == 0 && q.entities > 0).then_some(q)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"// entity 0\n{\n\"classname\" \"worldspawn\"\n{\n( 0 0 0 ) ( 1 0 0 ) ( 0 1 0 ) TEX 0 0 0 1 1\n( 0 0 1 ) ( 1 0 1 ) ( 0 1 1 ) TEX 0 0 0 1 1\n}\n{\n( 2 0 0 ) ( 3 0 0 ) ( 2 1 0 ) TEX 0 0 0 1 1\n}\n}\n{\n\"classname\" \"light\"\n\"origin\" \"1 1 1\"\n}\n";

    #[test]
    fn parses() {
        let q = parse(D).unwrap();
        assert_eq!(q.entities, 2);
        assert_eq!(q.brushes, 2);
        assert_eq!(q.planes, 3);
        assert_eq!(q.props, 3);
        assert!(q.worldspawn);
        assert_eq!(q.comments, 1);
        assert_eq!(q.max_depth, 2);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"{ }"));
        assert!(!detect(b"plain"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"{\n\"classname\" \"x\"\n").is_none()); // unbalanced
        assert!(parse(b"}\n{\n\"classname\" \"x\"\n}\n").is_none()); // } first
    }
}
