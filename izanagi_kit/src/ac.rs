//! AC3D `.ac` — AC3D modeller scene/object text.
//!
//! The first line is `AC3D` followed by a single version letter
//! (`b` for modern files). The body is keyword lines: `MATERIAL`,
//! `OBJECT <type>` (`world`, `poly`, `group`, `light`),
//! `numvert`/`numsurf` blocks, `SURF` flags and `refs` polygons.
//!
//! ```
//! let d = b"AC3Db\nMATERIAL \"m\" rgb 1 0 0\nOBJECT world\nkids 1\n\
//! OBJECT poly\nname \"p\"\nnumvert 2\n0 0 0\n1 0 0\nnumsurf 1\n\
//! SURF 0x30\nmat 0\nrefs 2\n0 1\nkids 0\n";
//! let f = izanagi_kit::ac::parse(d).unwrap();
//! assert_eq!(f.version, b'b');
//! assert_eq!(f.objects, 2);
//! assert_eq!(f.polys, 1);
//! assert_eq!(f.vertices, 2);
//! ```
//!
//! Reference: AC3D file format (ac3d.org file format documentation);
//! Blender `io_scene_ac3d` importer. Integer-only.

/// Parsed `.ac` header + object census.
#[derive(Debug, Clone, PartialEq)]
pub struct Ac {
    /// Version letter after `AC3D` (`b` for 0xb).
    pub version: u8,
    /// `MATERIAL` lines.
    pub materials: u32,
    /// `OBJECT` blocks (all types).
    pub objects: u32,
    /// `OBJECT poly` count.
    pub polys: u32,
    /// `OBJECT group` count.
    pub groups: u32,
    /// `OBJECT light` count.
    pub lights: u32,
    /// `SURF` records.
    pub surfaces: u32,
    /// Summed `numvert` vertex counts.
    pub vertices: u32,
    /// `texture` lines.
    pub textures: u32,
}

fn small_u(s: &str) -> u32 {
    let mut v: u32 = 0;
    for &b in s.trim().as_bytes().iter().take(6) {
        if !b.is_ascii_digit() {
            return 0;
        }
        v = v.saturating_mul(10).saturating_add((b - b'0') as u32);
    }
    v
}

/// Parse the header and census object/material/surface keywords.
/// `None` when the `AC3D<ver>` signature is absent.
pub fn parse(d: &[u8]) -> Option<Ac> {
    if d.len() < 6 || !d.starts_with(b"AC3D") {
        return None;
    }
    let version = d[4];
    let s = core::str::from_utf8(d).ok()?;
    let mut f = Ac {
        version,
        materials: 0,
        objects: 0,
        polys: 0,
        groups: 0,
        lights: 0,
        surfaces: 0,
        vertices: 0,
        textures: 0,
    };
    for l in s.lines().map(str::trim) {
        let (kw, rest) = match l.find([' ', '\t']) {
            Some(p) => (&l[..p], l[p..].trim_start()),
            None => (l, ""),
        };
        match kw {
            "MATERIAL" => f.materials += 1,
            "OBJECT" => {
                f.objects += 1;
                match rest.split([' ', '\t']).next().unwrap_or("") {
                    "poly" => f.polys += 1,
                    "group" => f.groups += 1,
                    "light" => f.lights += 1,
                    _ => {}
                }
            }
            "SURF" => f.surfaces += 1,
            "numvert" => f.vertices = f.vertices.saturating_add(small_u(rest)),
            "texture" => f.textures += 1,
            _ => {}
        }
    }
    if f.objects == 0 {
        return None;
    }
    Some(f)
}

/// `true` when the `AC3D` signature is present.
pub fn detect(d: &[u8]) -> bool {
    d.len() >= 5 && d.starts_with(b"AC3D")
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b"AC3Db\nMATERIAL \"m\" rgb 1 0 0\nOBJECT world\nkids 1\n\
        OBJECT poly\nname \"p\"\nnumvert 2\n0 0 0\n1 0 0\nnumsurf 1\n\
        SURF 0x30\nmat 0\nrefs 2\n0 1\nkids 0\n";

    #[test]
    fn parses() {
        let f = parse(DOC).unwrap();
        assert_eq!(f.version, b'b');
        assert_eq!(f.materials, 1);
        assert_eq!(f.objects, 2);
        assert_eq!(f.polys, 1);
        assert_eq!(f.groups, 0);
        assert_eq!(f.lights, 0);
        assert_eq!(f.surfaces, 1);
        assert_eq!(f.vertices, 2);
    }

    #[test]
    fn types_and_textures() {
        let d = b"AC3Dc\nOBJECT group\nkids 1\nOBJECT light\nkids 0\n\
            OBJECT poly\ntexture \"t.png\"\nnumvert 1\n0 0 0\nkids 0\n";
        let f = parse(d).unwrap();
        assert_eq!(f.groups, 1);
        assert_eq!(f.lights, 1);
        assert_eq!(f.polys, 1);
        assert_eq!(f.textures, 1);
        assert_eq!(f.vertices, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"AC3D").is_none());
        assert!(parse(b"OBJ3Db\nOBJECT poly\n").is_none());
        // signature but no OBJECT at all
        assert!(parse(b"AC3Db\nMATERIAL \"m\"\n").is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(DOC));
        assert!(!detect(b"AC3"));
    }
}
