//! drawio / diagrams.net XML file format (`.drawio` / `.dio`).
//!
//! drawio files are XML documents rooted at `<mxfile` with `<diagram>`
//! pages containing either a compressed base64 payload or an uncompressed
//! `<mxGraphModel` tree of `<root>` → `<mxCell` vertices/edges.
//!
//! ```
//! let b = concat!(
//!     "<mxfile host=\"app.diagrams.net\">\n",
//!     "  <diagram name=\"p1\" id=\"d1\">\n",
//!     "    <mxGraphModel>\n",
//!     "      <root><mxCell id=\"0\"/><mxCell id=\"1\" parent=\"0\"/>\n",
//!     "      <mxCell id=\"2\" vertex=\"1\" parent=\"1\"/>\n",
//!     "      <mxCell id=\"3\" edge=\"1\" source=\"2\" target=\"1\"/></root>\n",
//!     "    </mxGraphModel>\n",
//!     "  </diagram>\n",
//!     "</mxfile>\n"
//! ).as_bytes();
//! assert!(izanagi_kit::drawio::detect(b));
//! let c = izanagi_kit::drawio::Drawio::parse(b).unwrap();
//! assert_eq!(c.diagrams, 1);
//! assert_eq!(c.cells, 4);
//! ```

/// Parsed drawio file summary.
#[derive(Debug, Clone)]
pub struct Drawio {
    /// `<diagram` page count.
    pub diagrams: usize,
    /// `<mxCell` elements.
    pub cells: usize,
    /// Cells with `vertex="1"` / `edge="1"`.
    pub vertices: usize,
    /// Cells with `edge="1"`.
    pub edges: usize,
    /// `<mxGraphModel` occurrences.
    pub graph_models: usize,
    /// Compressed diagrams (no `<mxGraphModel` inside — base64 payload).
    pub compressed: usize,
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

/// Whether the buffer looks like a drawio file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("<mxfile") || (t.contains("<mxGraphModel") && t.contains("<mxCell"))
}

impl Drawio {
    /// Parses a drawio file summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            diagrams: 0,
            cells: 0,
            vertices: 0,
            edges: 0,
            graph_models: 0,
            compressed: 0,
        };
        c.diagrams = count(t, "<diagram");
        c.graph_models = count(t, "<mxGraphModel");
        c.cells = count(t, "<mxCell");
        c.vertices = count(t, "vertex=\"1\"");
        c.edges = count(t, "edge=\"1\"");
        c.compressed = c.diagrams.saturating_sub(c.graph_models);
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_uncompressed() {
        let b = concat!(
            "<mxfile host=\"app.diagrams.net\">\n",
            "  <diagram name=\"p1\" id=\"d1\">\n",
            "    <mxGraphModel>\n",
            "      <root><mxCell id=\"0\"/><mxCell id=\"1\" parent=\"0\"/>\n",
            "      <mxCell id=\"2\" vertex=\"1\" parent=\"1\"/>\n",
            "      <mxCell id=\"3\" edge=\"1\" source=\"2\" target=\"1\"/></root>\n",
            "    </mxGraphModel>\n",
            "  </diagram>\n",
            "</mxfile>\n"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Drawio::parse(b).unwrap();
        assert_eq!(c.diagrams, 1);
        assert_eq!(c.cells, 4);
        assert_eq!(c.vertices, 1);
        assert_eq!(c.edges, 1);
        assert_eq!(c.graph_models, 1);
        assert_eq!(c.compressed, 0);
    }

    #[test]
    fn parses_compressed_flag() {
        let b = br#"<mxfile><diagram id="d1">jVNLj9MwEP0rtpl4c3Np2druFq1WEGgV</diagram></mxfile>"#;
        assert!(detect(b));
        let c = Drawio::parse(b).unwrap();
        assert_eq!(c.diagrams, 1);
        assert_eq!(c.compressed, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"<html></html>"));
        assert!(Drawio::parse(b"x").is_none());
    }
}
