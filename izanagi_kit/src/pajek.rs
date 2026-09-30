//! Pajek `.net` — `*Vertices N` vertex table (`id "label" x y z`), then
//! `*Edges` / `*Arcs` / `*Matrix` sections listing `i j [weight]` pairs.
//!
//! ```
//! let d = b"*Vertices 3\n1 \"a\" 0 0 0\n2 \"b\" 0 0 0\n3 \"c\" 0 0 0\n*Arcs\n1 2 1\n2 3 1\n*Edges\n1 3 2\n";
//! let p = izanagi_kit::pajek::parse(d).unwrap();
//! assert_eq!(p.vertices_declared, 3);
//! assert_eq!(p.arc_lines, 2);
//! assert_eq!(p.edge_lines, 1);
//! assert!(p.directed);
//! assert!(izanagi_kit::pajek::detect(d));
//! ```

/// A parsed Pajek `.net` census.
#[derive(Debug, Clone)]
pub struct Pajek {
    /// Count declared by the `*Vertices N` header.
    pub vertices_declared: usize,
    /// Data rows seen under `*Vertices`.
    pub vertex_lines: usize,
    /// Quoted labels in vertex rows.
    pub labels: usize,
    /// `*Edges` rows (undirected pairs).
    pub edge_lines: usize,
    /// `*Arcs` rows (directed pairs).
    pub arc_lines: usize,
    /// `*Matrix` section present.
    pub matrix: bool,
    /// Any `*Arcs` rows seen.
    pub directed: bool,
    /// Rows carrying a third weight field.
    pub weighted: usize,
    /// Self loops (`i i`).
    pub loops: usize,
    /// Distinct `*Section` headers.
    pub sections: usize,
}

fn first_int(s: &str) -> Option<usize> {
    let mut n = 0usize;
    let mut any = false;
    for c in s.trim_start().chars() {
        if c.is_ascii_digit() {
            n = n.checked_mul(10)?.checked_add(c as usize - '0' as usize)?;
            any = true;
        } else {
            break;
        }
    }
    any.then_some(n)
}

/// Detects a Pajek file: first significant line is a `*` section header such
/// as `*Vertices`, `*Network`, `*Edges`, `*Arcs`, or `*Matrix`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    for line in t.lines() {
        let l = line.trim();
        if l.is_empty() || l.starts_with('%') {
            continue;
        }
        return l.starts_with("*Vertices")
            || l.starts_with("*vertices")
            || l.starts_with("*Network")
            || l.starts_with("*Edges")
            || l.starts_with("*Arcs")
            || l.starts_with("*Matrix");
    }
    false
}

/// Parses a Pajek `.net` into a census of sections, rows, and weights.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Pajek> {
    let t = std::str::from_utf8(b).ok()?;
    if !detect(b) {
        return None;
    }
    let mut p = Pajek {
        vertices_declared: 0,
        vertex_lines: 0,
        labels: 0,
        edge_lines: 0,
        arc_lines: 0,
        matrix: false,
        directed: false,
        weighted: 0,
        loops: 0,
        sections: 0,
    };
    #[derive(Clone, Copy, PartialEq)]
    enum Sec {
        Vertices,
        Edges,
        Arcs,
        Matrix,
        Other,
    }
    let mut sec = Sec::Other;
    for line in t.lines() {
        let l = line.trim();
        if l.is_empty() || l.starts_with('%') {
            continue;
        }
        if let Some(rest) = l.strip_prefix('*') {
            p.sections += 1;
            let name = rest.split_whitespace().next().unwrap_or("");
            let low = name.to_ascii_lowercase();
            sec = match low.as_str() {
                "vertices" => {
                    if let Some(n) = rest.split_whitespace().nth(1).and_then(first_int) {
                        p.vertices_declared = n;
                    }
                    Sec::Vertices
                }
                "edges" | "edgeslist" => Sec::Edges,
                "arcs" | "arcslist" => Sec::Arcs,
                "matrix" => {
                    p.matrix = true;
                    Sec::Matrix
                }
                _ => Sec::Other,
            };
            continue;
        }
        match sec {
            Sec::Vertices => {
                if first_int(l).is_some() {
                    p.vertex_lines += 1;
                    if l.contains('"') {
                        p.labels += 1;
                    }
                }
            }
            Sec::Edges | Sec::Arcs => {
                let mut it = l.split_whitespace();
                let i = it.next().and_then(first_int);
                let j = it.next().and_then(first_int);
                if let (Some(i), Some(j)) = (i, j) {
                    if sec == Sec::Edges {
                        p.edge_lines += 1;
                    } else {
                        p.arc_lines += 1;
                        p.directed = true;
                    }
                    if i == j {
                        p.loops += 1;
                    }
                    if it.next().is_some() {
                        p.weighted += 1;
                    }
                }
            }
            Sec::Matrix | Sec::Other => {}
        }
    }
    Some(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"*Vertices 3\n1 \"a\"\n2 \"b\"\n3 \"c\"\n*Arcs\n1 2 1\n3 3\n*Edges\n1 3\n";

    #[test]
    fn detects_pajek() {
        assert!(detect(D));
        assert!(!detect(b"1 2 3"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses_census() {
        let p = parse(D).unwrap();
        assert_eq!(p.vertices_declared, 3);
        assert_eq!(p.vertex_lines, 3);
        assert_eq!(p.arc_lines, 2);
        assert_eq!(p.edge_lines, 1);
        assert_eq!(p.loops, 1);
        assert_eq!(p.weighted, 1);
        assert!(p.directed);
        assert_eq!(p.sections, 3);
    }
}
