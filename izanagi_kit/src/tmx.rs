//! Tiled `.tmx` map XML — a `<map>` root carrying `orientation`,
//! `width`/`height`, `tilewidth`/`tileheight`, then `<tileset>`,
//! `<layer>`, `<objectgroup>`, `<imagelayer>` and `<group>` children
//! with `<data>` payloads (`csv`/`base64`/inline-XML `encoding`).
//!
//! ```
//! let d = b"<map orientation=\"ortho\" width=\"2\" height=\"2\" tilewidth=\"16\" tileheight=\"16\">\
//! <tileset firstgid=\"1\" source=\"x.tsx\"/><layer name=\"g\" width=\"2\" height=\"2\">\
//! <data encoding=\"csv\">1,2,3,4</data></layer><objectgroup name=\"o\"/></map>";
//! let m = izanagi_kit::tmx::parse(d).unwrap();
//! assert_eq!(m.width, Some(2));
//! assert_eq!(m.layers, 1);
//! assert_eq!(m.objectgroups, 1);
//! assert_eq!(m.csv_data, 1);
//! assert!(izanagi_kit::tmx::detect(d));
//! ```

/// Census of a Tiled `.tmx` map.
#[derive(Debug, Clone)]
pub struct Tmx {
    /// `width` attribute.
    pub width: Option<u32>,
    /// `height` attribute.
    pub height: Option<u32>,
    /// `tilewidth` attribute.
    pub tilewidth: Option<u32>,
    /// `tileheight` attribute.
    pub tileheight: Option<u32>,
    /// Length of the `orientation` value (orthogonal/isometric/…).
    pub orientation_len: usize,
    /// `<tileset>` elements.
    pub tilesets: usize,
    /// `<layer>` tile layers.
    pub layers: usize,
    /// `<objectgroup>` elements.
    pub objectgroups: usize,
    /// `<imagelayer>` elements.
    pub imagelayers: usize,
    /// `<group>` nested groups.
    pub groups: usize,
    /// `<data encoding="csv">` payloads.
    pub csv_data: usize,
    /// `<data encoding="base64">` payloads.
    pub base64_data: usize,
    /// `<object>` shapes.
    pub objects: usize,
}

fn has_tag(t: &str, name: &str) -> bool {
    count_tag(t, name) > 0
}

fn count_tag(t: &str, name: &str) -> usize {
    let needle = format!("<{name}");
    let mut n = 0;
    let mut at = 0;
    while let Some(p) = t[at..].find(&needle) {
        let end = at + p + needle.len();
        let ok = t[end..].chars().next().map_or(true, |c| {
            !(c.is_ascii_alphanumeric() || c == '_' || c == '-')
        });
        if ok {
            n += 1;
        }
        at = end;
    }
    n
}

fn attr<'a>(tag: &'a str, k: &str) -> Option<&'a str> {
    let mut from = 0;
    let at = loop {
        let p = tag.get(from..)?.find(k)? + from;
        let bounded = tag[..p]
            .chars()
            .last()
            .is_some_and(|c| c == ' ' || c == '\t' || c == '\n');
        if bounded {
            break p + k.len();
        }
        from = p + k.len();
    };
    let r = tag.get(at..)?.trim_start();
    let r = r.strip_prefix('=')?.trim_start();
    let q = r.chars().next()?;
    if q != '"' && q != '\'' {
        return None;
    }
    let r = &r[1..];
    let e = r.find(q)?;
    Some(&r[..e])
}

fn tag_region<'a>(t: &'a str, name: &str) -> Option<&'a str> {
    let needle = format!("<{name}");
    let at = t.find(&needle)?;
    let end = t[at..].find('>').map(|e| at + e + 1)?;
    t.get(at..end)
}

/// Detects a `.tmx`: a `<map>` element with `tilewidth`/`height` geometry.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    has_tag(t, "map")
        && tag_region(t, "map")
            .map(|r| attr(r, "tilewidth").is_some() || attr(r, "width").is_some())
            .unwrap_or(false)
}

/// Parses a `.tmx`; `None` on non-UTF-8 or a missing `<map>` root.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Tmx> {
    let t = std::str::from_utf8(b).ok()?;
    let root = tag_region(t, "map")?;
    let mut m = Tmx {
        width: attr(root, "width").and_then(|v| v.parse().ok()),
        height: attr(root, "height").and_then(|v| v.parse().ok()),
        tilewidth: attr(root, "tilewidth").and_then(|v| v.parse().ok()),
        tileheight: attr(root, "tileheight").and_then(|v| v.parse().ok()),
        orientation_len: attr(root, "orientation").map_or(0, str::len),
        tilesets: 0,
        layers: 0,
        objectgroups: 0,
        imagelayers: 0,
        groups: 0,
        csv_data: 0,
        base64_data: 0,
        objects: 0,
    };
    m.tilesets = count_tag(t, "tileset");
    m.layers = count_tag(t, "layer");
    m.objectgroups = count_tag(t, "objectgroup");
    m.imagelayers = count_tag(t, "imagelayer");
    m.groups = count_tag(t, "group");
    m.objects = count_tag(t, "object");
    let mut at = 0;
    while let Some(p) = t[at..].find("<data") {
        let tag_end = t[at + p..].find('>').map(|e| at + p + e + 1)?;
        let tag = &t[at + p..tag_end];
        match attr(tag, "encoding") {
            Some("csv") => m.csv_data += 1,
            Some("base64") => m.base64_data += 1,
            _ => {}
        }
        at = tag_end;
    }
    (m.width.is_some() || m.tilewidth.is_some()).then_some(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"<map orientation=\"orthogonal\" renderorder=\"rd\" width=\"3\" height=\"2\" tilewidth=\"8\" tileheight=\"8\"><properties/><tileset firstgid=\"1\" source=\"a.tsx\"/><tileset firstgid=\"9\" source=\"b.tsx\"/><layer name=\"bg\" width=\"3\" height=\"2\"><data encoding=\"csv\">1,2,3,4,5,6</data></layer><imagelayer name=\"img\"/><objectgroup name=\"obj\"><object id=\"1\"/></objectgroup><group><layer name=\"in\" width=\"3\" height=\"2\"><data encoding=\"base64\">AAAA</data></layer></group></map>";

    #[test]
    fn parses() {
        let m = parse(D).unwrap();
        assert_eq!(m.width, Some(3));
        assert_eq!(m.height, Some(2));
        assert_eq!(m.tilewidth, Some(8));
        assert_eq!(m.orientation_len, "orthogonal".len());
        assert_eq!(m.tilesets, 2);
        assert_eq!(m.layers, 2);
        assert_eq!(m.objectgroups, 1);
        assert_eq!(m.imagelayers, 1);
        assert_eq!(m.groups, 1);
        assert_eq!(m.csv_data, 1);
        assert_eq!(m.base64_data, 1);
        assert_eq!(m.objects, 1);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"<map/>"));
        assert!(!detect(b"plain"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<other tilewidth=\"1\"/>").is_none());
    }
}
