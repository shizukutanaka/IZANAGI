//! Tiled `.tsx` tileset XML — a `<tileset>` root with `name`,
//! `tilewidth`/`tileheight`, `tilecount`/`columns`, plus `<image>`,
//! `<tile>`, `<tileoffset>`, `<grid>`, `<properties>`,
//! `<transformations>`, `<wangsets>` and `<animation>`/`<frame>`
//! children.
//!
//! ```
//! let d = b"<tileset version=\"1\" name=\"grass\" tilewidth=\"16\" tileheight=\"16\" tilecount=\"4\" columns=\"2\">\
//! <image source=\"g.png\" width=\"32\" height=\"32\"/><tile id=\"0\"/></tileset>";
//! let s = izanagi_kit::tsx::parse(d).unwrap();
//! assert_eq!(s.tilewidth, Some(16));
//! assert_eq!(s.tilecount, Some(4));
//! assert_eq!(s.tiles, 1);
//! assert!(izanagi_kit::tsx::detect(d));
//! ```

/// Census of a Tiled `.tsx` tileset.
#[derive(Debug, Clone)]
pub struct Tsx {
    /// `tilewidth` attribute.
    pub tilewidth: Option<u32>,
    /// `tileheight` attribute.
    pub tileheight: Option<u32>,
    /// `tilecount` attribute.
    pub tilecount: Option<u32>,
    /// `columns` attribute.
    pub columns: Option<u32>,
    /// `name` attribute length.
    pub name_len: usize,
    /// `<image>` elements.
    pub images: usize,
    /// `<tile>` entries.
    pub tiles: usize,
    /// `<tileoffset>` present.
    pub tileoffset: bool,
    /// `<wangset>` wang sets.
    pub wangsets: usize,
    /// `<animation>` blocks.
    pub animations: usize,
    /// `<frame>` cells.
    pub frames: usize,
    /// `<properties>` blocks.
    pub properties: usize,
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

/// Detects a `.tsx`: a `<tileset>` element with `tilewidth`/`name`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    tag_region(t, "tileset")
        .map(|r| attr(r, "tilewidth").is_some() || attr(r, "name").is_some())
        .unwrap_or(false)
}

/// Parses a `.tsx`; `None` on non-UTF-8 or a missing `<tileset>` root.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Tsx> {
    let t = std::str::from_utf8(b).ok()?;
    let root = tag_region(t, "tileset")?;
    let s = Tsx {
        tilewidth: attr(root, "tilewidth").and_then(|v| v.parse().ok()),
        tileheight: attr(root, "tileheight").and_then(|v| v.parse().ok()),
        tilecount: attr(root, "tilecount").and_then(|v| v.parse().ok()),
        columns: attr(root, "columns").and_then(|v| v.parse().ok()),
        name_len: attr(root, "name").map_or(0, str::len),
        images: count_tag(t, "image"),
        tiles: count_tag(t, "tile"),
        tileoffset: count_tag(t, "tileoffset") > 0,
        wangsets: count_tag(t, "wangset"),
        animations: count_tag(t, "animation"),
        frames: count_tag(t, "frame"),
        properties: count_tag(t, "properties"),
    };
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"<?xml version=\"1\"?><tileset version=\"1\" name=\"grass\" tilewidth=\"16\" tileheight=\"16\" tilecount=\"4\" columns=\"2\"><tileoffset x=\"1\" y=\"2\"/><image source=\"g.png\" width=\"32\" height=\"32\"/><properties/><tile id=\"0\"><animation><frame tileid=\"0\" duration=\"100\"/><frame tileid=\"1\" duration=\"100\"/></animation></tile><tile id=\"1\"/><tile id=\"2\"/><tile id=\"3\"/><wangsets><wangset name=\"w\"/></wangsets></tileset>";

    #[test]
    fn parses() {
        let s = parse(D).unwrap();
        assert_eq!(s.tilewidth, Some(16));
        assert_eq!(s.tilecount, Some(4));
        assert_eq!(s.columns, Some(2));
        assert_eq!(s.name_len, 5);
        assert_eq!(s.images, 1);
        assert_eq!(s.tiles, 4);
        assert!(s.tileoffset);
        assert_eq!(s.wangsets, 1);
        assert_eq!(s.animations, 1);
        assert_eq!(s.frames, 2);
        assert_eq!(s.properties, 1);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"<map width=\"1\"/>"));
        assert!(!detect(b"plain"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<other/>").is_none());
    }
}
