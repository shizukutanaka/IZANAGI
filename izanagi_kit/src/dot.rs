//! Graphviz DOT directed/undirected graph description language.
//!
//! DOT files declare graphs with `digraph`/`graph` keywords, optional
//! `strict`, `subgraph`/`cluster` blocks, node/edge statements, and
//! `[attr]` attribute lists.
//!
//! ```
//! let b = concat!(
//!     "digraph G {\n",
//!     "  rankdir=LR;\n",
//!     "  a -> b [label=\"x\"];\n",
//!     "  b -> c;\n",
//!     "  subgraph cluster_0 { d; e }\n",
//!     "}\n"
//! ).as_bytes();
//! assert!(izanagi_kit::dot::detect(b));
//! let c = izanagi_kit::dot::Dot::parse(b).unwrap();
//! assert_eq!(c.edges, 2);
//! assert_eq!(c.subgraphs, 1);
//! ```

/// Parsed DOT graph summary.
#[derive(Debug, Clone)]
pub struct Dot {
    /// `digraph` (directed) vs `graph` (undirected) plus `strict` flag census.
    pub headers: usize,
    /// `strict` keyword occurrences.
    pub strict: usize,
    /// `subgraph` blocks.
    pub subgraphs: usize,
    /// `->` or `--` edge operators.
    pub edges: usize,
    /// `[` attribute list openings.
    pub attr_lists: usize,
    /// `key=value` attribute/statements (`key=` occurrences minus bracket contents).
    pub assignments: usize,
    /// Comment lines (`//`, `#`) plus `/*` block starts.
    pub comments: usize,
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

/// Whether the buffer looks like Graphviz DOT.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let has_kw = t.contains("digraph") || t.contains("graph") || t.contains("strict");
    let has_edge = t.contains("->") || t.contains("--");
    has_kw && (has_edge || (t.contains('{') && t.contains('}')))
}

impl Dot {
    /// Parses a DOT graph summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            headers: 0,
            strict: 0,
            subgraphs: 0,
            edges: 0,
            attr_lists: 0,
            assignments: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with("//") || tr.starts_with('#') || tr.starts_with("/*") {
                c.comments += 1;
                continue;
            }
            let mut rest = tr;
            if rest.starts_with("strict") {
                c.strict += 1;
                rest = rest[6..].trim_start();
            }
            if rest.starts_with("digraph") || rest.starts_with("graph") {
                c.headers += 1;
            }
            c.subgraphs += count(tr, "subgraph");
            c.edges += count(tr, "->") + count(tr, "--");
            let open = count(tr, "[");
            c.attr_lists += open;
            let eq = count(tr, "=");
            c.assignments += eq.saturating_sub(count(tr, "->") + count(tr, "--"));
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_digraph() {
        let b = concat!(
            "digraph G {\n",
            "  rankdir=LR;\n",
            "  a -> b [label=\"x\"];\n",
            "  b -> c;\n",
            "  subgraph cluster_0 { d; e }\n",
            "}\n"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Dot::parse(b).unwrap();
        assert_eq!(c.headers, 1);
        assert_eq!(c.edges, 2);
        assert_eq!(c.subgraphs, 1);
        assert_eq!(c.attr_lists, 1);
        assert_eq!(c.assignments, 1);
    }

    #[test]
    fn parses_undirected_graph() {
        let b = concat!(
            "strict graph {\n",
            "  // comment\n",
            "  a -- b;\n",
            "  b -- c;\n",
            "}\n"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Dot::parse(b).unwrap();
        assert_eq!(c.headers, 1);
        assert_eq!(c.strict, 1);
        assert_eq!(c.edges, 2);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"int main() { return 0; }"));
        assert!(Dot::parse(b"x").is_none());
    }
}
