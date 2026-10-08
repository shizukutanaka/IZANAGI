//! Studiomdl Data `.smd` — ASCII model/skeleton format: `version 1`,
//! `nodes` bone list (`id "name" parent`), `skeleton` frames
//! (`time N` + `id x y z rx ry rz`), `triangles` (material name then
//! 3 vertex lines) each closed by `end`.
//!
//! ```
//! let d = b"version 1\nnodes\n0 \"root\" -1\nend\nskeleton\ntime 0\n0 0 0 0 0 0 0\nend\n\
//! triangles\nm.bmp\n0 0 0 0 0 0 1 0 0\n0 1 0 0 0 0 1 0 0\n0 0 1 0 0 0 1 0 0\nend\n";
//! let s = izanagi_kit::smd::parse(d).unwrap();
//! assert_eq!(s.version, Some(1));
//! assert_eq!(s.nodes, 1);
//! assert_eq!(s.frames, 1);
//! assert_eq!(s.triangles, 1);
//! assert!(izanagi_kit::smd::detect(d));
//! ```

/// Census of an `.smd` file.
#[derive(Debug, Clone)]
pub struct Smd {
    /// `version N` value.
    pub version: Option<u32>,
    /// Bone lines inside `nodes`.
    pub nodes: usize,
    /// `time N` frame markers.
    pub frames: usize,
    /// Material-name lines inside `triangles` (one per triangle).
    pub triangles: usize,
    /// Vertex lines inside `triangles`.
    pub vertices: usize,
    /// `end` block terminators.
    pub ends: usize,
}

#[derive(Clone, Copy, PartialEq)]
enum Sec {
    None,
    Nodes,
    Skeleton,
    Triangles,
}
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// Detects `.smd`: `version` header plus a `nodes`/`skeleton`/`triangles` block.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    t.lines().any(|l| l.trim_start().starts_with("version"))
        && (t.contains("nodes") || t.contains("skeleton") || t.contains("triangles"))
}

/// Parses an `.smd`; `None` on non-UTF-8 or missing sections.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Smd> {
    let t = std::str::from_utf8(b).ok()?;
    let t = strip_bom(t);
    if !detect(b) {
        return None;
    }
    let mut s = Smd {
        version: None,
        nodes: 0,
        frames: 0,
        triangles: 0,
        vertices: 0,
        ends: 0,
    };
    let mut sec = Sec::None;
    let mut tri_first = true;
    for raw in t.lines() {
        let l = raw.trim();
        match l {
            "nodes" => {
                sec = Sec::Nodes;
                continue;
            }
            "skeleton" => {
                sec = Sec::Skeleton;
                continue;
            }
            "triangles" => {
                sec = Sec::Triangles;
                tri_first = true;
                continue;
            }
            "end" => {
                s.ends += 1;
                sec = Sec::None;
                continue;
            }
            _ => {}
        }
        if let Some(rest) = l.strip_prefix("version") {
            s.version = rest.trim().parse().ok();
            continue;
        }
        match sec {
            Sec::Nodes => s.nodes += 1,
            Sec::Skeleton => {
                if l.starts_with("time") {
                    s.frames += 1;
                }
            }
            Sec::Triangles => {
                if tri_first {
                    s.triangles += 1;
                    tri_first = false;
                } else {
                    s.vertices += 1;
                    if s.vertices % 3 == 0 {
                        tri_first = true;
                    }
                }
            }
            _ => {}
        }
    }
    (s.ends > 0).then_some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"version 1\nnodes\n0 \"root\" -1\n1 \"child\" 0\nend\nskeleton\ntime 0\n0 0 0 0 0 0 0\ntime 1\n0 0 0 0 0 0 0\nend\ntriangles\nm.bmp\n0 0 0 0 0 0 1 0 0\n0 1 0 0 0 0 1 0 0\n0 0 1 0 0 0 1 0 0\nn.bmp\n0 0 0 0 0 0 1 0 0\n0 1 0 0 0 0 1 0 0\n0 0 1 0 0 0 1 0 0\nend\n";

    #[test]
    fn parses() {
        let s = parse(D).unwrap();
        assert_eq!(s.version, Some(1));
        assert_eq!(s.nodes, 2);
        assert_eq!(s.frames, 2);
        assert_eq!(s.triangles, 2);
        assert_eq!(s.vertices, 6);
        assert_eq!(s.ends, 3);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"version 1\n"));
        assert!(!detect(b"nodes\nend"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"version 1\nnodes\n").is_none()); // no end
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
