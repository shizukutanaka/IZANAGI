//! COLLADA `.dae` — XML digital asset exchange (ISO/PAS 17506).
//!
//! Root element `<COLLADA>` carrying a `version` attribute
//! (`1.4.x` / `1.5.x`) and an `xmlns` schema URI, an `<asset>`
//! block with `<up_axis>`, and `library_*` collections for
//! geometries, animations, images, effects, materials, cameras,
//! lights and controllers.
//!
//! ```
//! let d = b"<COLLADA xmlns='http://www.collada.org/2005/11/COLLADASchema' version='1\x2e4\x2e1'>\
//! <asset><up_axis>Y_UP</up_axis></asset>\
//! <library_geometries><geometry id='g'/></library_geometries>\
//! <library_animations/></COLLADA>";
//! let f = izanagi_kit::dae::parse(d).unwrap();
//! assert_eq!(f.version, "1\x2e4\x2e1");
//! assert_eq!(f.geometries, 1);
//! assert!(f.libraries.iter().any(|l| l == "animations"));
//! ```
//!
//! Reference: ISO/PAS 17506 COLLADA specification (Khronos);
//! collada-dom / assimp loaders. Integer-only.

/// Parsed `.dae` root + element census.
#[derive(Debug, Clone, PartialEq)]
pub struct Dae {
    /// `version` attribute of `<COLLADA>` (e.g. `1.4.1`).
    pub version: String,
    /// Distinct `library_*` element names in file order
    /// (without the `library_` prefix).
    pub libraries: Vec<String>,
    /// `<geometry>` elements.
    pub geometries: u32,
    /// `<animation>` elements.
    pub animations: u32,
    /// `<image>` elements.
    pub images: u32,
    /// `<effect>` elements.
    pub effects: u32,
    /// `<material>` elements.
    pub materials: u32,
    /// `<camera>` elements.
    pub cameras: u32,
    /// `<light>` elements.
    pub lights: u32,
    /// `<controller>` elements.
    pub controllers: u32,
    /// `<scene>` element present.
    pub has_scene: bool,
    /// `<up_axis>` text when present (`X_UP`/`Y_UP`/`Z_UP`).
    pub up_axis: Option<String>,
}

fn attr<'a>(tag: &'a str, key: &str) -> Option<&'a str> {
    for q in ['"', '\''] {
        let pat = [key, "="].concat();
        let mut i = 0;
        while let Some(p) = tag[i..].find(&pat) {
            let st = i + p;
            let after = &tag[st + pat.len()..];
            if after.starts_with(q) {
                let v = &after[1..];
                let e = v.find(q)?;
                return Some(&v[..e]);
            }
            i = st + pat.len();
        }
    }
    None
}

fn count_elem(s: &str, name: &str) -> u32 {
    let pat = ["<", name].concat();
    let mut n = 0;
    let mut i = 0;
    while let Some(p) = s[i..].find(&pat) {
        let j = i + p + pat.len();
        match s[j..].chars().next() {
            Some(c) if c.is_ascii_alphanumeric() || c == '_' || c == '-' => {}
            _ => n += 1,
        }
        i = j;
    }
    n
}

/// Parse the `<COLLADA>` root tag and census library/element kinds.
/// `None` when there is no `<COLLADA>` element with `version`.
pub fn parse(d: &[u8]) -> Option<Dae> {
    let s = core::str::from_utf8(d).ok()?;
    let t = s.find("<COLLADA")?;
    let rest = &s[t + 8..];
    let end = rest.find('>')?;
    let tag = &rest[..end];
    let version = attr(tag, "version")?;
    if version.is_empty() {
        return None;
    }
    let mut libraries = Vec::new();
    let mut i = 0;
    while let Some(p) = s[i..].find("<library_") {
        let st = i + p + 9;
        let e = s[st..]
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .map(|o| st + o)
            .unwrap_or(s.len());
        let name = &s[st..e];
        if !name.is_empty() && !libraries.iter().any(|x: &String| x.as_str() == name) {
            libraries.push(name.to_string());
        }
        i = st;
    }
    let up_axis = s.find("<up_axis>").and_then(|p| {
        let st = p + 9;
        s[st..].find('<').map(|e| s[st..st + e].trim().to_string())
    });
    Some(Dae {
        version: version.to_string(),
        libraries,
        geometries: count_elem(s, "geometry"),
        animations: count_elem(s, "animation"),
        images: count_elem(s, "image"),
        effects: count_elem(s, "effect"),
        materials: count_elem(s, "material"),
        cameras: count_elem(s, "camera"),
        lights: count_elem(s, "light"),
        controllers: count_elem(s, "controller"),
        has_scene: count_elem(s, "scene") > 0,
        up_axis,
    })
}

/// `true` when a `<COLLADA` element start is present.
pub fn detect(d: &[u8]) -> bool {
    d.windows(8).any(|w| w == b"<COLLADA")
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b"<COLLADA xmlns='http://www.collada.org/2005/11/COLLADASchema' \
        version='1\x2e4\x2e1'><asset><up_axis>Z_UP</up_axis></asset>\
        <library_geometries><geometry id='a'/><geometry id='b'/></library_geometries>\
        <library_effects><effect id='e'/></library_effects>\
        <library_materials/><scene><instance_visual_scene/></scene></COLLADA>";

    #[test]
    fn parses() {
        let f = parse(DOC).unwrap();
        assert_eq!(f.version, "1\x2e4\x2e1");
        assert_eq!(f.libraries, vec!["geometries", "effects", "materials"]);
        assert_eq!(f.geometries, 2);
        assert_eq!(f.effects, 1);
        assert_eq!(f.animations, 0);
        assert!(f.has_scene);
        assert_eq!(f.up_axis.as_deref(), Some("Z_UP"));
    }

    #[test]
    fn element_names_do_not_bleed() {
        // `<geometrys>` must not count as `<geometry>`.
        let d = b"<COLLADA version='1\x2e5'><geometrys/><geometry/></COLLADA>";
        assert_eq!(parse(d).unwrap().geometries, 1);
    }

    #[test]
    fn double_quoted_attrs() {
        let d = b"<COLLADA version=\"1\x2e4\x2e1\"></COLLADA>";
        assert_eq!(parse(d).unwrap().version, "1\x2e4\x2e1");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<html><body/></html>").is_none());
        assert!(parse(b"<COLLADA></COLLADA>").is_none()); // no version
        assert!(parse(&[0xff, 0xfe]).is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(DOC));
        assert!(!detect(b"<svg/>"));
    }
}
