//! Visual Studio Directed Graph Markup Language (`.dgml`).
//!
//! DGML files are XML documents rooted at `<DirectedGraph` containing
//! `<Nodes>`/`<Links>`/`<Categories>`/`<Properties>` sections with `<Node`/
//! `<Link`/`<Category`/`<CategoryRef`/`<Style`/`<Property` elements.
//!
//! ```
//! let b = concat!(
//!     "<DirectedGraph xmlns=\"http://schemas.microsoft.com/vs/2009/dgml\">\n",
//!     "  <Nodes><Node Id=\"a\" Label=\"A\"/><Node Id=\"b\"/></Nodes>\n",
//!     "  <Links><Link Source=\"a\" Target=\"b\"/></Links>\n",
//!     "  <Categories><Category Id=\"c\"/></Categories>\n",
//!     "</DirectedGraph>\n"
//! ).as_bytes();
//! assert!(izanagi_kit::dgml::detect(b));
//! let c = izanagi_kit::dgml::Dgml::parse(b).unwrap();
//! assert_eq!(c.nodes, 2);
//! assert_eq!(c.links, 1);
//! ```

/// Parsed DGML summary.
#[derive(Debug, Clone)]
pub struct Dgml {
    /// `<Node` elements.
    pub nodes: usize,
    /// `<Link` elements.
    pub links: usize,
    /// `<Category` elements.
    pub categories: usize,
    /// `<Property` elements.
    pub properties: usize,
    /// `<Style` elements.
    pub styles: usize,
    /// `<Graph`/`Group`/`GroupLabel` nested graph elements.
    pub groups: usize,
}

fn count(t: &str, pat: &str) -> usize {
    if pat.is_empty() {
        return 0;
    }
    let mut n = 0;
    let mut off = 0;
    while off + pat.len() <= t.len() {
        match t[off..].find(pat) {
            Some(p) => {
                n += 1;
                off += p + pat.len();
            }
            None => break,
        }
    }
    n
}

/// Whether the buffer looks like DGML.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("<DirectedGraph") || (t.contains("<Nodes>") && t.contains("<Links>"))
}

impl Dgml {
    /// Parses a DGML summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let elem = |name: &str| count(t, &format!("<{name} ")) + count(t, &format!("<{name}/"));
        Some(Self {
            nodes: elem("Node"),
            links: elem("Link"),
            categories: elem("Category"),
            properties: elem("Property"),
            styles: elem("Style"),
            groups: elem("Group"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_dgml() {
        let b = concat!(
            "<DirectedGraph xmlns=\"http://schemas.microsoft.com/vs/2009/dgml\">\n",
            "  <Nodes><Node Id=\"a\" Label=\"A\"/><Node Id=\"b\"/></Nodes>\n",
            "  <Links><Link Source=\"a\" Target=\"b\"/></Links>\n",
            "  <Categories><Category Id=\"c\"/><Category Id=\"d\"/></Categories>\n",
            "</DirectedGraph>\n"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Dgml::parse(b).unwrap();
        assert_eq!(c.nodes, 2);
        assert_eq!(c.links, 1);
        assert_eq!(c.categories, 2);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"<html></html>"));
        assert!(Dgml::parse(b"x").is_none());
    }
}
