//! Mermaid text diagram description language (`.mmd` / `.mermaid`).
//!
//! Mermaid files declare diagram types on their first statement:
//! `graph`/`flowchart` (with direction `TD`/`LR`/…), `sequenceDiagram`,
//! `classDiagram`, `stateDiagram`, `erDiagram`, `gantt`, `pie`, `journey`,
//! `mindmap`, `timeline`, etc.
//!
//! ```
//! let b = concat!(
//!     "flowchart TD\n",
//!     "  A[Start] --> B{Ok?}\n",
//!     "  B -->|yes| C[Done]\n",
//!     "  B -->|no| D[Fix]\n"
//! ).as_bytes();
//! assert!(izanagi_kit::mermaid::detect(b));
//! let c = izanagi_kit::mermaid::Mermaid::parse(b).unwrap();
//! assert_eq!(c.diagram_types, 1);
//! assert_eq!(c.edges, 3);
//! ```

/// Parsed Mermaid diagram summary.
#[derive(Debug, Clone)]
pub struct Mermaid {
    /// Diagram-type statements recognized (`graph`, `flowchart`, `sequenceDiagram`, …).
    pub diagram_types: usize,
    /// Edge operators (`-->`, `---`, `->>`, `--x`, `-.->`, `==>`, `--|`).
    pub edges: usize,
    /// Subgraph / end blocks (`subgraph` keyword).
    pub subgraphs: usize,
    /// `class`, `participant`, `actor`, `note`, `state` declarations.
    pub declarations: usize,
    /// `%%` comment lines.
    pub comments: usize,
    /// Directive `%%{` blocks.
    pub directives: usize,
}

const DIAGRAM_TYPES: &[&str] = &[
    "graph",
    "flowchart",
    "sequencediagram",
    "classdiagram",
    "statediagram",
    "statediagram-v2",
    "erdiagram",
    "gantt",
    "pie",
    "journey",
    "mindmap",
    "timeline",
    "gitgraph",
    "c4context",
    "sankey-beta",
    "xychart-beta",
    "block-beta",
    "packet-beta",
    "kanban",
    "requirementdiagram",
    "zenuml",
];

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

/// Whether the buffer looks like a Mermaid diagram.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.lines().any(|l| {
        let tr = l.trim().to_ascii_lowercase();
        DIAGRAM_TYPES.iter().any(|dt| tr.starts_with(dt))
    })
}

impl Mermaid {
    /// Parses a Mermaid diagram summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            diagram_types: 0,
            edges: 0,
            subgraphs: 0,
            declarations: 0,
            comments: 0,
            directives: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with("%%{") {
                c.directives += 1;
                continue;
            }
            if tr.starts_with("%%") {
                c.comments += 1;
                continue;
            }
            let low = tr.to_ascii_lowercase();
            if DIAGRAM_TYPES.iter().any(|dt| low.starts_with(dt)) {
                c.diagram_types += 1;
            }
            // `->` is a substring of `->>` and `-.->`/`-->` contain `-` runs;
            // count every operator then subtract the inner overlaps.
            let mut edges = 0usize;
            for pat in ["-->", "---", "->>", "--x", "-.->", "==>", "--o", "->"] {
                edges += count(tr, pat);
            }
            c.edges +=
                edges.saturating_sub(count(tr, "->>") + count(tr, "-->") + count(tr, "-.->"));
            c.subgraphs += count(&low, "subgraph");
            for kw in [
                "class ",
                "participant ",
                "actor ",
                "note ",
                "state ",
                "activate ",
                "deactivate ",
            ] {
                if low.starts_with(kw) {
                    c.declarations += 1;
                }
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_flowchart() {
        let b = concat!(
            "flowchart TD\n",
            "  A[Start] --> B{Ok?}\n",
            "  B -->|yes| C[Done]\n",
            "  B -->|no| D[Fix]\n"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Mermaid::parse(b).unwrap();
        assert_eq!(c.diagram_types, 1);
        assert_eq!(c.edges, 3);
    }

    #[test]
    fn parses_sequence() {
        let b = concat!(
            "sequenceDiagram\n",
            "  %% a comment\n",
            "  participant A\n",
            "  actor B\n",
            "  A->>B: hello\n"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Mermaid::parse(b).unwrap();
        assert_eq!(c.diagram_types, 1);
        assert_eq!(c.edges, 1);
        assert_eq!(c.declarations, 2);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"fn main() {}"));
        assert!(Mermaid::parse(b"x").is_none());
    }
}
