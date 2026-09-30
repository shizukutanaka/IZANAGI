//! GraphML — `<graphml>` XML container with `<key>` declarations, `<graph>`
//! elements, `<node id>` / `<edge source target>` entries, and `<data>` values.
//!
//! ```
//! let d = b"<?xml version=\"1\x2e0\"?>\n<graphml>\n<key id=\"d0\" for=\"node\" attr\x2ename=\"color\" attr\x2etype=\"string\"/>\n<graph id=\"G\" edgedefault=\"directed\">\n<node id=\"n0\"><data key=\"d0\">red</data></node>\n<node id=\"n1\"/>\n<edge source=\"n0\" target=\"n1\"/>\n</graph>\n</graphml>\n";
//! let g = izanagi_kit::graphml::parse(d).unwrap();
//! assert_eq!(g.nodes, 2);
//! assert_eq!(g.edges, 1);
//! assert_eq!(g.edgedefault, "directed");
//! assert!(izanagi_kit::graphml::detect(d));
//! ```

/// A parsed GraphML document census.
#[derive(Debug, Clone)]
pub struct Graphml {
    /// `<graph>` elements.
    pub graphs: usize,
    /// `<node>` elements.
    pub nodes: usize,
    /// `<edge>` elements.
    pub edges: usize,
    /// `<key>` attribute declarations.
    pub keys: usize,
    /// `<data>` attribute values.
    pub datas: usize,
    /// `<hyperedge>` elements.
    pub hyperedges: usize,
    /// `<locator>` elements.
    pub locators: usize,
    /// `port=` attributes.
    pub ports: usize,
    /// First graph's `edgedefault` attribute.
    pub edgedefault: String,
    /// `directed` edges implied by `edgedefault="directed"`.
    pub directed: bool,
}

/// Counts `<name` tag opens where the next byte is whitespace, `>` or `/`.
fn tag_count(t: &str, name: &str) -> usize {
    let mut n = 0usize;
    let pat = name.as_bytes();
    let tb = t.as_bytes();
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

/// Detects a GraphML document: `<graphml` element present.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("<graphml")
}

/// Parses a GraphML document into a census of elements and attributes.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Graphml> {
    let t = std::str::from_utf8(b).ok()?;
    if !detect(b) {
        return None;
    }
    let edgedefault = attr(t, "<graph", "edgedefault").unwrap_or_default();
    let mut ports = 0usize;
    let mut rest = t;
    while let Some(p) = rest.find("port=") {
        ports += 1;
        rest = &rest[p + 5..];
    }
    Some(Graphml {
        graphs: tag_count(t, "graph"),
        nodes: tag_count(t, "node"),
        edges: tag_count(t, "edge"),
        keys: tag_count(t, "key"),
        datas: tag_count(t, "data"),
        hyperedges: tag_count(t, "hyperedge"),
        locators: tag_count(t, "locator"),
        ports,
        directed: edgedefault == "directed",
        edgedefault,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"<graphml>\n<graph id=\"G\" edgedefault=\"undirected\">\n<node id=\"a\"/>\n<node id=\"b\"/>\n<edge source=\"a\" target=\"b\"/>\n</graph>\n</graphml>\n";

    #[test]
    fn detects_graphml() {
        assert!(detect(D));
        assert!(!detect(b"<gexf/>"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses_census() {
        let g = parse(D).unwrap();
        assert_eq!(g.graphs, 1);
        assert_eq!(g.nodes, 2);
        assert_eq!(g.edges, 1);
        assert_eq!(g.edgedefault, "undirected");
        assert!(!g.directed);
    }

    #[test]
    fn tag_boundaries_respected() {
        assert_eq!(tag_count("<node/><nodes/><node x=\"1\"/>", "node"), 2);
        assert_eq!(tag_count("<edge/><edgedefault=\"x\"/><edges/>", "edge"), 1);
    }
}
