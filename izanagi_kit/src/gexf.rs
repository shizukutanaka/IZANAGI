//! GEXF (Graph Exchange XML Format) — `<gexf xmlns… version>` document with
//! `<meta>`, `<attributes>`/`class` declarations, `<nodes>`/`<node id label>`
//! and `<edges>`/`<edge id source target>` sections.
//!
//! ```
//! let d = b"<?xml version=\"1\x2e0\"?>\n<gexf xmlns=\"http://www\x2egexf\x2enet/1\x2e2draft\" version=\"1\x2e2\">\n<graph mode=\"static\" defaultedgetype=\"directed\">\n<attributes class=\"node\"><attribute id=\"0\" title=\"rank\" type=\"integer\"/></attributes>\n<nodes>\n<node id=\"0\" label=\"a\"/>\n<node id=\"1\" label=\"b\"/>\n</nodes>\n<edges>\n<edge id=\"0\" source=\"0\" target=\"1\" weight=\"2\"/>\n</edges>\n</graph>\n</gexf>\n";
//! let g = izanagi_kit::gexf::parse(d).unwrap();
//! assert_eq!(g.nodes, 2);
//! assert_eq!(g.edges, 1);
//! assert_eq!(g.version, "1.2");
//! assert!(izanagi_kit::gexf::detect(d));
//! ```

/// A parsed GEXF document census.
#[derive(Debug, Clone)]
pub struct Gexf {
    /// `<gexf>` `version` attribute.
    pub version: String,
    /// `<graph>` `mode` attribute (`static`/`dynamic`).
    pub mode: String,
    /// `<graph>` `defaultedgetype` attribute.
    pub defaultedgetype: String,
    /// `<meta>` blocks.
    pub metas: usize,
    /// `<attributes>` sections.
    pub attribute_sections: usize,
    /// `<attribute>` declarations.
    pub attributes: usize,
    /// `<attvalue>` entries.
    pub attvalues: usize,
    /// `<node>` elements.
    pub nodes: usize,
    /// `<edge>` elements.
    pub edges: usize,
    /// `<weight>`/`weight=` weight carriers.
    pub weights: usize,
    /// `spells`/`spell` dynamic slices.
    pub spells: usize,
}

/// Counts `<name` tag opens where the next byte is whitespace, `>` or `/`.
fn tag_count(t: &str, name: &str) -> usize {
    let pat = name.as_bytes();
    let tb = t.as_bytes();
    let mut n = 0usize;
    let mut i = 0usize;
    while i + 1 + pat.len() <= tb.len() {
        if tb[i] == b'<' && &tb[i + 1..i + 1 + pat.len()] == pat {
            let next = tb.get(i + 1 + pat.len()).copied().unwrap_or(b'>');
            if next == b' '
                || next == b'\t'
                || next == b'\r'
                || next == b'\n'
                || next == b'>'
                || next == b'/'
            {
                n += 1;
                i += 1 + pat.len();
                continue;
            }
        }
        i += 1;
    }
    n
}

/// Finds the first occurrence of `<name` whose next byte is a boundary.
fn find_tag(t: &str, anchor: &str) -> Option<usize> {
    let pat = anchor.as_bytes();
    let tb = t.as_bytes();
    let mut i = 0usize;
    while i + pat.len() <= tb.len() {
        if &tb[i..i + pat.len()] == pat {
            let next = tb.get(i + pat.len()).copied().unwrap_or(b'>');
            if next == b' '
                || next == b'\t'
                || next == b'\r'
                || next == b'\n'
                || next == b'>'
                || next == b'/'
            {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

/// Extracts the `name="…"` attribute value nearest after `anchor`.
fn attr(t: &str, anchor: &str, name: &str) -> Option<String> {
    let a = find_tag(t, anchor)?;
    let seg = &t[a..];
    let end = seg.find('>').unwrap_or(seg.len());
    let seg = &seg[..end];
    let key = seg.find(name)?;
    let rest = &seg[key + name.len()..];
    let rest = rest.trim_start();
    let rest = rest.strip_prefix('=')?;
    let rest = rest.trim_start();
    let q = rest.chars().next()?;
    if q != '"' && q != '\'' {
        return None;
    }
    let rest = &rest[1..];
    let e = rest.find(q)?;
    Some(rest[..e].to_string())
}

/// Detects a GEXF document: `<gexf` element present.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("<gexf")
}

/// Parses a GEXF document into a census of sections and elements.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Gexf> {
    let t = std::str::from_utf8(b).ok()?;
    if !detect(b) {
        return None;
    }
    let mut weights = tag_count(t, "weight");
    let mut rest = t;
    while let Some(p) = rest.find("weight=") {
        weights += 1;
        rest = &rest[p + 7..];
    }
    Some(Gexf {
        version: attr(t, "<gexf", "version").unwrap_or_default(),
        mode: attr(t, "<graph", "mode").unwrap_or_default(),
        defaultedgetype: attr(t, "<graph", "defaultedgetype").unwrap_or_default(),
        metas: tag_count(t, "meta"),
        attribute_sections: tag_count(t, "attributes"),
        attributes: tag_count(t, "attribute"),
        attvalues: tag_count(t, "attvalue"),
        nodes: tag_count(t, "node"),
        edges: tag_count(t, "edge"),
        weights,
        spells: tag_count(t, "spell"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"<gexf version=\"1\x2e3\">\n<graph defaultedgetype=\"undirected\">\n<nodes><node id=\"0\"/><node id=\"1\"/></nodes>\n<edges><edge id=\"0\" source=\"0\" target=\"1\"/></edges>\n</graph>\n</gexf>\n";

    #[test]
    fn detects_gexf() {
        assert!(detect(D));
        assert!(!detect(b"<graphml/>"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses_census() {
        let g = parse(D).unwrap();
        assert_eq!(g.nodes, 2);
        assert_eq!(g.edges, 1);
        assert_eq!(g.version, "1.3");
        assert_eq!(g.defaultedgetype, "undirected");
    }
}
