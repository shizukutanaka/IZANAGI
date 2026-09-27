//! Minimal reader for X3D (XML encoding, ISO/IEC 19776): `<X3D
//! profile="...">` root, `<Scene>` content — this reports the profile,
//! and counts common nodes (`<Shape>`, `<Transform>`, `<Viewpoint>`).
//! Self-contained tag scanning.
//!
//! ```
//! use izanagi_kit::x3d::parse;
//!
//! let x = parse(
//!     b"<X3D profile=\"Immersive\"><Scene><Transform><Shape/><Shape/></Transform>\
//!        <Viewpoint/></Scene></X3D>",
//! )
//! .unwrap();
//! assert_eq!(x.profile.as_deref(), Some("Immersive"));
//! assert_eq!(x.count("Shape"), 2);
//! ```

/// A parsed X3D document.
#[derive(Debug)]
pub struct X3d {
    /// `profile` attribute of `<X3D>` (`Immersive`, `Interchange`, ...).
    pub profile: Option<String>,
    /// `version` attribute of `<X3D>`.
    pub version: Option<String>,
    /// `<Scene>` present.
    pub has_scene: bool,
    /// `(element-name, count)` tally for every element seen under
    /// `<Scene>` (including nested).
    pub nodes: Vec<(String, usize)>,
}

impl X3d {
    /// Count of an element name under `<Scene>` (e.g. `"Shape"`).
    pub fn count(&self, name: &str) -> usize {
        self.nodes
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, c)| *c)
            .unwrap_or(0)
    }
}

fn attr(tag: &str, key: &str) -> Option<String> {
    let pat = format!("{key}=\"");
    let at = tag.find(&pat)? + pat.len();
    let end = tag[at..].find('"')? + at;
    Some(tag[at..end].to_string())
}

fn name_ok(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b':' || b == b'.'
}

/// Parse an X3D document. `None` without an `<X3D` root.
pub fn parse(data: &[u8]) -> Option<X3d> {
    let src = std::str::from_utf8(data).ok()?;
    let a = src.find("<X3D")?;
    // `<X3D` vs `<X3Dxxx`: next byte must end the name
    let nb = src.as_bytes().get(a + 4).copied();
    if matches!(nb, Some(b) if name_ok(b)) {
        return None;
    }
    let head_end = src[a..].find('>')? + a;
    let head = &src[a..head_end + 1];
    let scene = src.find("<Scene").and_then(|s| {
        // boundary check on the element name
        let nb = src.as_bytes().get(s + 6).copied();
        if matches!(nb, Some(b) if name_ok(b)) {
            return None;
        }
        let e = src[s..].find("</Scene>")? + s;
        Some(&src[s..e])
    });
    let mut nodes: Vec<(String, usize)> = Vec::new();
    if let Some(body) = scene {
        let mut rest = body;
        while let Some(a) = rest.find('<') {
            if rest.as_bytes().get(a + 1) == Some(&b'/') {
                rest = &rest[a + 1..];
                continue;
            }
            let mut i = a + 1;
            while i < rest.len() && name_ok(rest.as_bytes()[i]) {
                i += 1;
            }
            if i == a + 1 {
                rest = &rest[a + 1..];
                continue;
            }
            let name = &rest[a + 1..i];
            if let Some((_, c)) = nodes.iter_mut().find(|(n, _)| *n == name) {
                *c += 1;
            } else {
                nodes.push((name.to_string(), 1));
            }
            rest = &rest[i..];
        }
    }
    Some(X3d {
        profile: attr(head, "profile"),
        version: attr(head, "version"),
        has_scene: scene.is_some(),
        nodes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b"<X3D profile=\"Immersive\" version=\"3.3\"><Scene>\
<Transform><Shape><Box/></Shape><Shape/></Transform>\
<Group/><Viewpoint/></Scene></X3D>";

    #[test]
    fn parses() {
        let x = parse(DOC).unwrap();
        assert_eq!(x.profile.as_deref(), Some("Immersive"));
        assert_eq!(x.version.as_deref(), Some("3.3"));
        assert!(x.has_scene);
        assert_eq!(x.count("Shape"), 2);
        assert_eq!(x.count("Box"), 1);
        assert_eq!(x.count("Viewpoint"), 1);
        assert_eq!(x.count("Nope"), 0);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<html/>").is_none());
        assert!(parse(b"<X3Dfoo/>").is_none()); // name must end at boundary
        let x = parse(b"<X3D/>").unwrap();
        assert!(!x.has_scene);
    }
}
