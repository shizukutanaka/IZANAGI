//! Unreal Engine text export `.t3d` — `Begin Object`/`End Object`
//! blocks with `Name=`/`Class=`/`Archetype=` assignments, wrapped in
//! `Begin Map`/`End Map` and containing `Begin Actor`/`Begin Brush`
//! nested blocks.
//!
//! ```
//! let d = b"Begin Map\nBegin Actor Class=Light Name=L1\nEnd Actor\n\
//! Begin Object Class=Texture Name=T1\nEnd Object\nEnd Map\n";
//! let t = izanagi_kit::t3d::parse(d).unwrap();
//! assert_eq!(t.objects, 1);
//! assert_eq!(t.actors, 1);
//! assert!(t.map);
//! assert_eq!(t.classes, 2);
//! assert!(izanagi_kit::t3d::detect(d));
//! ```

/// Census of a `.t3d` Unreal text export.
#[derive(Debug, Clone)]
pub struct T3d {
    /// `Begin Map` block present.
    pub map: bool,
    /// `Begin Object`/`End Object` pairs.
    pub objects: usize,
    /// `Begin Actor` blocks.
    pub actors: usize,
    /// `Begin Brush` blocks.
    pub brushes: usize,
    /// `Begin PolyList`/`Begin Terrain`/`Begin Item`-style other blocks.
    pub other_begins: usize,
    /// `Class=` assignments.
    pub classes: usize,
    /// `Name=` assignments.
    pub names: usize,
    /// `Archetype=` references.
    pub archetypes: usize,
    /// `Group=` assignments.
    pub groups: usize,
    /// `Begin Surface` blocks.
    pub surfaces: usize,
}

const KNOWN: &[&str] = &[
    "Object", "Actor", "Brush", "Map", "Surface", "PolyList", "Item",
];

/// Detects `.t3d`: a `Begin Map`/`Begin Object`/`Begin Actor` banner.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("Begin Map") || t.contains("Begin Object") || t.contains("Begin Actor"))
        && (t.contains("End") || t.contains("Class="))
}

/// Parses a `.t3d`; `None` on non-UTF-8 or no `Begin` blocks.
#[must_use]
pub fn parse(b: &[u8]) -> Option<T3d> {
    let t = std::str::from_utf8(b).ok()?;
    if !detect(b) {
        return None;
    }
    let mut r = T3d {
        map: t.contains("Begin Map"),
        objects: t.matches("Begin Object").count(),
        actors: t.matches("Begin Actor").count(),
        brushes: t.matches("Begin Brush").count(),
        other_begins: 0,
        classes: t.matches("Class=").count(),
        names: t.matches("Name=").count(),
        archetypes: t.matches("Archetype=").count(),
        groups: t.matches("Group=").count(),
        surfaces: t.matches("Begin Surface").count(),
    };
    for raw in t.lines() {
        let l = raw.trim_start();
        if let Some(rest) = l.strip_prefix("Begin ") {
            let kind = rest
                .split(|c: char| c.is_whitespace() || c == '(')
                .next()
                .unwrap_or("");
            if !kind.is_empty() && !KNOWN.contains(&kind) {
                r.other_begins += 1;
            }
        }
    }
    (r.objects > 0 || r.actors > 0 || r.map || r.surfaces > 0).then_some(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"Begin Map\nBegin Actor Class=Light Name=L1 Group=G\nEnd Actor\nBegin Brush Name=B1\nEnd Brush\nBegin Surface\nEnd Surface\nBegin Object Class=Texture Name=T1 Archetype=TA\nEnd Object\nBegin Object Class=Mesh Name=M1\nEnd Object\nBegin CustomBlock\nEnd CustomBlock\nEnd Map\n";

    #[test]
    fn parses() {
        let t = parse(D).unwrap();
        assert!(t.map);
        assert_eq!(t.objects, 2);
        assert_eq!(t.actors, 1);
        assert_eq!(t.brushes, 1);
        assert_eq!(t.surfaces, 1);
        assert_eq!(t.other_begins, 1);
        assert_eq!(t.classes, 3);
        assert_eq!(t.names, 4);
        assert_eq!(t.archetypes, 1);
        assert_eq!(t.groups, 1);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"Begin"));
        assert!(!detect(b"plain"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"Begin Foo\nEnd Foo").is_none()); // unknown kind only
    }
}
