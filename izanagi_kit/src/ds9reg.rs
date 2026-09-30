//! SAOImage DS9 region (`*.reg`) census.
//!
//! `global` defaults, a coordinate-system line (`fk5`, `icrs`, `j2000`,
//! `galactic`, `ecliptic`, `image`, `physical`, `linear`, `wcs`), `#`
//! comments, then `shape(args)` region lines.
//!
//! ```
//! let s = b"# Region file\nfk5\nglobal color=green dashlist=8 3\ncircle(12,34,10) # text={a}\nbox(12,34,5,6,0)\npoint(12,34) # point=x\n";
//! assert!(izanagi_kit::ds9reg::detect(s));
//! let r = izanagi_kit::ds9reg::Ds9reg::parse(s).unwrap();
//! assert_eq!(r.shapes, 3);
//! assert_eq!(r.circles, 1);
//! assert_eq!(r.boxes, 1);
//! assert_eq!(r.points, 1);
//! assert_eq!(r.globals, 1);
//! ```

/// Parsed census of a DS9 region file.
#[derive(Debug, Clone)]
pub struct Ds9reg {
    /// `global` default lines.
    pub globals: usize,
    /// Coordinate-system lines.
    pub coord_systems: usize,
    /// `#` comment lines.
    pub comments: usize,
    /// Total `shape(...)` region lines.
    pub shapes: usize,
    /// `circle(` regions.
    pub circles: usize,
    /// `annulus(` regions.
    pub annulus: usize,
    /// `ellipse(` regions.
    pub ellipses: usize,
    /// `box(` regions.
    pub boxes: usize,
    /// `polygon(` regions.
    pub polygons: usize,
    /// `point(` regions.
    pub points: usize,
    /// `line(` regions.
    pub lines: usize,
    /// `vector(` regions.
    pub vectors: usize,
    /// `text(` regions.
    pub texts: usize,
    /// `compass(` regions.
    pub compasses: usize,
    /// `ruler(` regions.
    pub rulers: usize,
    /// `projection(` regions.
    pub projections: usize,
    /// `segment(` regions.
    pub segments: usize,
    /// `panda(`/`epanda(`/`bpanda(` regions.
    pub pandas: usize,
    /// `diamond(`/`noline(`/`boxcircle(` regions.
    pub misc: usize,
    /// Trailing `# attr` region attributes.
    pub attributes: usize,
    /// `fk5`/`j2000`/`icrs` equatorial lines.
    pub equatorial: usize,
    /// `image`/`physical`/`linear`/`wcs` lines.
    pub local_systems: usize,
}

const SHAPES: &[&str] = &[
    "circle",
    "annulus",
    "ellipse",
    "box",
    "polygon",
    "point",
    "line",
    "vector",
    "text",
    "compass",
    "ruler",
    "projection",
    "segment",
    "panda",
    "epanda",
    "bpanda",
    "diamond",
    "noline",
    "boxcircle",
];

const COORDS: &[&str] = &[
    "fk5",
    "j2000",
    "b1950",
    "icrs",
    "galactic",
    "ecliptic",
    "image",
    "physical",
    "linear",
    "amplifier",
    "detector",
];

/// Reports whether `b` looks like a DS9 region file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = core::str::from_utf8(b) else {
        return false;
    };
    t.lines().any(|l| {
        let l = l.trim();
        let lower = l.to_lowercase();
        let w = lower
            .split_whitespace()
            .next()
            .unwrap_or("")
            .trim_start_matches('-');
        COORDS.contains(&w)
            || w.starts_with("wcs")
            || l.starts_with("global ")
            || SHAPES
                .iter()
                .any(|s| lower.starts_with(s) && lower[s.len()..].starts_with('('))
    })
}

impl Ds9reg {
    /// Parses `b` as a DS9 region file, returning `None` when the shape fails.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = core::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let mut r = Ds9reg {
            globals: 0,
            coord_systems: 0,
            comments: 0,
            shapes: 0,
            circles: 0,
            annulus: 0,
            ellipses: 0,
            boxes: 0,
            polygons: 0,
            points: 0,
            lines: 0,
            vectors: 0,
            texts: 0,
            compasses: 0,
            rulers: 0,
            projections: 0,
            segments: 0,
            pandas: 0,
            misc: 0,
            attributes: 0,
            equatorial: 0,
            local_systems: 0,
        };
        for l in t.lines() {
            let l = l.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with('#') {
                r.comments += 1;
                continue;
            }
            if let Some((pre, post)) = l.split_once('#') {
                if !post.trim().is_empty() {
                    r.attributes += 1;
                }
                let _ = pre;
            }
            let lower = l.to_lowercase();
            if lower.starts_with("global ") {
                r.globals += 1;
                continue;
            }
            let w = lower
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_start_matches('-');
            if COORDS.contains(&w)
                || (w.starts_with("wcs") && w.chars().all(|c| c.is_ascii_alphabetic()))
            {
                r.coord_systems += 1;
                if matches!(
                    w,
                    "fk5" | "j2000" | "b1950" | "icrs" | "galactic" | "ecliptic"
                ) {
                    r.equatorial += 1;
                } else {
                    r.local_systems += 1;
                }
                continue;
            }
            if let Some(p) = lower.find('(') {
                let name = &lower[..p];
                if SHAPES.contains(&name) {
                    r.shapes += 1;
                    match name {
                        "circle" => r.circles += 1,
                        "annulus" => r.annulus += 1,
                        "ellipse" => r.ellipses += 1,
                        "box" => r.boxes += 1,
                        "polygon" => r.polygons += 1,
                        "point" => r.points += 1,
                        "line" => r.lines += 1,
                        "vector" => r.vectors += 1,
                        "text" => r.texts += 1,
                        "compass" => r.compasses += 1,
                        "ruler" => r.rulers += 1,
                        "projection" => r.projections += 1,
                        "segment" => r.segments += 1,
                        "panda" | "epanda" | "bpanda" => r.pandas += 1,
                        _ => r.misc += 1,
                    }
                }
            }
        }
        Some(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: &[u8] = b"# Region file\nfk5\nglobal color=green dashlist=8 3\ncircle(12,34,10) # text={a}\nbox(12,34,5,6,0)\npoint(12,34) # point=x\n";

    #[test]
    fn parses_ds9reg() {
        assert!(detect(S));
        let r = Ds9reg::parse(S).unwrap();
        assert_eq!(r.shapes, 3);
        assert_eq!(r.circles, 1);
        assert_eq!(r.boxes, 1);
        assert_eq!(r.points, 1);
        assert_eq!(r.globals, 1);
    }

    #[test]
    fn rejects_non_ds9reg() {
        assert!(!detect(b"just text"));
        assert!(Ds9reg::parse(b"{}").is_none());
    }
}
